# Status Model and Event Semantics

## 1. Purpose

This document defines the canonical language used between adapters, the Rust core, the virtual widget and the hardware output.

Agent-specific events must be normalized before they reach the status resolver.

## 2. Canonical states

| State | Meaning | Light |
|---|---|---|
| `WAITING` | Human attention is required, usually permission/input | Red |
| `WORKING` | Agent is actively executing or processing | Yellow |
| `COMPLETED` | A tracked session/task completed successfully | Green |
| `IDLE` | No active work or pending attention | All off |
| `ERROR` | Session or integration failed | Red |

Do not equate `COMPLETED` with `IDLE`. Completion is a meaningful transient/result state; idle means no active session requiring display.

## 3. Canonical event envelope

Example:

```json
{
  "schema_version": 1,
  "event_id": "unique-event-id",
  "agent": "codex",
  "session_id": "agent-session-id",
  "project": "optional-project-label",
  "event": "permission_required",
  "occurred_at": "2026-01-01T12:00:00Z"
}
```

Fields:
- `schema_version`: required integer.
- `event_id`: unique identifier for deduplication.
- `agent`: stable adapter identifier.
- `session_id`: stable identifier within the agent.
- `project`: optional, non-sensitive display label.
- `event`: canonical event name.
- `occurred_at`: optional RFC 3339 timestamp.

Do not include prompts, source code, tool arguments, terminal output or environment secrets.

## 4. Canonical event names

Initial vocabulary:
- `session_started`
- `agent_working`
- `permission_required`
- `user_input_required`
- `task_completed`
- `session_idle`
- `session_error`
- `session_ended`
- `heartbeat`

Adapters may have richer source events, but must map them to this vocabulary.

## 5. Session state

Each session should track:
- Agent identifier.
- Session identifier.
- Optional project label.
- Current canonical state.
- Last event timestamp.
- Last heartbeat timestamp, if supported.
- Whether the session is considered stale.
- Optional short error category (not raw sensitive output).

## 6. Aggregate state resolution

The overall traffic light represents the highest-priority active condition across sessions.

Initial priority:

1. `WAITING`
2. `ERROR`
3. `WORKING`
4. `COMPLETED`
5. `IDLE`

Rationale: a pending human decision should not be hidden by another session working. An error should remain visible, but should not mask a permission request.

Completion behavior:
- `COMPLETED` should be visible for a configurable short duration, initially 5 seconds.
- After that, a completed session transitions to `IDLE` unless a new event arrives.
- The dashboard can retain session history separately.

This duration must be configurable later and tested with multiple sessions.

## 7. Stale sessions

- A session must not remain `WORKING` forever if the adapter stops reporting.
- Use configurable stale timeout, initially 2 minutes as a provisional default.
- On stale timeout, mark the session stale and exclude it from active aggregate status, or represent it as `IDLE` with a stale marker in the dashboard.
- Do not silently turn a stale session into `COMPLETED`.
- A permission request should remain active until a corresponding resolution event, session end, or explicit user dismissal policy. Never infer permission approval.

## 8. Event ordering and deduplication

- Ignore duplicate `event_id` values.
- Prefer source timestamps when valid, but guard against clock skew.
- A delayed event must not overwrite a newer state without a defined reconciliation rule.
- Keep a bounded in-memory event/session history.
- Document behavior for adapters that cannot provide stable session IDs.

## 9. Manual override

Manual test/override is for checking outputs, not for changing the authoritative agent session state. Clearly indicate when manual test mode is active and restore the resolved state afterward.

The Rust core tracks sessions independently and resolves their aggregate state using the priority above. Development controls publish a `manual_preview` snapshot without changing authoritative session state. A subsequent accepted agent event restores the resolved Codex state; periodic expiry checks publish only when the aggregate changes. Completion duration begins when the resolver receives a completion event.

Current implementation bounds the in-memory event ID history to 2,048 entries and tracks at most 256 sessions. Session identity is the `(agent, session_id)` pair. Valid source timestamps are parsed as RFC 3339; events at or before the last valid timestamp for that session are ignored. Unparseable or more-than-five-minutes-future timestamps fall back to receive order. `WORKING` sessions become `IDLE` after two minutes without an event. `WAITING` and `ERROR` remain visible until a later event resolves them.
