# LLM Shadow Relay

An audited LLM API proxy that sits between your clients and LLM providers. It intercepts requests/responses and audits them with a trusted model to detect prompt injection, system prompt extraction, jailbreak attempts, and other security risks — before they reach your application.

## Features

- **Dual protocol support** — Accept both OpenAI (`/v1/chat/completions`) and Anthropic (`/v1/messages`) client formats, and forward to either protocol upstream
- **Automatic protocol conversion** — Client talks OpenAI? Upstream talks Anthropic? No problem. The proxy transparently converts between formats.
- **Request-side + response-side auditing** — Audits incoming prompts for injection attempts AND outgoing responses for leaked data or malicious content
- **Dual audit mode**: `sync` (blocking — waits for audit before responding) or `async` (non-blocking — returns immediately, audits in background)
- **Streaming support** — SSE streaming with real-time chunk-level audit
- **Configurable policy engine** — Block by risk level, category, custom keywords, or risk score threshold
- **Provider-flexible audit** — Audit via OpenAI-compatible, Anthropic, Ollama, or any local model
- **Audit metadata headers** — Risk level and score returned in response headers (sync mode)

## Architecture

```
┌─────────┐   OpenAI or   ┌──────────────────┐   OpenAI or   ┌──────────┐
│ Client  │ ────────────▶ │ LLM Shadow Relay │ ────────────▶ │ Upstream │
│ (App)   │ ◀──────────── │   (audit proxy)  │ ◀──────────── │  LLM API │
└─────────┘               └──────────────────┘               └──────────┘
                                  │
                                  │ audit via
                                  ▼
                          ┌──────────────────┐
                          │   Audit Model    │
                          │  (trusted, tiny) │
                          └──────────────────┘
```

## Quick Start

### 1. Build

```bash
cargo build --release
```

The binary is `target/release/llm-shadow-relay.exe` (Windows) or `target/release/llm-shadow-relay` (Linux/macOS).

### 2. Configure

Copy `config.example.toml` to `config.toml` and edit:

```toml
[upstream]
base_url = "https://api.openai.com/v1"
api_key = "sk-your-upstream-key"
default_model = "gpt-4o"
protocol = "openai"          # "openai" or "anthropic"

[audit]
provider = "openai"          # "openai", "anthropic", "ollama", "local"
base_url = "https://api.openai.com/v1"
api_key = "sk-your-audit-key"
model = "gpt-4o-mini"
enabled = true
mode = "sync"                # "sync" or "async"
```

### 3. Run

```bash
./target/release/llm-shadow-relay
# → Listening on 127.0.0.1:8080
```

### 4. Use

**OpenAI client:**
```bash
curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"Hello"}]}'
```

**Anthropic client:**
```bash
curl http://127.0.0.1:8080/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-3","max_tokens":1024,"messages":[{"role":"user","content":"Hello"}]}'
```

## Configuration Reference

### `[server]`

| Field | Default | Description |
|-------|---------|-------------|
| `listen` | `"127.0.0.1:8080"` | Server binding address |
| `timeout` | `300` | Request timeout in seconds |
| `max_request_mb` | `10` | Maximum request body size in MB |

### `[upstream]`

| Field | Default | Description |
|-------|---------|-------------|
| `base_url` | — | Upstream API base URL (e.g. `https://api.openai.com/v1`) |
| `api_key` | — | Upstream API key (can also use env var `LLM_SHADOW_RELAY_UPSTREAM_API_KEY`) |
| `default_model` | — | Default model when not specified in request |
| `protocol` | `"openai"` | Upstream protocol: `"openai"` or `"anthropic"` |
| `extra_headers` | `{}` | Additional HTTP headers sent to upstream |
| `timeout` | — | Upstream timeout in seconds (defaults to `server.timeout`) |

### `[audit]`

| Field | Default | Description |
|-------|---------|-------------|
| `provider` | — | Audit model provider: `"openai"`, `"anthropic"`, `"ollama"`, `"local"` |
| `base_url` | — | Audit API base URL (for non-OpenAI providers) |
| `api_key` | — | Audit API key (env var: `LLM_SHADOW_RELAY_AUDIT_API_KEY`) |
| `model` | — | Audit model name (e.g. `gpt-4o-mini`, `claude-3-haiku`, `qwen2.5-8b`) |
| `temperature` | `0.1` | Lower = more consistent audit decisions |
| `max_tokens` | `2048` | Max tokens for audit response |
| `enabled` | `true` | Enable/disable audit |
| `stream_audit` | `true` | Audit streaming chunks in real-time |
| `batch_size` | `10` | Chunks to accumulate before streaming audit |
| `mode` | `"sync"` | `"sync"` (blocking) or `"async"` (background) |
| `system_prompt` | — | Custom audit system prompt |

### `[policy]`

| Field | Default | Description |
|-------|---------|-------------|
| `block_risk_levels` | `["critical", "high"]` | Risk levels that block the request |
| `block_prompt_injection` | `true` | Block on prompt injection detection |
| `block_system_prompt_extraction` | `true` | Block on system prompt extraction |
| `block_tool_call` | `true` | Block on tool call injection |
| `log_all` | `true` | Log all audit decisions |
| `custom_keywords` | `[]` | Extra keywords to flag |

## Audit Logic

1. **Pre-request audit**: User's prompt is checked for injection patterns **before** forwarding to upstream
2. **Response audit**: The upstream response is analyzed by the audit model for security risks
3. **Policy enforcement**: Audit decisions are evaluated against configurable policy rules
4. **Sync mode**: Client waits for full audit before receiving response
5. **Async mode**: Response is returned immediately; audit runs in background (logged only)

## Protocol Conversion

The proxy uses OpenAI-style `ChatCompletionRequest`/`ChatCompletionResponse` as its **internal canonical format**:

| Client Sends | Upstream Protocol | What Happens |
|-------------|-------------------|--------------|
| OpenAI | OpenAI | Direct passthrough |
| OpenAI | Anthropic | Convert → send to `/messages` → convert response back |
| Anthropic | OpenAI | Convert to canonical → send → convert to Anthropic |
| Anthropic | Anthropic | Convert → send → convert back (round-trip) |

This ensures the audit engine always works with the same canonical format regardless of the client or upstream protocol.

## Security Considerations

- The **audit model** should be a **trusted** model — this is your security checkpoint
- Recommended audit models: `gpt-4o-mini`, `claude-3-haiku`, `deepseek-chat`, `qwen2.5-8b`
- The upstream model is treated as **untrusted** — that's why we audit it
- API keys can be set via environment variables to avoid committing secrets:
  - `LLM_SHADOW_RELAY_UPSTREAM_API_KEY`
  - `LLM_SHADOW_RELAY_AUDIT_API_KEY`
- In `async` mode, audit failures are logged but not returned to the client — monitor your logs!

## License

MIT
