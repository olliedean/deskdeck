(() => {
    const PROTOCOL_VERSION = 1;
    const reconnectDelay = 1800;
    const grid = document.querySelector("#buttons");
    const pageTabs = document.querySelector("#page-tabs");
    const toast = document.querySelector("#toast");
    let deck = null;
    let currentPage = 0;
    let socket = null;
    let reconnectTimer = null;
    let toastTimer = null;

    function showError(message) {
        toast.textContent = message;
        toast.hidden = false;
        clearTimeout(toastTimer);
        toastTimer = setTimeout(() => { toast.hidden = true; }, 5000);
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
        const pages = deck?.pages ?? [];
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
        if (!deck) return;
        const page = deck.pages?.[currentPage] ?? deck.pages?.[0];
        if (!page) {
            grid.replaceChildren();
            return;
        }
        currentPage = deck.pages.indexOf(page);
        renderPages();
        const cols = Math.max(1, Number(deck.grid?.cols) || 5);
        const rows = Math.max(1, Number(deck.grid?.rows) || 3);
        const capacity = Math.min(cols * rows, 80);
        grid.style.gridTemplateColumns = `repeat(${cols}, minmax(0, 1fr))`;
        grid.replaceChildren();

        page.buttons.slice(0, capacity).forEach((button, index) => {
            const element = document.createElement("button");
            element.type = "button";
            element.className = "deck-button";
            element.dataset.buttonId = button.id;
            element.setAttribute("aria-label", button.label);
            const icon = document.createElement("span");
            icon.className = "button-icon";
            icon.setAttribute("aria-hidden", "true");
            setIcon(icon, button.icon);
            const label = document.createElement("span");
            label.className = "button-label";
            label.textContent = button.label;
            element.append(icon, label);
            element.addEventListener("pointerdown", event => {
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
                if ((event.key === " " || event.key === "Enter") && !event.repeat) press(button, true);
            });
            element.addEventListener("keyup", event => {
                if (event.key === " " || event.key === "Enter") press(button, false);
            });
            grid.append(element);
        });

        for (let index = page.buttons.length; index < capacity; index += 1) {
            const slot = document.createElement("div");
            slot.className = "empty-slot";
            slot.setAttribute("aria-hidden", "true");
            grid.append(slot);
        }
    }

    function handleHostMessage(message) {
        if (message.Welcome) {
            deck = message.Welcome.deck;
            renderDeck();
            return;
        }
        if (message.DeckUpdated) {
            deck = message.DeckUpdated;
            currentPage = Math.min(currentPage, Math.max(0, deck.pages.length - 1));
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
