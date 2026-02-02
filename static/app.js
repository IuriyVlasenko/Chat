(() => {
  const statusEl = document.getElementById("status");
  const messagesEl = document.getElementById("messages");
  const formEl = document.getElementById("composer");
  const inputEl = document.getElementById("input");
  const languageEl = document.getElementById("language");

  const i18n = {
    en: {
      title: "Rust WebSocket Chat",
      subtitle: "Open two tabs to chat with yourself.",
      send: "Send",
      input: "Type a message",
      connecting: "Connecting…",
      connected: "Connected as",
      disconnected: "Disconnected",
      error: "Connection error",
    },
    uk: {
      title: "Rust WebSocket Чат",
      subtitle: "Відкрийте дві вкладки для чату.",
      send: "Надіслати",
      input: "Введіть повідомлення",
      connecting: "З'єднання…",
      connected: "Підключено як",
      disconnected: "Відключено",
      error: "Помилка з'єднання",
    },
    ru: {
      title: "Rust WebSocket Чат",
      subtitle: "Откройте две вкладки для чата.",
      send: "Отправить",
      input: "Введите сообщение",
      connecting: "Подключение…",
      connected: "Подключено как",
      disconnected: "Отключено",
      error: "Ошибка соединения",
    },
  };

  let userId = sessionStorage.getItem("chat_user_id");
  if (!userId) {
    userId = `User-${Math.floor(Math.random() * 10000)}`;
    sessionStorage.setItem("chat_user_id", userId);
  }

  let language = sessionStorage.getItem("chat_language") || "en";
  if (!i18n[language]) language = "en";
  languageEl.value = language;

  function t(key) {
    return i18n[language][key] || key;
  }

  function applyI18n() {
    document.querySelectorAll("[data-i18n]").forEach((el) => {
      const key = el.getAttribute("data-i18n");
      el.textContent = t(key);
    });

    document.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
      const key = el.getAttribute("data-i18n-placeholder");
      el.setAttribute("placeholder", t(key));
    });
  }

  function setStatus(key, ok, extra) {
    const base = t(key);
    statusEl.textContent = extra ? `${base} ${extra}` : base;
    statusEl.classList.toggle("ok", ok);
  }

  applyI18n();
  setStatus("connecting", false);

  languageEl.addEventListener("change", () => {
    language = languageEl.value;
    sessionStorage.setItem("chat_language", language);
    applyI18n();
  });

  const wsUrl = `${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/ws`;
  const socket = new WebSocket(wsUrl);

  function appendMessage(msg) {
    const item = document.createElement("div");
    item.className = msg.user === userId ? "message me" : "message";

    const header = document.createElement("div");
    header.className = "meta";
    const time = new Date(msg.ts * 1000).toLocaleTimeString();
    header.textContent = `${msg.user} · ${time}`;

    const body = document.createElement("div");
    body.className = "text";
    body.textContent = msg.text;

    item.appendChild(header);
    item.appendChild(body);
    messagesEl.appendChild(item);
    messagesEl.scrollTop = messagesEl.scrollHeight;
  }

  socket.addEventListener("open", () => setStatus("connected", true, userId));
  socket.addEventListener("close", () => setStatus("disconnected", false));
  socket.addEventListener("error", () => setStatus("error", false));
  socket.addEventListener("message", (event) => {
    try {
      const msg = JSON.parse(event.data);
      if (msg && msg.user && msg.text) {
        appendMessage(msg);
      }
    } catch (_) {
      // Ignore invalid messages
    }
  });

  formEl.addEventListener("submit", (event) => {
    event.preventDefault();
    const text = inputEl.value.trim();
    if (!text || socket.readyState !== WebSocket.OPEN) return;

    const msg = { user: userId, text };
    socket.send(JSON.stringify(msg));
    inputEl.value = "";
    inputEl.focus();
  });
})();