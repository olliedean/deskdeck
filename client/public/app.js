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
    let deck = null;
    let draftDeck = null;
    let currentPage = 0;
    let editingButtonIndex = null;
    let socket = null;
    let reconnectTimer = null;
    let toastTimer = null;
    const isDesktop = window.matchMedia("(min-width: 800px) and (pointer: fine)");
    const isLocalHost = ["localhost", "127.0.0.1", "[::1]"].includes(location.hostname);

    function activeDeck() {
        return draftDeck ?? deck;
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
        const classes = icon?.trim().split(/\s+/) ?? [];
        if (classes.length === 0 || !classes.every(className => /^fa-[a-z0-9-]+$/.test(className))) {
            element.hidden = true;
            return;
        }
        if (!classes.some(className => ["fa-solid", "fa-regular", "fa-brands"].includes(className))) {
            classes.unshift("fa-solid");
        }
        element.classList.add(...classes);
    }

    function safeHttpUrl(value) {
        if (!value?.trim()) return null;
        try {
            const url = new URL(value.trim(), location.href);
            return ["http:", "https:"].includes(url.protocol) ? url.href : null;
        } catch {
            return null;
        }
    }

    function appendSafeHtml(parent, source) {
        const template = document.createElement("template");
        template.innerHTML = source;
        const allowedTags = new Set(["DIV", "SPAN", "P", "STRONG", "B", "EM", "I", "SMALL", "BR", "IMG"]);
        const blockedTags = new Set(["SCRIPT", "STYLE", "IFRAME", "OBJECT", "EMBED", "SVG", "MATH", "FORM", "INPUT", "BUTTON"]);

        const copyNode = node => {
            if (node.nodeType === Node.TEXT_NODE) return document.createTextNode(node.textContent ?? "");
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
        const css = button.custom_css?.trim();
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
        const pages = activeDeck()?.pages ?? [];
        pageTabs.replaceChildren();
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
        const page = shownDeck.pages?.[currentPage] ?? shownDeck.pages?.[0];
        const cols = Math.max(1, Number(shownDeck.grid?.cols) || 5);
        const rows = Math.max(1, Number(shownDeck.grid?.rows) || 3);
        toolbar.hidden = !isDesktop.matches;
        gridSettingsButton.hidden = !isDesktop.matches || !isLocalHost;
        editorHint.textContent = isLocalHost ? "Deck setup" : "Open http://127.0.0.1:8080 on the host to edit";
        editModeButton.hidden = Boolean(draftDeck) || !isLocalHost;
        addButton.hidden = !draftDeck;
        addButton.disabled = Boolean(draftDeck && page && page.buttons.length >= Math.min(cols * rows, 80));
        saveButton.hidden = !draftDeck;
        discardButton.hidden = !draftDeck;
        if (!page) {
            grid.replaceChildren();
            renderPages();
            return;
        }
        currentPage = shownDeck.pages.indexOf(page);
        renderPages();
        const capacity = Math.min(cols * rows, 80);
        grid.style.gridTemplateColumns = `repeat(${cols}, minmax(0, 1fr))`;
        grid.style.gridTemplateRows = `repeat(${rows}, minmax(0, 1fr))`;
        grid.replaceChildren();

        page.buttons.slice(0, capacity).forEach((button, index) => {
            const element = document.createElement(draftDeck ? "div" : "button");
            element.type = "button";
            element.className = "deck-button";
            element.dataset.buttonId = button.id;
            element.setAttribute("aria-label", button.label);
            setButtonAppearance(element, button);
            if (draftDeck) element.setAttribute("role", "group");
            if (button.custom_html?.trim()) {
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
                        page.buttons.splice(index, 1);
                        renderDeck();
                    }
                });
                tools.append(edit, remove);
                element.append(tools);
            }
            element.addEventListener("pointerdown", event => {
                if (draftDeck) return;
                if (event.button !== undefined && event.button !== 0) return;
                event.preventDefault();
                element.setPointerCapture(event.pointerId);
                press(button, true);
            });
            const release = event => {
                if (element.classList.contains("is-active")) {
                    if (event) event.preventDefault();
                    press(button, false);
                }
            };
            element.addEventListener("pointerup", release);
            element.addEventListener("pointercancel", release);
            element.addEventListener("lostpointercapture", release);
            element.addEventListener("keydown", event => {
                if (!draftDeck && (event.key === " " || event.key === "Enter") && !event.repeat) press(button, true);
            });
            element.addEventListener("keyup", event => {
                if (!draftDeck && (event.key === " " || event.key === "Enter")) press(button, false);
            });
            grid.append(element);
        });

        for (let index = page.buttons.length; index < capacity; index += 1) {
            if (draftDeck) {
                const addSlot = document.createElement("button");
                addSlot.type = "button";
                addSlot.className = "empty-slot add-slot";
                addSlot.textContent = "+";
                addSlot.setAttribute("aria-label", "Add button");
                addSlot.addEventListener("click", addDraftButton);
                grid.append(addSlot);
                continue;
            }
            const slot = document.createElement("div");
            slot.className = "empty-slot";
            slot.setAttribute("aria-hidden", "true");
            grid.append(slot);
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
        if (message.StatePatch?.ButtonActive) {
            const patch = message.StatePatch.ButtonActive;
            const button = grid.querySelector(`[data-button-id="${CSS.escape(patch.button_id)}"]`);
            button?.classList.toggle("is-active", patch.active);
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

    function makeActionEditor(action = { action: "hotkey", keys: ["ctrl", "d"] }) {
            const actionKind = action.action ?? action.kind ?? "hotkey";
            const row = document.createElement("fieldset");
            row.className = "action-row";
            const heading = document.createElement("div");
            heading.className = "action-row-heading";
            const kindLabel = document.createElement("label");
            kindLabel.className = "field compact";
            const kindCaption = document.createElement("span");
            kindCaption.textContent = "Action type";
            const kind = document.createElement("select");
            kind.className = "action-kind";
            [
                ["hotkey", "Hotkey"],
                ["url", "Open URL"],
                ["command", "Run command"],
                ["type", "Type text"],
                ["delay", "Delay"],
            ].forEach(([value, label]) => {
                const option = document.createElement("option");
                option.value = value;
                option.textContent = label;
                kind.append(option);
            });
            if (![...kind.options].some(option => option.value === actionKind)) {
                const option = document.createElement("option");
                option.value = actionKind;
                option.textContent = `Plugin: ${actionKind}`;
                kind.append(option);
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

            const updateFields = () => {
                valueFields.replaceChildren();
                const field = (name, label, value, placeholder = "") => {
                    const wrapper = document.createElement("label");
                    wrapper.className = "field compact";
                    const caption = document.createElement("span");
                    caption.textContent = label;
                    const input = document.createElement(name === "delay" ? "input" : "textarea");
                    input.dataset.value = name;
                    input.value = value ?? "";
                    input.placeholder = placeholder;
                    input.required =
                        (kind.value === "hotkey" && name === "keys")
                        || (kind.value === name && ["url", "command", "delay"].includes(name));
                    if (name === "delay") {
                        input.type = "number";
                        input.min = "0";
                        input.step = "0.1";
                    } else if (name === "keys") {
                        input.rows = 1;
                    } else {
                        input.rows = 2;
                    }
                    wrapper.append(caption, input);
                    valueFields.append(wrapper);
                };
                if (kind.value === "hotkey") {
                    field("keys", "Keys (comma-separated)", (action.keys ?? []).join(", "), "ctrl, shift, m");
                } else if (kind.value === "url") {
                    field("url", "URL", action.url, "https://example.com");
                } else if (kind.value === "command") {
                    field("command", "Command", action.command, "your-command");
                } else if (kind.value === "type") {
                    field("text", "Text", action.text, "Text to type");
                } else if (kind.value === "delay") {
                    field("delay", "Seconds", action.delay ?? 1, "1");
                } else {
                    field("keys", "Keys", (action.keys ?? []).join(", "));
                    field("text", "Text", action.text);
                    field("url", "URL", action.url);
                    field("command", "Command", action.command);
                    field("delay", "Seconds", action.delay);
                }
            };
            kind.addEventListener("change", () => {
                action = { action: kind.value };
                updateFields();
            });
            updateFields();
            return row;
        }

        function openButtonDialog(index = null) {
            const page = activeDeck().pages[currentPage];
            editingButtonIndex = index;
            const button = index === null
                ? {
                    id: `button-${Date.now().toString(36)}`,
                    label: "New button",
                    icon: null,
                    on_press: [{ action: "hotkey", keys: ["ctrl", "d"] }],
                }
                : page.buttons[index];
            buttonForm.elements.namedItem("id").value = button.id;
            buttonForm.elements.namedItem("label").value = button.label;
            buttonForm.elements.namedItem("icon").value = button.icon ?? "";
            buttonForm.elements.namedItem("background_image").value = button.background_image ?? "";
            buttonForm.elements.namedItem("custom_css").value = button.custom_css ?? "";
            buttonForm.elements.namedItem("custom_html").value = button.custom_html ?? "";
            document.querySelector("#dialog-title").textContent = index === null ? "Add button" : "Edit button";
            actionList.replaceChildren(...button.on_press.map(makeActionEditor));
            dialog.showModal();
        }

        function addDraftButton() {
            const page = activeDeck().pages[currentPage];
            if (page.buttons.length < Math.min(activeDeck().grid.cols * activeDeck().grid.rows, 80)) {
                openButtonDialog();
            }
        }

    function readAction(row) {
            const kind = row.querySelector(".action-kind").value;
            const action = { action: kind, keys: [], text: null, url: null, command: null, delay: null };
            const value = row.querySelector("[data-value]")?.value ?? "";
            if (kind === "hotkey") action.keys = value.split(",").map(key => key.trim()).filter(Boolean);
            if (kind === "url") action.url = value.trim() || null;
            if (kind === "command") action.command = value.trim() || null;
            if (kind === "type") action.text = value || null;
            if (kind === "delay") action.delay = value.trim() ? Number(value) : null;
            return action;
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
        draftDeck = structuredClone(deck);
        renderDeck();
    });
    gridSettingsButton.addEventListener("click", () => {
        if (!deck) return;
        if (!draftDeck) draftDeck = structuredClone(deck);
        gridForm.elements.namedItem("cols").value = draftDeck.grid.cols;
        gridForm.elements.namedItem("rows").value = draftDeck.grid.rows;
        renderDeck();
        gridDialog.showModal();
    });
    document.querySelector("#close-grid-dialog").addEventListener("click", () => gridDialog.close());
    document.querySelector("#cancel-grid-dialog").addEventListener("click", () => gridDialog.close());
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
        gridDialog.close();
        renderDeck();
    });
    addButton.addEventListener("click", addDraftButton);
    discardButton.addEventListener("click", () => {
        draftDeck = null;
        renderDeck();
    });
    saveButton.addEventListener("click", saveDraft);
    document.querySelector("#add-action").addEventListener("click", () => {
        actionList.append(makeActionEditor());
    });
    document.querySelector("#close-dialog").addEventListener("click", () => dialog.close());
    document.querySelector("#cancel-dialog").addEventListener("click", () => dialog.close());
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
        if (onPress.some(action => action.action === "hotkey" && action.keys.length === 0)) {
            showError("A hotkey action needs at least one key.");
            return;
        }
        const button = {
            id,
            label: buttonForm.elements.namedItem("label").value.trim(),
            icon: buttonForm.elements.namedItem("icon").value.trim() || null,
            background_image: buttonForm.elements.namedItem("background_image").value.trim() || null,
            custom_css: buttonForm.elements.namedItem("custom_css").value.trim() || null,
            custom_html: buttonForm.elements.namedItem("custom_html").value || null,
            on_press: onPress,
        };
        if (editingButtonIndex === null) page.buttons.push(button);
        else page.buttons[editingButtonIndex] = button;
        dialog.close();
        renderDeck();
    });
    dialog.addEventListener("close", () => {
        editingButtonIndex = null;
        buttonForm.reset();
    });
    isDesktop.addEventListener("change", renderDeck);
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
    connect();
    if ("serviceWorker" in navigator) {
        window.addEventListener("load", () => {
            navigator.serviceWorker.register("/service-worker.js").catch(error => {
                console.error("DeskDeck service worker registration failed:", error);
            });
        });
    }
})();
