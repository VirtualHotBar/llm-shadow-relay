//! Embedded Web UI for LLM Shadow Relay

use axum::response::Html;

pub async fn web_ui() -> Html<&'static str> {
    Html(INDEX_HTML)
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>LLM Shadow Relay</title>
  <link rel="icon" href='data:image/svg+xml,%3Csvg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"%3E%3Crect width="32" height="32" rx="6" fill="%23f8fbfd"/%3E%3Cpath d="M8 16h16M16 8v16" stroke="%231769aa" stroke-width="3" stroke-linecap="round"/%3E%3Crect x="2" y="2" width="28" height="28" rx="6" fill="none" stroke="%23b8c4cf" stroke-width="2"/%3E%3C/svg%3E'>
  <style>
    :root {
      color-scheme: light;
      --bg: #eef1f4;
      --surface: #ffffff;
      --surface-2: #f7f9fb;
      --ink: #18212c;
      --muted: #64717f;
      --line: #d8e0e7;
      --line-strong: #b8c4cf;
      --primary: #1769aa;
      --primary-dark: #0f548b;
      --ok: #16735f;
      --warn: #a05a00;
      --bad: #b42318;
      --code: #0d1620;
      --code-bg: #101820;
      --focus: #6aa9e9;
      --radius: 6px;
      --mono: "Cascadia Mono", "SFMono-Regular", Consolas, monospace;
      --ui: "Segoe UI", "Aptos", "Helvetica Neue", sans-serif;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      min-height: 100vh;
      color: var(--ink);
      background: var(--bg);
      font-family: var(--ui);
      letter-spacing: 0;
    }

    button,
    input,
    select,
    textarea {
      font: inherit;
    }

    button {
      min-height: 36px;
      border: 1px solid var(--line-strong);
      border-radius: var(--radius);
      background: var(--surface);
      color: var(--ink);
      cursor: pointer;
      transition: background 140ms ease, border-color 140ms ease, color 140ms ease;
    }

    button:hover {
      border-color: #8da1b4;
      background: #f8fbfd;
    }

    button:focus-visible,
    input:focus-visible,
    select:focus-visible,
    textarea:focus-visible {
      outline: 2px solid var(--focus);
      outline-offset: 1px;
    }

    button.primary {
      border-color: var(--primary);
      background: var(--primary);
      color: #fff;
      font-weight: 650;
    }

    button.primary:hover {
      border-color: var(--primary-dark);
      background: var(--primary-dark);
    }

    button.icon {
      width: 36px;
      padding: 0;
      font-family: var(--mono);
      font-size: 16px;
      line-height: 1;
    }

    button:disabled {
      cursor: wait;
      opacity: 0.7;
    }

    .app {
      min-height: 100vh;
      display: grid;
      grid-template-rows: auto 1fr;
    }

    .topbar {
      display: grid;
      grid-template-columns: minmax(240px, 1fr) auto;
      gap: 16px;
      align-items: center;
      padding: 14px 18px;
      border-bottom: 1px solid var(--line);
      background: rgba(255, 255, 255, 0.92);
      backdrop-filter: blur(14px);
      position: sticky;
      top: 0;
      z-index: 10;
    }

    .brand {
      display: flex;
      align-items: center;
      gap: 12px;
      min-width: 0;
    }

    .mark {
      width: 32px;
      height: 32px;
      border: 1px solid var(--line-strong);
      border-radius: var(--radius);
      background:
        linear-gradient(90deg, transparent 47%, #8094a8 48%, #8094a8 52%, transparent 53%),
        linear-gradient(0deg, transparent 47%, #8094a8 48%, #8094a8 52%, transparent 53%),
        #f8fbfd;
      flex: 0 0 auto;
    }

    h1 {
      margin: 0;
      font-size: 17px;
      line-height: 1.2;
      font-weight: 700;
      text-wrap: pretty;
    }

    .subtle {
      color: var(--muted);
      font-size: 12px;
      line-height: 1.2;
    }

    .status-strip {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: flex-end;
      gap: 8px;
    }

    .pill {
      display: inline-flex;
      align-items: center;
      gap: 7px;
      min-height: 28px;
      padding: 0 10px;
      border: 1px solid var(--line);
      border-radius: 999px;
      background: var(--surface-2);
      color: var(--muted);
      font-size: 12px;
      white-space: nowrap;
    }

    .dot {
      width: 8px;
      height: 8px;
      border-radius: 999px;
      background: var(--muted);
    }

    .dot.ok { background: var(--ok); }
    .dot.bad { background: var(--bad); }
    .dot.warn { background: var(--warn); }

    main {
      display: grid;
      grid-template-columns: minmax(320px, 420px) minmax(0, 1fr);
      min-height: 0;
    }

    .rail,
    .workspace {
      min-width: 0;
      min-height: 0;
    }

    .rail {
      border-right: 1px solid var(--line);
      background: var(--surface);
      padding: 14px;
      display: grid;
      grid-template-rows: auto auto auto minmax(260px, 1fr);
      gap: 12px;
    }

    .workspace {
      padding: 14px;
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      gap: 12px;
    }

    .panel {
      border: 1px solid var(--line);
      border-radius: var(--radius);
      background: var(--surface);
      min-width: 0;
    }

    .panel-head {
      min-height: 42px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      padding: 9px 11px;
      border-bottom: 1px solid var(--line);
      background: var(--surface-2);
    }

    .panel-title {
      margin: 0;
      font-size: 13px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      color: #334253;
    }

    .fields {
      display: grid;
      gap: 10px;
      padding: 11px;
    }

    label {
      display: grid;
      gap: 5px;
      color: var(--muted);
      font-size: 12px;
      font-weight: 600;
    }

    input,
    select,
    textarea {
      width: 100%;
      border: 1px solid var(--line-strong);
      border-radius: var(--radius);
      background: #fff;
      color: var(--ink);
      padding: 8px 9px;
      min-height: 36px;
    }

    textarea {
      min-height: 88px;
      resize: vertical;
      font-family: var(--mono);
      font-size: 12px;
      line-height: 1.45;
      tab-size: 2;
    }

    .grid-2 {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 10px;
    }

    .segmented {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 4px;
      padding: 4px;
      border: 1px solid var(--line-strong);
      border-radius: var(--radius);
      background: var(--surface-2);
    }

    .segmented button {
      min-height: 30px;
      border: 0;
      background: transparent;
      color: var(--muted);
      font-size: 12px;
      font-weight: 650;
    }

    .segmented button.active {
      background: var(--surface);
      color: var(--primary);
      border: 1px solid var(--line);
    }

    .toolbar {
      display: flex;
      align-items: center;
      gap: 8px;
      flex-wrap: wrap;
    }

    .toolbar .spacer {
      flex: 1 1 auto;
    }

    .lang-select {
      width: auto;
      min-width: 96px;
      min-height: 28px;
      padding: 4px 8px;
      font-size: 12px;
    }

    .output-grid {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(260px, 34%);
      gap: 12px;
      min-height: 0;
    }

    .output-grid .panel {
      min-height: 0;
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
    }

    .codebox {
      margin: 0;
      min-height: 0;
      overflow: auto;
      padding: 12px;
      background: var(--code-bg);
      color: #d9e6f2;
      font-family: var(--mono);
      font-size: 12px;
      line-height: 1.5;
      white-space: pre-wrap;
      word-break: break-word;
    }

    .meta-list {
      padding: 11px;
      display: grid;
      gap: 8px;
      align-content: start;
      overflow: auto;
    }

    .meta-row {
      display: grid;
      grid-template-columns: 112px minmax(0, 1fr);
      gap: 8px;
      align-items: start;
      padding-bottom: 8px;
      border-bottom: 1px solid var(--line);
      font-size: 12px;
    }

    .meta-row strong {
      color: #334253;
      font-weight: 700;
    }

    .meta-row span {
      min-width: 0;
      color: var(--muted);
      overflow-wrap: anywhere;
    }

    .request-area {
      min-height: 0;
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
    }

    .request-area textarea {
      min-height: 260px;
      height: 100%;
      border-radius: 0 0 var(--radius) var(--radius);
      border: 0;
    }

    .inline {
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .inline input[type="checkbox"] {
      width: 16px;
      min-height: 16px;
      padding: 0;
    }

    .agent-picker {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      gap: 8px;
    }

    .history-list {
      padding: 8px;
      display: grid;
      gap: 8px;
      max-height: 220px;
      overflow: auto;
    }

    .history-item {
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 8px;
      align-items: center;
      padding: 8px;
      border: 1px solid var(--line);
      border-radius: var(--radius);
      background: #fff;
    }

    .history-title {
      color: var(--ink);
      font-size: 12px;
      font-weight: 700;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .history-subtitle {
      color: var(--muted);
      font-size: 11px;
      overflow-wrap: anywhere;
    }

    .warn-text {
      color: var(--warn);
    }

    .bad-text {
      color: var(--bad);
    }

    @media (max-width: 980px) {
      main,
      .output-grid {
        grid-template-columns: 1fr;
      }

      .rail {
        border-right: 0;
        border-bottom: 1px solid var(--line);
      }
    }

    @media (max-width: 640px) {
      .topbar {
        grid-template-columns: 1fr;
      }

      .status-strip {
        justify-content: flex-start;
      }

      .grid-2 {
        grid-template-columns: 1fr;
      }

      .agent-picker {
        grid-template-columns: 1fr;
      }

      main {
        display: block;
      }

      .rail,
      .workspace {
        padding: 10px;
      }

      .workspace > .panel .panel-head {
        align-items: stretch;
        flex-direction: column;
      }

      .workspace > .panel .toolbar {
        width: 100%;
      }

      .workspace > .panel .toolbar .spacer {
        display: none;
      }
    }
  </style>
</head>
<body>
  <div class="app">
    <header class="topbar">
      <div class="brand">
        <div class="mark" aria-hidden="true"></div>
        <div>
          <h1>LLM Shadow Relay</h1>
          <div class="subtle" id="originLabel"></div>
        </div>
      </div>
      <div class="status-strip">
        <span class="pill"><span class="dot" id="healthDot"></span><span id="healthText">checking</span></span>
        <span class="pill"><span data-i18n="audit">audit</span> <span id="auditState">-</span></span>
        <span class="pill"><span data-i18n="agents">agents</span> <span id="agentCount">-</span></span>
        <select id="languageSelect" class="lang-select" aria-label="Language">
          <option value="auto">Auto</option>
          <option value="en">English</option>
          <option value="zh">中文</option>
        </select>
        <button class="icon" id="refreshHealth" title="Refresh health" aria-label="Refresh health" data-i18n-title="refreshHealth" data-i18n-aria="refreshHealth">↻</button>
      </div>
    </header>

    <main>
      <aside class="rail">
        <section class="panel">
          <div class="panel-head">
            <h2 class="panel-title" data-i18n="route">Route</h2>
          </div>
          <div class="fields">
            <div class="segmented" aria-label="Protocol">
              <button id="protoOpenAI" class="active" type="button">OpenAI</button>
              <button id="protoAnthropic" type="button">Anthropic</button>
            </div>
            <label>
              <span data-i18n="agentId">Agent ID</span>
              <div class="agent-picker">
                <select id="agentSelect">
                  <option value="">default</option>
                  <option value="__custom">custom</option>
                </select>
                <input id="agentId" placeholder="custom agent id" data-i18n-placeholder="customAgentId">
              </div>
            </label>
            <div class="grid-2">
              <label>
                <span data-i18n="model">Model</span>
                <input id="model" value="gpt-4o">
              </label>
              <label>
                <span data-i18n="maxTokens">Max tokens</span>
                <input id="maxTokens" type="number" min="1" value="512">
              </label>
            </div>
            <label class="inline">
              <input id="stream" type="checkbox">
              <span data-i18n="stream">Stream</span>
            </label>
          </div>
        </section>

        <section class="panel">
          <div class="panel-head">
            <h2 class="panel-title" data-i18n="headersPanel">Headers</h2>
          </div>
          <div class="fields">
            <label>
              Authorization
              <input id="authHeader" placeholder="Bearer ...">
            </label>
            <label>
              <span data-i18n="extraJson">Extra JSON</span>
              <textarea id="extraHeaders" spellcheck="false">{
  "x-request-id": "ui-demo"
}</textarea>
            </label>
          </div>
        </section>

        <section class="panel">
          <div class="panel-head">
            <h2 class="panel-title" data-i18n="history">History</h2>
            <div class="toolbar">
              <button id="clearHistory" class="icon" title="Clear history" aria-label="Clear history" data-i18n-title="clearHistory" data-i18n-aria="clearHistory">×</button>
            </div>
          </div>
          <div class="history-list" id="historyList">
            <div class="subtle" data-i18n="noRequests">No requests yet.</div>
          </div>
        </section>

        <section class="panel request-area">
          <div class="panel-head">
            <h2 class="panel-title" data-i18n="request">Request</h2>
            <div class="toolbar">
              <button id="formatBody" class="icon" title="Format JSON" aria-label="Format JSON" data-i18n-title="formatJson" data-i18n-aria="formatJson">{}</button>
            </div>
          </div>
          <textarea id="bodyEditor" spellcheck="false"></textarea>
        </section>
      </aside>

      <section class="workspace">
        <section class="panel">
          <div class="panel-head">
            <h2 class="panel-title" data-i18n="dispatch">Dispatch</h2>
            <div class="toolbar">
              <span class="subtle" id="endpointPreview"></span>
              <span class="spacer"></span>
              <button id="previewRequest" type="button" data-i18n="preview">Preview</button>
              <button id="copyCurl" type="button" data-i18n="copyCurl">Copy cURL</button>
              <button id="abortRequest" type="button" disabled data-i18n="abort">Abort</button>
              <button id="sendRequest" class="primary" type="button" data-i18n="send">Send</button>
            </div>
          </div>
        </section>

        <div class="output-grid">
          <section class="panel">
            <div class="panel-head">
              <h2 class="panel-title" data-i18n="response">Response</h2>
              <div class="toolbar">
                <button id="copyResponse" type="button" data-i18n="copyResponse">Copy response</button>
                <button id="clearOutput" class="icon" title="Clear response" aria-label="Clear response" data-i18n-title="clearResponse" data-i18n-aria="clearResponse">×</button>
              </div>
            </div>
            <pre class="codebox" id="responseBox">No response yet.</pre>
          </section>

          <section class="panel">
            <div class="panel-head">
              <h2 class="panel-title" data-i18n="metadata">Metadata</h2>
            </div>
            <div class="meta-list" id="metaList"></div>
          </section>
        </div>
      </section>
    </main>
  </div>

  <script>
    const I18N = {
      en: {
        abort: "Abort",
        aborted: "aborted",
        agents: "agents",
        agentId: "Agent ID",
        audit: "audit",
        authConfigured: "configured",
        authPassthrough: "client passthrough",
        checking: "checking",
        clearHistory: "Clear history",
        clearResponse: "Clear response",
        clientError: "client error",
        copied: "copied",
        copyCurl: "Copy cURL",
        copyHistoryCurl: "cURL",
        copyResponse: "Copy response",
        customAgentId: "custom agent id",
        dispatch: "Dispatch",
        emptyStream: "(empty stream)",
        extraJson: "Extra JSON",
        formatJson: "Format JSON",
        headersPanel: "Headers",
        headersOff: "passthrough off",
        headersOn: "passthrough on",
        health: "health",
        healthy: "healthy",
        history: "History",
        historyCleared: "cleared",
        historyCurl: "history cURL",
        historyRestored: "restored",
        invalid: "invalid",
        maxTokens: "Max tokens",
        metadata: "Metadata",
        model: "Model",
        noRequests: "No requests yet.",
        noResponse: "No response yet.",
        offline: "offline",
        off: "off",
        on: "on",
        preview: "Preview",
        request: "Request",
        requestAborted: "Request aborted.",
        requestAbortedByUser: "Request aborted by user",
        response: "Response",
        route: "Route",
        send: "Send",
        sending: "Sending...",
        use: "Use",
        stream: "Stream",
        uiConfig: "ui config",
        unavailable: "unavailable",
        unreachable: "unreachable",
        updated: "updated"
      },
      zh: {
        abort: "中止",
        aborted: "已中止",
        agents: "智能体",
        agentId: "Agent ID",
        audit: "审计",
        authConfigured: "已配置",
        authPassthrough: "客户端透传",
        checking: "检查中",
        clearHistory: "清空历史",
        clearResponse: "清空响应",
        clientError: "客户端错误",
        copied: "已复制",
        copyCurl: "复制 cURL",
        copyHistoryCurl: "cURL",
        copyResponse: "复制响应",
        customAgentId: "自定义 agent id",
        dispatch: "发送",
        emptyStream: "(空流)",
        extraJson: "额外 JSON",
        formatJson: "格式化 JSON",
        headersPanel: "请求头",
        headersOff: "透传关闭",
        headersOn: "透传开启",
        health: "健康",
        healthy: "健康",
        history: "历史",
        historyCleared: "已清空",
        historyCurl: "历史 cURL",
        historyRestored: "已恢复",
        invalid: "无效",
        maxTokens: "最大 tokens",
        metadata: "元数据",
        model: "模型",
        noRequests: "暂无请求。",
        noResponse: "暂无响应。",
        offline: "离线",
        off: "关",
        on: "开",
        preview: "预览",
        request: "请求",
        requestAborted: "请求已中止。",
        requestAbortedByUser: "用户已中止请求",
        response: "响应",
        route: "路由",
        send: "发送",
        sending: "发送中...",
        use: "使用",
        stream: "流式",
        uiConfig: "UI 配置",
        unavailable: "不可用",
        unreachable: "不可达",
        updated: "更新时间"
      }
    };

    const LANGUAGE_STORAGE_KEY = "llm-shadow-relay-language";

    const state = {
      protocol: "openai",
      lastCurl: "",
      uiConfig: null,
      activeController: null,
      history: [],
      language: "en",
      languageMode: "auto"
    };

    const els = {
      originLabel: document.getElementById("originLabel"),
      healthDot: document.getElementById("healthDot"),
      healthText: document.getElementById("healthText"),
      auditState: document.getElementById("auditState"),
      agentCount: document.getElementById("agentCount"),
      languageSelect: document.getElementById("languageSelect"),
      refreshHealth: document.getElementById("refreshHealth"),
      protoOpenAI: document.getElementById("protoOpenAI"),
      protoAnthropic: document.getElementById("protoAnthropic"),
      agentSelect: document.getElementById("agentSelect"),
      agentId: document.getElementById("agentId"),
      model: document.getElementById("model"),
      maxTokens: document.getElementById("maxTokens"),
      stream: document.getElementById("stream"),
      authHeader: document.getElementById("authHeader"),
      extraHeaders: document.getElementById("extraHeaders"),
      clearHistory: document.getElementById("clearHistory"),
      historyList: document.getElementById("historyList"),
      bodyEditor: document.getElementById("bodyEditor"),
      formatBody: document.getElementById("formatBody"),
      endpointPreview: document.getElementById("endpointPreview"),
      previewRequest: document.getElementById("previewRequest"),
      copyCurl: document.getElementById("copyCurl"),
      abortRequest: document.getElementById("abortRequest"),
      sendRequest: document.getElementById("sendRequest"),
      copyResponse: document.getElementById("copyResponse"),
      clearOutput: document.getElementById("clearOutput"),
      responseBox: document.getElementById("responseBox"),
      metaList: document.getElementById("metaList")
    };

    function resolveLanguage(mode) {
      if (mode === "zh" || mode === "en") return mode;
      return navigator.language?.toLowerCase().startsWith("zh") ? "zh" : "en";
    }

    function t(key) {
      return I18N[state.language]?.[key] || I18N.en[key] || key;
    }

    function storedLanguageMode() {
      try {
        const value = localStorage.getItem(LANGUAGE_STORAGE_KEY);
        return ["auto", "en", "zh"].includes(value) ? value : "auto";
      } catch (_) {
        return "auto";
      }
    }

    function storeLanguageMode(mode) {
      try {
        localStorage.setItem(LANGUAGE_STORAGE_KEY, mode);
      } catch (_) {
        // Language preference is convenience-only; ignore restricted storage.
      }
    }

    function setLanguage(mode, persist = true) {
      state.languageMode = mode;
      state.language = resolveLanguage(mode);
      document.documentElement.lang = state.language === "zh" ? "zh-CN" : "en";
      els.languageSelect.value = mode;
      if (persist) storeLanguageMode(mode);
      applyI18n();
    }

    function applyI18n() {
      document.querySelectorAll("[data-i18n]").forEach((node) => {
        node.textContent = t(node.dataset.i18n);
      });
      document.querySelectorAll("[data-i18n-placeholder]").forEach((node) => {
        node.setAttribute("placeholder", t(node.dataset.i18nPlaceholder));
      });
      document.querySelectorAll("[data-i18n-title]").forEach((node) => {
        node.setAttribute("title", t(node.dataset.i18nTitle));
      });
      document.querySelectorAll("[data-i18n-aria]").forEach((node) => {
        node.setAttribute("aria-label", t(node.dataset.i18nAria));
      });
      renderHistory();
      if (els.responseBox.textContent === I18N.en.noResponse || els.responseBox.textContent === I18N.zh.noResponse) {
        els.responseBox.textContent = t("noResponse");
      }
      if ([I18N.en.healthy, I18N.zh.healthy].includes(els.healthText.textContent)) {
        els.healthText.textContent = t("healthy");
      }
      if ([I18N.en.checking, I18N.zh.checking].includes(els.healthText.textContent)) {
        els.healthText.textContent = t("checking");
      }
      if ([I18N.en.offline, I18N.zh.offline].includes(els.healthText.textContent)) {
        els.healthText.textContent = t("offline");
      }
      if ([I18N.en.on, I18N.zh.on].includes(els.auditState.textContent)) {
        els.auditState.textContent = t("on");
      }
      if ([I18N.en.off, I18N.zh.off].includes(els.auditState.textContent)) {
        els.auditState.textContent = t("off");
      }
    }

    function endpointPath() {
      const agent = selectedAgentId();
      const base = agent ? `/v1/agents/${encodeURIComponent(agent)}` : "/v1";
      return state.protocol === "openai"
        ? `${base}/chat/completions`
        : `${base}/messages`;
    }

    function selectedAgentId() {
      return els.agentSelect.value === "__custom"
        ? els.agentId.value.trim()
        : els.agentSelect.value.trim();
    }

    function selectedRouteConfig() {
      const id = selectedAgentId();
      if (!id || !state.uiConfig) return state.uiConfig?.upstream || null;
      return state.uiConfig.agents.find((agent) => agent.id === id) || null;
    }

    function defaultBody() {
      if (state.protocol === "openai") {
        return {
          model: els.model.value.trim(),
          stream: els.stream.checked,
          messages: [
            { role: "system", content: "You are a concise assistant." },
            { role: "user", content: "Say hello through the relay." }
          ]
        };
      }
      return {
        model: els.model.value.trim(),
        max_tokens: Number(els.maxTokens.value || 512),
        stream: els.stream.checked,
        messages: [
          { role: "user", content: "Say hello through the relay." }
        ]
      };
    }

    function updateProtocol(protocol) {
      state.protocol = protocol;
      els.protoOpenAI.classList.toggle("active", protocol === "openai");
      els.protoAnthropic.classList.toggle("active", protocol === "anthropic");
      const route = selectedRouteConfig();
      els.model.value = route?.default_model || fallbackModel(protocol);
      writeBody(defaultBody());
      updateEndpointPreview();
    }

    function fallbackModel(protocol) {
      return protocol === "openai" ? "gpt-4o" : "claude-3-haiku-20240307";
    }

    function writeBody(value) {
      els.bodyEditor.value = JSON.stringify(value, null, 2);
    }

    function parseJsonField(text, fallback) {
      const trimmed = text.trim();
      if (!trimmed) return fallback;
      return JSON.parse(trimmed);
    }

    function collectHeaders() {
      const headers = {
        "content-type": "application/json"
      };
      const auth = els.authHeader.value.trim();
      if (auth) headers.authorization = auth;
      const extra = parseJsonField(els.extraHeaders.value, {});
      for (const [key, value] of Object.entries(extra)) {
        if (value !== null && value !== undefined && String(value).trim() !== "") {
          headers[key] = String(value);
        }
      }
      return headers;
    }

    function collectBody() {
      const body = parseJsonField(els.bodyEditor.value, {});
      if (state.protocol === "openai") {
        if (els.model.value.trim()) body.model = els.model.value.trim();
        body.stream = els.stream.checked;
      } else {
        if (els.model.value.trim()) body.model = els.model.value.trim();
        body.max_tokens = Number(els.maxTokens.value || body.max_tokens || 512);
        body.stream = els.stream.checked;
      }
      return body;
    }

    function updateEndpointPreview() {
      els.endpointPreview.textContent = endpointPath();
    }

    async function loadUiConfig() {
      try {
        const response = await fetch("/ui/config");
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        state.uiConfig = await response.json();
        applyUiConfig();
      } catch (error) {
        renderMeta([
          ["ui config", "unavailable"],
          ["error", error.message]
        ]);
      }
    }

    function applyUiConfig() {
      if (!state.uiConfig) return;

      els.agentSelect.innerHTML = [
        `<option value="">default</option>`,
        ...state.uiConfig.agents.map((agent) =>
          `<option value="${escapeAttribute(agent.id)}">${escapeHtml(agent.id)}</option>`
        ),
        `<option value="__custom">custom</option>`
      ].join("");

      const protocol = state.uiConfig.upstream.protocol || "openai";
      updateProtocol(protocol);
      els.agentCount.textContent = state.uiConfig.agents.length;
      els.auditState.textContent = state.uiConfig.audit.enabled ? t("on") : t("off");
      renderRouteMeta();
    }

    function updateAgentSelection() {
      const custom = els.agentSelect.value === "__custom";
      els.agentId.disabled = !custom;
      if (!custom) els.agentId.value = "";

      const route = selectedRouteConfig();
      if (route) {
        updateProtocol(route.protocol || "openai");
      } else {
        updateEndpointPreview();
      }
      renderRouteMeta();
    }

    function renderRouteMeta() {
      const route = selectedRouteConfig();
      if (!route) return;
      renderMeta([
        ["route", selectedAgentId() || "default"],
        ["protocol", route.protocol],
        ["model", route.default_model],
        ["headers", route.pass_through_headers ? t("headersOn") : t("headersOff")],
        ["auth", route.has_configured_api_key ? t("authConfigured") : t("authPassthrough")]
      ]);
    }

    function escapeAttribute(value) {
      return escapeHtml(String(value)).replaceAll("'", "&#39;");
    }

    function renderMeta(rows) {
      els.metaList.innerHTML = rows.map(([key, value]) => `
        <div class="meta-row">
          <strong>${escapeHtml(key)}</strong>
          <span>${escapeHtml(String(value || "-"))}</span>
        </div>
      `).join("");
    }

    function escapeHtml(value) {
      return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;");
    }

    function makeCurl(path, headers, body) {
      const headerFlags = Object.entries(headers)
        .map(([key, value]) => `  -H ${JSON.stringify(`${key}: ${value}`)} \\`)
        .join("\n");
      return `curl ${JSON.stringify(location.origin + path)} \\\n${headerFlags}\n  -d ${JSON.stringify(JSON.stringify(body))}`;
    }

    function requestPreview() {
      const path = endpointPath();
      const headers = collectHeaders();
      const body = collectBody();
      return {
        method: "POST",
        url: location.origin + path,
        headers: redactHeaders(headers),
        body
      };
    }

    function requestSnapshot(kind) {
      const path = endpointPath();
      const headers = collectHeaders();
      const body = collectBody();
      return {
        id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
        kind,
        time: new Date().toLocaleTimeString(),
        protocol: state.protocol,
        agent: selectedAgentId(),
        model: body.model || "",
        path,
        headers,
        body,
        auth: els.authHeader.value,
        extraHeadersText: els.extraHeaders.value,
        maxTokens: els.maxTokens.value,
        stream: els.stream.checked
      };
    }

    function pushHistory(snapshot) {
      state.history = [
        snapshot,
        ...state.history.filter((item) => item.path !== snapshot.path || item.model !== snapshot.model)
      ].slice(0, 8);
      renderHistory();
    }

    function renderHistory() {
      if (state.history.length === 0) {
        els.historyList.innerHTML = `<div class="subtle">${escapeHtml(t("noRequests"))}</div>`;
        return;
      }

      els.historyList.innerHTML = state.history.map((item) => `
        <div class="history-item">
          <div>
            <div class="history-title">${escapeHtml(item.time)} · ${escapeHtml(item.model || "model")}</div>
            <div class="history-subtitle">${escapeHtml(item.kind)} · ${escapeHtml(item.path)}</div>
          </div>
          <div class="toolbar">
            <button type="button" data-history-use="${escapeAttribute(item.id)}">${escapeHtml(t("use"))}</button>
            <button type="button" data-history-curl="${escapeAttribute(item.id)}">${escapeHtml(t("copyHistoryCurl"))}</button>
          </div>
        </div>
      `).join("");
    }

    function historyItem(id) {
      return state.history.find((item) => item.id === id);
    }

    function restoreHistory(id) {
      const item = historyItem(id);
      if (!item) return;

      if (item.agent) {
        const hasConfiguredAgent = Array.from(els.agentSelect.options).some((option) => option.value === item.agent);
        els.agentSelect.value = hasConfiguredAgent ? item.agent : "__custom";
        els.agentId.disabled = hasConfiguredAgent;
        els.agentId.value = hasConfiguredAgent ? "" : item.agent;
      } else {
        els.agentSelect.value = "";
        els.agentId.disabled = true;
        els.agentId.value = "";
      }

      updateProtocol(item.protocol);
      els.authHeader.value = item.auth || "";
      els.extraHeaders.value = item.extraHeadersText || "{}";
      els.maxTokens.value = item.maxTokens || "512";
      els.stream.checked = item.stream;
      els.model.value = item.model || "";
      writeBody(item.body);
      updateEndpointPreview();
      renderMeta([
        [t("history"), t("historyRestored")],
        ["endpoint", item.path],
        ["time", item.time]
      ]);
    }

    async function copyHistoryCurl(id) {
      const item = historyItem(id);
      if (!item) return;
      await writeClipboard(makeCurl(item.path, item.headers, item.body));
      renderMeta([
        [t("copied"), t("historyCurl")],
        ["endpoint", item.path]
      ]);
    }

    function onHistoryClick(event) {
      const useId = event.target?.dataset?.historyUse;
      const curlId = event.target?.dataset?.historyCurl;
      if (useId) restoreHistory(useId);
      if (curlId) copyHistoryCurl(curlId);
    }

    function clearHistory() {
      state.history = [];
      renderHistory();
      renderMeta([[t("history"), t("historyCleared")]]);
    }

    function previewRequest() {
      try {
        pushHistory(requestSnapshot("preview"));
        const preview = requestPreview();
        state.lastCurl = makeCurl(endpointPath(), collectHeaders(), preview.body);
        els.responseBox.textContent = JSON.stringify(preview, null, 2);
        renderMeta([
          [t("preview"), t("request")],
          ["endpoint", endpointPath()],
          ["headers", JSON.stringify(preview.headers)]
        ]);
      } catch (error) {
        els.responseBox.textContent = error.message;
        renderMeta([
          [t("preview"), t("invalid")],
          ["error", error.message]
        ]);
      }
    }

    function redactHeaders(headers) {
      const redacted = {};
      for (const [key, value] of Object.entries(headers)) {
        redacted[key] = shouldRedactHeader(key) ? redactSecret(String(value)) : String(value);
      }
      return redacted;
    }

    function responseHeaders(response) {
      const headers = {};
      for (const [key, value] of response.headers.entries()) {
        headers[key] = shouldRedactHeader(key) ? redactSecret(value) : value;
      }
      return headers;
    }

    function shouldRedactHeader(key) {
      const normalized = key.toLowerCase();
      return normalized === "authorization"
        || normalized === "x-api-key"
        || normalized.includes("token")
        || normalized.includes("secret")
        || normalized.includes("key");
    }

    function redactSecret(value) {
      if (!value) return "";
      if (value.length <= 12) return "***";
      return `${value.slice(0, 8)}...${value.slice(-4)}`;
    }

    async function refreshHealth() {
      try {
        const response = await fetch("/health");
        const health = await response.json();
        els.healthDot.className = `dot ${response.ok ? "ok" : "bad"}`;
        els.healthText.textContent = health.status === "healthy" ? t("healthy") : (health.status || response.status);
        els.auditState.textContent = health.audit_enabled ? t("on") : t("off");
        els.agentCount.textContent = health.upstream_agents ?? "-";
        renderMeta([
          [t("health"), JSON.stringify(health)],
          ["origin", location.origin],
          [t("updated"), new Date().toLocaleTimeString()]
        ]);
      } catch (error) {
        els.healthDot.className = "dot bad";
        els.healthText.textContent = t("offline");
        renderMeta([
          [t("health"), t("unreachable")],
          ["error", error.message]
        ]);
      }
    }

    async function sendRequest() {
      const started = performance.now();
      const controller = new AbortController();
      state.activeController = controller;
      setBusy(true);
      els.responseBox.textContent = t("sending");

      try {
        const path = endpointPath();
        const headers = collectHeaders();
        const body = collectBody();
        pushHistory(requestSnapshot("send"));
        state.lastCurl = makeCurl(path, headers, body);
        updateEndpointPreview();

        const response = await fetch(path, {
          method: "POST",
          headers,
          body: JSON.stringify(body),
          signal: controller.signal
        });

        const elapsed = Math.round(performance.now() - started);
        const requestId = response.headers.get("x-request-id") || "-";
        const riskLevel = response.headers.get("x-audit-risk-level") || response.headers.get("x-audit-mode") || "-";
        renderMeta([
          ["status", `${response.status} ${response.statusText}`],
          ["elapsed", `${elapsed} ms`],
          ["request id", requestId],
          ["audit", riskLevel],
          ["endpoint", path],
          ["request headers", JSON.stringify(redactHeaders(headers))],
          ["response headers", JSON.stringify(responseHeaders(response))]
        ]);

        if (body.stream && response.body) {
          await readStream(response);
        } else {
          const text = await response.text();
          els.responseBox.textContent = formatMaybeJson(text);
        }
      } catch (error) {
        const aborted = error.name === "AbortError";
        els.responseBox.textContent = aborted ? t("requestAborted") : error.message;
        renderMeta([
          ["status", aborted ? t("aborted") : t("clientError")],
          ["error", aborted ? t("requestAbortedByUser") : error.message]
        ]);
      } finally {
        if (state.activeController === controller) {
          state.activeController = null;
          setBusy(false);
        }
      }
    }

    function setBusy(isBusy) {
      els.sendRequest.disabled = isBusy;
      els.abortRequest.disabled = !isBusy;
      els.previewRequest.disabled = isBusy;
      els.copyCurl.disabled = isBusy;
    }

    function abortRequest() {
      state.activeController?.abort();
    }

    async function readStream(response) {
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let output = "";
      els.responseBox.textContent = "";

      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        output += decoder.decode(value, { stream: true });
        els.responseBox.textContent = output;
        els.responseBox.scrollTop = els.responseBox.scrollHeight;
      }

      output += decoder.decode();
      els.responseBox.textContent = output || t("emptyStream");
    }

    function formatMaybeJson(text) {
      try {
        return JSON.stringify(JSON.parse(text), null, 2);
      } catch (_) {
        return text;
      }
    }

    async function copyCurl() {
      const path = endpointPath();
      const headers = collectHeaders();
      const body = collectBody();
      state.lastCurl = makeCurl(path, headers, body);
      await writeClipboard(state.lastCurl);
      renderMeta([
        [t("copied"), "cURL"],
        ["endpoint", path]
      ]);
    }

    async function copyResponse() {
      const text = els.responseBox.textContent || "";
      await writeClipboard(text);
      renderMeta([
        [t("copied"), t("response")],
        ["bytes", text.length]
      ]);
    }

    async function writeClipboard(text) {
      if (navigator.clipboard?.writeText) {
        try {
          await navigator.clipboard.writeText(text);
          return;
        } catch (_) {
          // Fall through to the textarea method for restricted browser contexts.
        }
      }

      const scratch = document.createElement("textarea");
      scratch.value = text;
      scratch.setAttribute("readonly", "");
      scratch.style.position = "fixed";
      scratch.style.left = "-9999px";
      document.body.appendChild(scratch);
      scratch.select();
      const ok = document.execCommand("copy");
      document.body.removeChild(scratch);
      if (!ok) throw new Error("Clipboard copy is unavailable in this browser context");
    }

    els.originLabel.textContent = location.origin;
    els.languageSelect.addEventListener("change", () => setLanguage(els.languageSelect.value));
    els.refreshHealth.addEventListener("click", refreshHealth);
    els.protoOpenAI.addEventListener("click", () => updateProtocol("openai"));
    els.protoAnthropic.addEventListener("click", () => updateProtocol("anthropic"));
    els.agentSelect.addEventListener("change", updateAgentSelection);
    els.agentId.addEventListener("input", updateEndpointPreview);
    els.historyList.addEventListener("click", onHistoryClick);
    els.clearHistory.addEventListener("click", clearHistory);
    els.stream.addEventListener("change", () => {
      const body = collectBody();
      body.stream = els.stream.checked;
      writeBody(body);
    });
    els.formatBody.addEventListener("click", () => writeBody(collectBody()));
    els.previewRequest.addEventListener("click", previewRequest);
    els.copyCurl.addEventListener("click", copyCurl);
    els.abortRequest.addEventListener("click", abortRequest);
    els.sendRequest.addEventListener("click", sendRequest);
    els.copyResponse.addEventListener("click", copyResponse);
    els.clearOutput.addEventListener("click", () => {
      els.responseBox.textContent = t("noResponse");
      renderMeta([]);
    });

    els.agentId.disabled = true;
    setLanguage(storedLanguageMode(), false);
    renderHistory();
    updateProtocol("openai");
    loadUiConfig();
    refreshHealth();
  </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::INDEX_HTML;

    #[test]
    fn embedded_ui_contains_core_controls() {
        assert!(INDEX_HTML.contains("id=\"sendRequest\""));
        assert!(INDEX_HTML.contains("id=\"abortRequest\""));
        assert!(INDEX_HTML.contains("id=\"previewRequest\""));
        assert!(INDEX_HTML.contains("id=\"copyResponse\""));
        assert!(INDEX_HTML.contains("id=\"copyCurl\""));
        assert!(INDEX_HTML.contains("id=\"historyList\""));
        assert!(INDEX_HTML.contains("id=\"extraHeaders\""));
        assert!(INDEX_HTML.contains("id=\"agentSelect\""));
        assert!(INDEX_HTML.contains("id=\"responseBox\""));
        assert!(INDEX_HTML.contains("id=\"languageSelect\""));
        assert!(INDEX_HTML.contains("/v1/agents/"));
        assert!(INDEX_HTML.contains("/ui/config"));
    }

    #[test]
    fn embedded_ui_declares_inline_favicon() {
        assert!(INDEX_HTML.contains("rel=\"icon\""));
        assert!(INDEX_HTML.contains("data:image/svg+xml"));
    }

    #[test]
    fn embedded_ui_redacts_sensitive_header_names() {
        assert!(INDEX_HTML.contains("shouldRedactHeader"));
        assert!(INDEX_HTML.contains("authorization"));
        assert!(INDEX_HTML.contains("x-api-key"));
    }

    #[test]
    fn embedded_ui_contains_language_adapter() {
        assert!(INDEX_HTML.contains("const I18N"));
        assert!(INDEX_HTML.contains("setLanguage"));
        assert!(INDEX_HTML.contains("localStorage"));
        assert!(INDEX_HTML.contains("llm-shadow-relay-language"));
        assert!(INDEX_HTML.contains("中文"));
        assert!(INDEX_HTML.contains("复制响应"));
    }
}
