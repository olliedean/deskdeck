(() => {
    const PROTOCOL_VERSION = 1;
    const reconnectDelay = 1800;
    const grid = document.querySelector("#buttons");
    const pageTabs = document.querySelector("#page-tabs");
    const toast = document.querySelector("#toast");
    const toolbar = document.querySelector("#editor-toolbar");
    const workspace = document.querySelector(".workspace");
    const deckFrame = document.querySelector(".deck-frame");
    const editorHint = document.querySelector(".editor-hint");
    const gridSettingsButton = document.querySelector("#grid-settings");
    const editModeButton = document.querySelector("#edit-mode");
    const addButton = document.querySelector("#add-button");
    const saveButton = document.querySelector("#save-changes");
    const discardButton = document.querySelector("#discard-changes");
    const dialog = document.querySelector("#edit-dialog");
    const gridDialog = document.querySelector("#grid-dialog");
    const gridForm = document.querySelector("#grid-form");
    const buttonForm = document.querySelector("#button-form");
    const actionList = document.querySelector("#action-list");
    const addActionButton = document.querySelector("#add-action");
    let deck = null;
    let draftDeck = null;
    let actionDefinitions = new Map();
    let actionsReady = false;
    let currentPage = 0;
    let editingButtonIndex = null;
    let newButtonPosition = null;
    let socket = null;
    let reconnectTimer = null;
    let toastTimer = null;
    const isDesktop = window.matchMedia("(min-width: 800px) and (pointer: fine)");
    const isLocalHost = ["localhost", "127.0.0.1", "[::1]"].includes(location.hostname);

    function activeDeck() {
        return draftDeck !== null ? draftDeck : deck;
    }

    function replaceChildren(element, ...children) {
        while (element.firstChild) element.removeChild(element.firstChild);
        children.forEach(child => element.appendChild(child));
    }

    function cloneData(value) {
        return JSON.parse(JSON.stringify(value));
    }

    function openDialog(element) {
        if (typeof element.showModal === "function") {
            element.showModal();
            return;
        }
        document.documentElement.classList.add("dialog-fallback");
        document.body.classList.add("dialog-open");
        element.setAttribute("open", "");
        element.setAttribute("aria-modal", "true");
    }

    function closeDialog(element) {
        if (typeof element.close === "function") {
            element.close();
            return;
        }
        element.removeAttribute("open");
        element.removeAttribute("aria-modal");
        if (!document.querySelector(".edit-dialog[open]")) {
            document.body.classList.remove("dialog-open");
        }
        element.dispatchEvent(new Event("close"));
    }

    function showToast(message, kind = "error") {
        toast.textContent = message;
        toast.dataset.kind = kind;
        toast.hidden = false;
        clearTimeout(toastTimer);
        toastTimer = setTimeout(() => { toast.hidden = true; }, 5000);
    }

    function showError(message) {
        showToast(message);
    }

    function setIcon(element, icon) {
        const classes = icon ? icon.trim().split(/\s+/) : [];
        if (classes.length === 0 || !classes.every(className => /^fa-[a-z0-9-]+$/.test(className))) {
            element.hidden = true;
            return;
        }
        if (!classes.some(className => ["fa-solid", "fa-regular", "fa-brands"].includes(className))) {
            classes.unshift("fa-solid");
        }
        element.classList.add(...classes);
    }

    function isAppearanceJson(text) {
        if (!text || typeof text !== 'string' || !text.trim()) return false;
        try {
            const parsed = JSON.parse(text);
            const appearanceFields = [
                'font_size', 'horizontal_alignment', 'vertical_alignment',
                'padding', 'corner_radius', 'border', 'opacity',
                'badge_text', 'badge_color'
            ];
            const nullCount = appearanceFields.filter(field =>
                parsed.hasOwnProperty(field) && parsed[field] === null
            ).length;
            return nullCount >= 3;
        } catch (e) {
            return false;
        }
    }

    function safeHttpUrl(value) {
        if (!value || !value.trim()) return null;
        try {
            const url = new URL(value.trim(), location.href);
            return ["http:", "https:"].includes(url.protocol) ? url.href : null;
        } catch (error) {
            return null;
        }
    }

    function appendSafeHtml(parent, source) {
        const template = document.createElement("template");
        template.innerHTML = source;
        const allowedTags = new Set(["DIV", "SPAN", "P", "STRONG", "B", "EM", "I", "SMALL", "BR", "IMG"]);
        const blockedTags = new Set(["SCRIPT", "STYLE", "IFRAME", "OBJECT", "EMBED", "SVG", "MATH", "FORM", "INPUT", "BUTTON"]);

        const copyNode = node => {
            if (node.nodeType === Node.TEXT_NODE) return document.createTextNode(node.textContent || "");
            if (node.nodeType !== Node.ELEMENT_NODE) return null;
            if (blockedTags.has(node.tagName)) return null;
            if (!allowedTags.has(node.tagName)) {
                const fragment = document.createDocumentFragment();
                for (const child of node.childNodes) {
                    const safeChild = copyNode(child);
                    if (safeChild) fragment.append(safeChild);
                }
                return fragment;
            }

            const safeElement = document.createElement(node.tagName.toLowerCase());
            for (const attribute of ["class", "title", "alt"]) {
                const value = node.getAttribute(attribute);
                if (value && (attribute !== "class" || /^[\w -]{1,120}$/.test(value))) {
                    safeElement.setAttribute(attribute, value);
                }
            }
            if (node.tagName === "IMG") {
                const src = safeHttpUrl(node.getAttribute("src"));
                if (!src) return null;
                safeElement.src = src;
                safeElement.loading = "lazy";
                safeElement.referrerPolicy = "no-referrer";
            } else {
                for (const child of node.childNodes) {
                    const safeChild = copyNode(child);
                    if (safeChild) safeElement.append(safeChild);
                }
            }
            return safeElement;
        };

        const fragment = document.createDocumentFragment();
        for (const child of template.content.childNodes) {
            const safeChild = copyNode(child);
            if (safeChild) fragment.append(safeChild);
        }
        parent.append(fragment);
    }

    function setButtonAppearance(element, button) {
        const image = safeHttpUrl(button.background_image);
        if (image) {
            element.style.backgroundImage = `linear-gradient(rgba(0, 0, 0, .18), rgba(0, 0, 0, .35)), url("${image.replace(/"/g, "%22")}")`;
            element.style.backgroundPosition = "center";
            element.style.backgroundSize = "cover";
        }
        const css = button.custom_css ? button.custom_css.trim() : "";
        if (css && !/(?:url\s*\(|@import|expression\s*\(|javascript\s*:)/i.test(css)) {
            element.style.cssText += `;${css}`;
        }
    }

    function send(message) {
        if (!socket || socket.readyState !== WebSocket.OPEN) {
            showError("The deck is offline. Reconnecting…");
            return false;
        }
        socket.send(JSON.stringify(message));
        return true;
    }

    function press(button, down) {
        send({ Press: { button_id: button.id, down } });
        const element = grid.querySelector(`[data-button-id="${CSS.escape(button.id)}"]`);
        if (element) element.classList.toggle("is-active", down);
    }

    function renderPages() {
        const currentDeck = activeDeck();
        const pages = currentDeck ? currentDeck.pages : [];
        replaceChildren(pageTabs);
        pageTabs.hidden = pages.length < 2;
        pages.forEach((page, index) => {
            const tab = document.createElement("button");
            tab.type = "button";
            tab.className = "page-tab";
            tab.textContent = page.name;
            tab.setAttribute("aria-selected", String(index === currentPage));
            tab.addEventListener("click", () => {
                currentPage = index;
                renderDeck();
            });
            pageTabs.append(tab);
        });
    }

    function renderDeck() {
        const shownDeck = activeDeck();
        if (!shownDeck) return;
        const page = shownDeck.pages
            ? (shownDeck.pages[currentPage] || shownDeck.pages[0])
            : null;
        const cols = Math.max(1, Number(shownDeck.grid && shownDeck.grid.cols) || 5);
        const rows = Math.max(1, Number(shownDeck.grid && shownDeck.grid.rows) || 3);
        toolbar.hidden = !isDesktop.matches;
        gridSettingsButton.hidden = !isDesktop.matches || !isLocalHost;
        editorHint.textContent = isLocalHost ? "Deck setup" : "Open http://127.0.0.1:8080 on the host to edit";
        editModeButton.hidden = Boolean(draftDeck) || !isLocalHost;
        editModeButton.disabled = !actionsReady;
        addButton.hidden = !draftDeck;
        addButton.disabled = !actionsReady
            || Boolean(draftDeck && page && page.buttons.length >= Math.min(cols * rows, 80));
        saveButton.hidden = !draftDeck;
        discardButton.hidden = !draftDeck;
        if (!page) {
            replaceChildren(grid);
            renderPages();
            return;
        }
        currentPage = shownDeck.pages.indexOf(page);
        renderPages();
        const capacity = Math.min(cols * rows, 80);
        const positionedButtons = new Map();
        page.buttons.forEach((button, index) => {
            positionedButtons.set(Number.isInteger(button.position) ? button.position : index, { button, index });
        });
        grid.style.gridTemplateColumns = `repeat(${cols}, minmax(0, 1fr))`;
        grid.style.gridTemplateRows = `repeat(${rows}, minmax(0, 1fr))`;
        replaceChildren(grid);

        for (let position = 0; position < capacity; position += 1) {
            const placed = positionedButtons.get(position);
            if (!placed) {
                const slot = draftDeck ? document.createElement("button") : document.createElement("div");
                slot.className = draftDeck ? "empty-slot add-slot" : "empty-slot";
                if (draftDeck) {
                    slot.type = "button";
                    slot.textContent = "+";
                    slot.setAttribute("aria-label", `Add button at position ${position + 1}`);
                    slot.addEventListener("click", () => addDraftButton(position));
                } else {
                    slot.setAttribute("aria-hidden", "true");
                }
                grid.append(slot);
                continue;
            }

            const { button, index } = placed;
            const element = document.createElement(draftDeck ? "div" : "button");
            element.type = "button";
            element.className = "deck-button";
            element.dataset.buttonId = button.id;
            element.setAttribute("aria-label", button.label);
            setButtonAppearance(element, button);
            if (draftDeck) element.setAttribute("role", "group");
            if (button.custom_html && button.custom_html.trim() && !isAppearanceJson(button.custom_html)) {
                const content = document.createElement("span");
                content.className = "button-custom-content";
                appendSafeHtml(content, button.custom_html);
                element.append(content);
            } else {
                const icon = document.createElement("span");
                icon.className = "button-icon";
                icon.setAttribute("aria-hidden", "true");
                setIcon(icon, button.icon);
                const label = document.createElement("span");
                label.className = "button-label";
                label.textContent = button.label;
                element.append(icon, label);
            }
            if (draftDeck) {
                const tools = document.createElement("span");
                tools.className = "button-tools";
                const edit = document.createElement("button");
                edit.type = "button";
                edit.className = "button-tool control";
                edit.textContent = "Edit";
                edit.setAttribute("aria-label", `Edit ${button.label}`);
                const remove = document.createElement("button");
                remove.type = "button";
                remove.className = "button-tool control delete";
                remove.textContent = "×";
                remove.setAttribute("aria-label", `Delete ${button.label}`);
                edit.addEventListener("click", event => {
                    event.stopPropagation();
                    openButtonDialog(index);
                });
                remove.addEventListener("click", event => {
                    event.stopPropagation();
                    if (window.confirm(`Delete "${button.label}"?`)) {
                        page.buttons.forEach((candidate, candidateIndex) => {
                            if (!Number.isInteger(candidate.position)) candidate.position = candidateIndex;
                        });
                        page.buttons.splice(index, 1);
                        renderDeck();
                    }
                });
                tools.append(edit, remove);
                element.append(tools);
            }
            const release = event => {
                if (element.classList.contains("is-active")) {
                    if (event) event.preventDefault();
                    press(button, false);
                }
            };
            if (window.PointerEvent) {
                element.addEventListener("pointerdown", event => {
                    if (draftDeck) return;
                    if (event.button !== undefined && event.button !== 0) return;
                    event.preventDefault();
                    if (typeof element.setPointerCapture === "function") {
                        element.setPointerCapture(event.pointerId);
                    }
                    press(button, true);
                });
                element.addEventListener("pointerup", release);
                element.addEventListener("pointercancel", release);
                element.addEventListener("lostpointercapture", release);
            } else {
                element.addEventListener("touchstart", event => {
                    if (draftDeck) return;
                    event.preventDefault();
                    press(button, true);
                }, { passive: false });
                element.addEventListener("touchend", release, { passive: false });
                element.addEventListener("touchcancel", release, { passive: false });
                element.addEventListener("mousedown", event => {
                    if (draftDeck || event.button !== 0) return;
                    event.preventDefault();
                    press(button, true);
                });
                element.addEventListener("mouseup", release);
                element.addEventListener("mouseleave", release);
            }
            element.addEventListener("keydown", event => {
                if (!draftDeck && (event.key === " " || event.key === "Enter") && !event.repeat) press(button, true);
            });
            element.addEventListener("keyup", event => {
                if (!draftDeck && (event.key === " " || event.key === "Enter")) press(button, false);
            });
            grid.append(element);
        }
        fitGrid(cols, rows);
    }

    function fitGrid(cols, rows) {
        const frameStyle = getComputedStyle(deckFrame);
        const workspaceStyle = getComputedStyle(workspace);
        const frameHorizontalInsets = ["paddingLeft", "paddingRight", "borderLeftWidth", "borderRightWidth"]
            .reduce((total, key) => total + Number.parseFloat(frameStyle[key] || "0"), 0);
        const frameVerticalInsets = ["paddingTop", "paddingBottom", "borderTopWidth", "borderBottomWidth"]
            .reduce((total, key) => total + Number.parseFloat(frameStyle[key] || "0"), 0);
        const workspacePadding = ["paddingTop", "paddingBottom"]
            .reduce((total, key) => total + Number.parseFloat(workspaceStyle[key] || "0"), 0);
        const toolbarHeight = toolbar.hidden ? 0 : toolbar.getBoundingClientRect().height
            + Number.parseFloat(getComputedStyle(toolbar).marginBottom || "0");
        const tabsHeight = pageTabs.hidden ? 0 : pageTabs.getBoundingClientRect().height
            + Number.parseFloat(getComputedStyle(pageTabs).marginBottom || "0");
        const gridStyle = getComputedStyle(grid);
        const gapX = Number.parseFloat(gridStyle.columnGap) || 0;
        const gapY = Number.parseFloat(gridStyle.rowGap) || 0;
        const availableWidth = workspace.clientWidth - frameHorizontalInsets;
        const availableHeight = workspace.clientHeight - workspacePadding - toolbarHeight - tabsHeight - frameVerticalInsets;
        const widthPerCell = (availableWidth - gapX * (cols - 1)) / cols;
        const heightPerCell = (availableHeight - gapY * (rows - 1)) / rows;
        const size = Math.max(1, Math.floor(Math.min(widthPerCell, heightPerCell)));
        grid.style.setProperty("--button-size", `${size}px`);
        grid.style.gridTemplateColumns = `repeat(${cols}, ${size}px)`;
        grid.style.gridTemplateRows = `repeat(${rows}, ${size}px)`;
    }

    function handleHostMessage(message) {
        if (message.Welcome) {
            deck = message.Welcome.deck;
            if (!draftDeck) currentPage = Math.min(currentPage, Math.max(0, deck.pages.length - 1));
            renderDeck();
            return;
        }
        if (message.DeckUpdated) {
            deck = message.DeckUpdated;
            if (!draftDeck) currentPage = Math.min(currentPage, Math.max(0, deck.pages.length - 1));
            renderDeck();
            return;
        }
        if (message.StatePatch && message.StatePatch.ButtonActive) {
            const patch = message.StatePatch.ButtonActive;
            const button = grid.querySelector(`[data-button-id="${CSS.escape(patch.button_id)}"]`);
            if (button) button.classList.toggle("is-active", patch.active);
            return;
        }
        if (message.Error) showError(`${message.Error.code}: ${message.Error.message}`);
    }

    async function loadDeck() {
        try {
            const response = await fetch("/deck", { cache: "no-store" });
            if (!response.ok) throw new Error(`Deck request failed (${response.status})`);
            deck = await response.json();
            renderDeck();
        } catch (error) {
            showError(`Could not load the deck: ${error.message}`);
        }
    }

    async function loadActionDefinitions() {
        try {
            const response = await fetch("/actions", { cache: "no-store" });
            if (!response.ok) throw new Error(`Action catalog request failed (${response.status})`);
            const definitions = await response.json();
            if (!Array.isArray(definitions)) throw new Error("Action catalog response is invalid");
            actionDefinitions = new Map(definitions.map(definition => [definition.kind, definition]));
            actionsReady = true;
            renderDeck();
        } catch (error) {
            showError(`Could not load action definitions: ${error.message}`);
        }
    }

    function makeActionEditor(action = { action: "hotkey", config: { keys: "ctrl, d" } }) {
        const actionKind = action.action || action.kind || "hotkey";
        let config = cloneData(action.config || {});
        const row = document.createElement("fieldset");
        row.className = "action-row";
        const heading = document.createElement("div");
        heading.className = "action-row-heading";
        const kindLabel = document.createElement("label");
        kindLabel.className = "field compact";
        const kindCaption = document.createElement("span");
        kindCaption.textContent = "Action";
        const kind = document.createElement("select");
        kind.className = "action-kind";
        for (const definition of actionDefinitions.values()) {
            const option = document.createElement("option");
            option.value = definition.kind;
            option.textContent = definition.name;
            kind.append(option);
        }
        if (!actionDefinitions.has(actionKind)) {
            const option = document.createElement("option");
            option.value = actionKind;
            option.textContent = `Unavailable: ${actionKind}`;
            kind.append(option);
            row.dataset.originalConfig = JSON.stringify(config);
        }
        kind.value = actionKind;
        kindLabel.append(kindCaption, kind);
        const remove = document.createElement("button");
        remove.type = "button";
        remove.className = "text-button remove-action";
        remove.textContent = "Remove";
        remove.addEventListener("click", () => row.remove());
        heading.append(kindLabel, remove);
        const valueFields = document.createElement("div");
        valueFields.className = "action-values";
        row.append(heading, valueFields);

        const renderFields = () => {
            replaceChildren(valueFields);
            const definition = actionDefinitions.get(kind.value);
            if (!definition) {
                const notice = document.createElement("small");
                notice.className = "action-description";
                notice.textContent = "This action plugin is not currently available.";
                valueFields.append(notice);
                return;
            }
            const description = document.createElement("p");
            description.className = "action-description";
            description.textContent = definition.description;
            valueFields.append(description);
            for (const field of definition.fields) {
                const wrapper = document.createElement("label");
                wrapper.className = "field compact";
                const caption = document.createElement("span");
                caption.textContent = field.label;
                let input;
                if (field.type === "multiline") {
                    input = document.createElement("textarea");
                    input.rows = 3;
                } else if (field.type === "select") {
                    input = document.createElement("select");
                    for (const optionValue of field.options || []) {
                        const option = document.createElement("option");
                        option.value = optionValue;
                        option.textContent = optionValue;
                        input.append(option);
                    }
                } else {
                    input = document.createElement("input");
                    input.type = field.type === "number" ? "number" : "text";
                }
                input.dataset.configKey = field.key;
                input.required = field.required;
                input.placeholder = field.placeholder || "";
                if (field.min !== undefined) input.min = field.min;
                if (field.max !== undefined) input.max = field.max;
                if (field.step !== undefined) input.step = field.step;
                let value = config[field.key];
                if (value === undefined || value === null) value = field.default;
                if (value === undefined || value === null) {
                    value = field.type === "select" && field.options && field.options.length
                        ? field.options[0]
                        : "";
                }
                input.value = value;
                wrapper.append(caption, input);
                if (field.help) {
                    const help = document.createElement("small");
                    help.textContent = field.help;
                    wrapper.append(help);
                }
                valueFields.append(wrapper);
            }
        };

        kind.addEventListener("change", () => {
            const definition = actionDefinitions.get(kind.value);
            config = Object.fromEntries(
                (definition ? definition.fields : [])
                    .filter(field => field.default !== undefined)
                    .map(field => [field.key, field.default]),
            );
            delete row.dataset.originalConfig;
            renderFields();
        });
        renderFields();
        return row;
    }

        function openButtonDialog(index = null) {
            if (!actionsReady) {
                showError("Action definitions are not available yet.");
                return;
            }
            const page = activeDeck().pages[currentPage];
            editingButtonIndex = index;
            const button = index === null
                ? {
                    id: `button-${Date.now().toString(36)}`,
                    label: "New button",
                    icon: null,
                    position: newButtonPosition,
                    on_press: [{ action: "hotkey", config: { keys: "ctrl, d" } }],
                }
                : page.buttons[index];
            buttonForm.elements.namedItem("id").value = button.id;
            buttonForm.elements.namedItem("label").value = button.label;
            buttonForm.elements.namedItem("icon").value = button.icon || "";
            buttonForm.elements.namedItem("background_image").value = button.background_image || "";
            buttonForm.elements.namedItem("custom_css").value = button.custom_css || "";
            buttonForm.elements.namedItem("custom_html").value = button.custom_html || "";
            document.querySelector("#dialog-title").textContent = index === null ? "Add button" : "Edit button";
            replaceChildren(actionList, ...button.on_press.map(makeActionEditor));
            openDialog(dialog);
        }

        function addDraftButton(position = null) {
            if (!actionsReady) return;
            const page = activeDeck().pages[currentPage];
            const capacity = Math.min(activeDeck().grid.cols * activeDeck().grid.rows, 80);
            const occupied = new Set(page.buttons.map((button, index) =>
                Number.isInteger(button.position) ? button.position : index,
            ));
            const target = position !== null && position !== undefined
                ? position
                : Array.from({ length: capacity }, (_, index) => index).find(index => !occupied.has(index));
            if (target === undefined || target >= capacity || occupied.has(target)) return;
            newButtonPosition = target;
            openButtonDialog();
        }

    function readAction(row) {
        const kind = row.querySelector(".action-kind").value;
        const definition = actionDefinitions.get(kind);
        if (!definition) {
            return {
                action: kind,
                config: JSON.parse(row.dataset.originalConfig || "{}"),
            };
        }
        const config = {};
        for (const field of definition.fields) {
            const input = row.querySelector(`[data-config-key="${CSS.escape(field.key)}"]`);
            if (!input || (input.value === "" && !field.required)) continue;
            config[field.key] = field.type === "number" ? Number(input.value) : input.value;
        }
        return { action: kind, config };
    }

    async function saveDraft() {
            saveButton.disabled = true;
            saveButton.textContent = "Saving…";
            try {
                const response = await fetch("/deck", {
                    method: "PUT",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify(draftDeck),
                });
                if (!response.ok) throw new Error(await response.text());
                deck = await response.json();
                draftDeck = null;
                currentPage = Math.min(currentPage, deck.pages.length - 1);
                renderDeck();
                showToast("Deck saved.", "success");
            } catch (error) {
                showError(`Could not save the deck: ${error.message}`);
            } finally {
                saveButton.disabled = false;
                saveButton.textContent = "Save changes";
            }
        }

    editModeButton.addEventListener("click", () => {
        if (!deck) return;
        draftDeck = cloneData(deck);
        renderDeck();
    });
    gridSettingsButton.addEventListener("click", () => {
        if (!deck) return;
        if (!draftDeck) draftDeck = cloneData(deck);
        gridForm.elements.namedItem("cols").value = draftDeck.grid.cols;
        gridForm.elements.namedItem("rows").value = draftDeck.grid.rows;
        renderDeck();
        openDialog(gridDialog);
    });
    document.querySelector("#close-grid-dialog").addEventListener("click", () => closeDialog(gridDialog));
    document.querySelector("#cancel-grid-dialog").addEventListener("click", () => closeDialog(gridDialog));
    gridForm.addEventListener("submit", event => {
        event.preventDefault();
        if (!gridForm.reportValidity()) return;
        const cols = Number(gridForm.elements.namedItem("cols").value);
        const rows = Number(gridForm.elements.namedItem("rows").value);
        if (cols * rows > 80) {
            showError("The grid can contain no more than 80 buttons.");
            return;
        }
        const fullestPage = Math.max(...draftDeck.pages.map(page => page.buttons.length));
        if (cols * rows < fullestPage) {
            showError(`The grid must fit the ${fullestPage} buttons on the fullest page.`);
            return;
        }
        draftDeck.grid = { cols, rows };
        closeDialog(gridDialog);
        renderDeck();
    });
    addButton.addEventListener("click", () => addDraftButton());
    discardButton.addEventListener("click", () => {
        draftDeck = null;
        renderDeck();
    });
    saveButton.addEventListener("click", saveDraft);
    addActionButton.addEventListener("click", () => {
        if (!actionsReady) return;
        actionList.append(makeActionEditor());
    });
    document.querySelector("#close-dialog").addEventListener("click", () => closeDialog(dialog));
    document.querySelector("#cancel-dialog").addEventListener("click", () => closeDialog(dialog));
    document.addEventListener("keydown", event => {
        if (event.key === "Escape" && document.body.classList.contains("dialog-open")) {
            const open = document.querySelector(".edit-dialog[open]");
            if (open) closeDialog(open);
        }
    });
    buttonForm.addEventListener("submit", event => {
        event.preventDefault();
        if (!buttonForm.reportValidity()) return;
        const page = activeDeck().pages[currentPage];
        const id = buttonForm.elements.namedItem("id").value.trim();
        const duplicate = activeDeck().pages.some(candidate =>
            candidate.buttons.some((button, index) =>
                button.id === id && !(candidate === page && index === editingButtonIndex),
            ),
        );
        if (duplicate) {
            showError(`Button ID "${id}" is already in use.`);
            return;
        }
        const onPress = [...actionList.querySelectorAll(".action-row")].map(readAction);
        const button = {
            id,
            label: buttonForm.elements.namedItem("label").value.trim(),
            icon: buttonForm.elements.namedItem("icon").value.trim() || null,
            position: editingButtonIndex === null
                ? newButtonPosition
                : (page.buttons[editingButtonIndex].position !== null
                    && page.buttons[editingButtonIndex].position !== undefined
                    ? page.buttons[editingButtonIndex].position
                    : editingButtonIndex),
            background_image: buttonForm.elements.namedItem("background_image").value.trim() || null,
            custom_css: buttonForm.elements.namedItem("custom_css").value.trim() || null,
            custom_html: buttonForm.elements.namedItem("custom_html").value || null,
            on_press: onPress,
        };
        if (editingButtonIndex === null) page.buttons.push(button);
        else page.buttons[editingButtonIndex] = button;
        closeDialog(dialog);
        renderDeck();
    });
    dialog.addEventListener("close", () => {
        editingButtonIndex = null;
        newButtonPosition = null;
        buttonForm.reset();
    });
    if (typeof isDesktop.addEventListener === "function") {
        isDesktop.addEventListener("change", renderDeck);
    } else if (typeof isDesktop.addListener === "function") {
        isDesktop.addListener(renderDeck);
    }
    window.addEventListener("resize", () => {
        const active = activeDeck();
        if (active) fitGrid(active.grid.cols, active.grid.rows);
    });

    function connect() {
        clearTimeout(reconnectTimer);
        const scheme = location.protocol === "https:" ? "wss:" : "ws:";
        socket = new WebSocket(`${scheme}//${location.host}/ws`);
        socket.addEventListener("open", () => {
            send({
                Hello: {
                    protocol_version: PROTOCOL_VERSION,
                    token: "",
                    screen_width: window.screen.width,
                    screen_height: window.screen.height,
                },
            });
        });
        socket.addEventListener("message", event => {
            try {
                handleHostMessage(JSON.parse(event.data));
            } catch (error) {
                showError(`Received an invalid host message: ${error.message}`);
            }
        });
        socket.addEventListener("close", () => {
            reconnectTimer = setTimeout(connect, reconnectDelay);
        });
    }

    loadDeck();
    loadActionDefinitions();
    connect();
    if ("serviceWorker" in navigator) {
        window.addEventListener("load", () => {
            navigator.serviceWorker.register("/service-worker.js").catch(error => {
                console.error("DeskDeck service worker registration failed:", error);
            });
        });
    }
})();
