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
      grid-template-rows: auto auto 1fr;
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
        <span class="pill">audit <span id="auditState">-</span></span>
        <span class="pill">agents <span id="agentCount">-</span></span>
        <button class="icon" id="refreshHealth" title="Refresh health" aria-label="Refresh health">↻</button>
      </div>
    </header>

    <main>
      <aside class="rail">
        <section class="panel">
          <div class="panel-head">
            <h2 class="panel-title">Route</h2>
          </div>
          <div class="fields">
            <div class="segmented" aria-label="Protocol">
              <button id="protoOpenAI" class="active" type="button">OpenAI</button>
              <button id="protoAnthropic" type="button">Anthropic</button>
            </div>
            <label>
              Agent ID
              <input id="agentId" placeholder="default">
            </label>
            <div class="grid-2">
              <label>
                Model
                <input id="model" value="gpt-4o">
              </label>
              <label>
                Max tokens
                <input id="maxTokens" type="number" min="1" value="512">
              </label>
            </div>
            <label class="inline">
              <input id="stream" type="checkbox">
              <span>Stream</span>
            </label>
          </div>
        </section>

        <section class="panel">
          <div class="panel-head">
            <h2 class="panel-title">Headers</h2>
          </div>
          <div class="fields">
            <label>
              Authorization
              <input id="authHeader" placeholder="Bearer ...">
            </label>
            <label>
              Extra JSON
              <textarea id="extraHeaders" spellcheck="false">{
  "x-request-id": "ui-demo"
}</textarea>
            </label>
          </div>
        </section>

        <section class="panel request-area">
          <div class="panel-head">
            <h2 class="panel-title">Request</h2>
            <div class="toolbar">
              <button id="formatBody" class="icon" title="Format JSON" aria-label="Format JSON">{}</button>
            </div>
          </div>
          <textarea id="bodyEditor" spellcheck="false"></textarea>
        </section>
      </aside>

      <section class="workspace">
        <section class="panel">
          <div class="panel-head">
            <h2 class="panel-title">Dispatch</h2>
            <div class="toolbar">
              <span class="subtle" id="endpointPreview"></span>
              <span class="spacer"></span>
              <button id="copyCurl" type="button">Copy cURL</button>
              <button id="sendRequest" class="primary" type="button">Send</button>
            </div>
          </div>
        </section>

        <div class="output-grid">
          <section class="panel">
            <div class="panel-head">
              <h2 class="panel-title">Response</h2>
              <div class="toolbar">
                <button id="clearOutput" class="icon" title="Clear response" aria-label="Clear response">×</button>
              </div>
            </div>
            <pre class="codebox" id="responseBox">No response yet.</pre>
          </section>

          <section class="panel">
            <div class="panel-head">
              <h2 class="panel-title">Metadata</h2>
            </div>
            <div class="meta-list" id="metaList"></div>
          </section>
        </div>
      </section>
    </main>
  </div>

  <script>
    const state = {
      protocol: "openai",
      lastCurl: ""
    };

    const els = {
      originLabel: document.getElementById("originLabel"),
      healthDot: document.getElementById("healthDot"),
      healthText: document.getElementById("healthText"),
      auditState: document.getElementById("auditState"),
      agentCount: document.getElementById("agentCount"),
      refreshHealth: document.getElementById("refreshHealth"),
      protoOpenAI: document.getElementById("protoOpenAI"),
      protoAnthropic: document.getElementById("protoAnthropic"),
      agentId: document.getElementById("agentId"),
      model: document.getElementById("model"),
      maxTokens: document.getElementById("maxTokens"),
      stream: document.getElementById("stream"),
      authHeader: document.getElementById("authHeader"),
      extraHeaders: document.getElementById("extraHeaders"),
      bodyEditor: document.getElementById("bodyEditor"),
      formatBody: document.getElementById("formatBody"),
      endpointPreview: document.getElementById("endpointPreview"),
      copyCurl: document.getElementById("copyCurl"),
      sendRequest: document.getElementById("sendRequest"),
      clearOutput: document.getElementById("clearOutput"),
      responseBox: document.getElementById("responseBox"),
      metaList: document.getElementById("metaList")
    };

    function endpointPath() {
      const agent = els.agentId.value.trim();
      const base = agent ? `/v1/agents/${encodeURIComponent(agent)}` : "/v1";
      return state.protocol === "openai"
        ? `${base}/chat/completions`
        : `${base}/messages`;
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
      els.model.value = protocol === "openai" ? "gpt-4o" : "claude-3-haiku-20240307";
      writeBody(defaultBody());
      updateEndpointPreview();
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

    async function refreshHealth() {
      try {
        const response = await fetch("/health");
        const health = await response.json();
        els.healthDot.className = `dot ${response.ok ? "ok" : "bad"}`;
        els.healthText.textContent = health.status || response.status;
        els.auditState.textContent = health.audit_enabled ? "on" : "off";
        els.agentCount.textContent = health.upstream_agents ?? "-";
        renderMeta([
          ["health", JSON.stringify(health)],
          ["origin", location.origin],
          ["updated", new Date().toLocaleTimeString()]
        ]);
      } catch (error) {
        els.healthDot.className = "dot bad";
        els.healthText.textContent = "offline";
        renderMeta([
          ["health", "unreachable"],
          ["error", error.message]
        ]);
      }
    }

    async function sendRequest() {
      const started = performance.now();
      els.sendRequest.disabled = true;
      els.responseBox.textContent = "Sending...";

      try {
        const path = endpointPath();
        const headers = collectHeaders();
        const body = collectBody();
        state.lastCurl = makeCurl(path, headers, body);
        updateEndpointPreview();

        const response = await fetch(path, {
          method: "POST",
          headers,
          body: JSON.stringify(body)
        });

        const elapsed = Math.round(performance.now() - started);
        const requestId = response.headers.get("x-request-id") || "-";
        const riskLevel = response.headers.get("x-audit-risk-level") || response.headers.get("x-audit-mode") || "-";
        renderMeta([
          ["status", `${response.status} ${response.statusText}`],
          ["elapsed", `${elapsed} ms`],
          ["request id", requestId],
          ["audit", riskLevel],
          ["endpoint", path]
        ]);

        if (body.stream && response.body) {
          await readStream(response);
        } else {
          const text = await response.text();
          els.responseBox.textContent = formatMaybeJson(text);
        }
      } catch (error) {
        els.responseBox.textContent = error.message;
        renderMeta([
          ["status", "client error"],
          ["error", error.message]
        ]);
      } finally {
        els.sendRequest.disabled = false;
      }
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
      els.responseBox.textContent = output || "(empty stream)";
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
        ["copied", "cURL"],
        ["endpoint", path]
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
    els.refreshHealth.addEventListener("click", refreshHealth);
    els.protoOpenAI.addEventListener("click", () => updateProtocol("openai"));
    els.protoAnthropic.addEventListener("click", () => updateProtocol("anthropic"));
    els.agentId.addEventListener("input", updateEndpointPreview);
    els.stream.addEventListener("change", () => {
      const body = collectBody();
      body.stream = els.stream.checked;
      writeBody(body);
    });
    els.formatBody.addEventListener("click", () => writeBody(collectBody()));
    els.copyCurl.addEventListener("click", copyCurl);
    els.sendRequest.addEventListener("click", sendRequest);
    els.clearOutput.addEventListener("click", () => {
      els.responseBox.textContent = "No response yet.";
      renderMeta([]);
    });

    updateProtocol("openai");
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
        assert!(INDEX_HTML.contains("id=\"copyCurl\""));
        assert!(INDEX_HTML.contains("id=\"extraHeaders\""));
        assert!(INDEX_HTML.contains("id=\"responseBox\""));
        assert!(INDEX_HTML.contains("/v1/agents/"));
    }

    #[test]
    fn embedded_ui_declares_inline_favicon() {
        assert!(INDEX_HTML.contains("rel=\"icon\""));
        assert!(INDEX_HTML.contains("data:image/svg+xml"));
    }
}
