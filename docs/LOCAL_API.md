# Local API Specification

## 1. Purpose

The local API allows agent integrations written in different languages to send normalized events to the running AgentLight desktop application.

Initial transport: HTTP over loopback.

## 2. Binding and authentication

- Bind only to `127.0.0.1`.
- Do not bind to `0.0.0.0` or a LAN interface.
- Generate a random per-installation token.
- Require the token for event submission.
- Do not expose unauthenticated state-changing endpoints.
- Reject oversized or malformed requests.
- Do not enable permissive browser CORS by default.
- Avoid logging authentication tokens.

The exact token delivery mechanism to adapters must be designed before implementation. Prefer a protected local configuration file or OS credential storage over command-line arguments.

## 3. Initial endpoint

`POST /api/v1/events`

Headers:
```http
Content-Type: application/json
Authorization: Bearer <local-token>
```

Body:
```json
{
  "schema_version": 1,
  "event_id": "evt-123",
  "agent": "claude",
  "session_id": "session-456",
  "project": "website",
  "event": "permission_required",
  "occurred_at": "2026-01-01T12:00:00Z"
}
```

Success response:
```json
{
  "accepted": true,
  "event_id": "evt-123"
}
```

Error response:
```json
{
  "accepted": false,
  "error": "invalid_event"
}
```

## 4. Health endpoint

`GET /api/v1/health` returns a minimal `{"status":"ok"}` response. It is unauthenticated because it exposes no application or session data and the server binds only to loopback. It does not change application state.

## 5. Additional endpoints (future)

- `GET /api/v1/health` — local health check; expose minimal information.
- `GET /api/v1/status` — current aggregate state, authenticated.
- `GET /api/v1/sessions` — active sessions, authenticated.
- `POST /api/v1/sessions/{id}/dismiss` — only if a deliberate dismissal UX is implemented; must not represent agent approval.

## 6. Validation

Validate:
- Schema version.
- Required fields and maximum lengths.
- Allowed agent identifier format.
- Allowed event names.
- Session identifier.
- Timestamp parsing and reasonable bounds.
- Request body size.
- Authentication token.

Unknown event types should be rejected with a clear error or handled through an explicitly versioned extension mechanism.

## 7. Versioning

Use `/api/v1` and `schema_version: 1`. Breaking changes require a new API/schema version or a documented compatibility strategy.

## 8. Privacy

The API is for status metadata only. Do not accept full prompts, source files, terminal output, tool call arguments or credentials.

## 9. Current implementation

The server exposes `POST /api/v1/events` and `GET /api/v1/health` on `127.0.0.1:38527`. AgentLight creates a per-installation bearer token and writes the endpoint and token to its app configuration as `codex-bridge.json`; the Codex hook reads that file. Event submissions require the token, `application/json`, schema version `1`, and the `codex` agent. The payload is size-limited and routed through the session resolver before state is broadcast to both native windows.

Requests produce one-line JSON logs with timestamp, level, route and HTTP status. Unknown request paths are logged as `unknown`; authorization values and event bodies are never logged. API tests cover health, authentication, content type, malformed payloads, core rejection, unknown routes, response framing, oversized content lengths, and a real loopback TCP request through the HTTP reader and writer. A GUI-driven test against a running Tauri instance remains separate.
