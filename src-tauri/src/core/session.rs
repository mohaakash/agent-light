use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::DisplayState;

pub const COMPLETION_DISPLAY_DURATION: Duration = Duration::from_secs(5);
pub const STALE_SESSION_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_RECENT_EVENT_IDS: usize = 2_048;
const MAX_SESSIONS: usize = 256;
const MAX_FUTURE_CLOCK_SKEW: chrono::Duration = chrono::Duration::minutes(5);

#[derive(Clone, Debug, Serialize)]
pub struct SessionSnapshot {
    pub agent: String,
    pub session_id: String,
    pub project: Option<String>,
    pub state: DisplayState,
    pub seconds_since_event: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    SessionStarted,
    AgentWorking,
    PermissionRequired,
    UserInputRequired,
    TaskCompleted,
    SessionIdle,
    SessionError,
    SessionEnded,
    Heartbeat,
}

#[derive(Clone, Debug)]
pub struct SessionEvent {
    pub event_id: String,
    pub agent: String,
    pub session_id: String,
    pub project: Option<String>,
    pub kind: EventKind,
    pub occurred_at: Option<String>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct SessionKey {
    agent: String,
    session_id: String,
}

#[derive(Debug)]
struct Session {
    project: Option<String>,
    state: DisplayState,
    last_event_at: Instant,
    source_timestamp: Option<DateTime<Utc>>,
    completed_at: Option<Instant>,
}

#[derive(Debug)]
pub struct SessionManager {
    sessions: HashMap<SessionKey, Session>,
    recent_event_ids: HashSet<String>,
    event_id_order: VecDeque<String>,
    last_resolved_state: DisplayState,
    status_changed: bool,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            recent_event_ids: HashSet::new(),
            event_id_order: VecDeque::new(),
            last_resolved_state: DisplayState::Idle,
            status_changed: false,
        }
    }
}

impl SessionManager {
    pub fn state_for_agent(&self, agent: &str) -> DisplayState {
        self.sessions
            .iter()
            .filter(|(key, _)| key.agent.eq_ignore_ascii_case(agent))
            .map(|(_, session)| session.state)
            .max_by_key(|state| priority(*state))
            .unwrap_or(DisplayState::Idle)
    }

    pub fn snapshots(&self, now: Instant) -> Vec<SessionSnapshot> {
        let mut snapshots: Vec<_> = self
            .sessions
            .iter()
            .map(|(key, session)| SessionSnapshot {
                agent: key.agent.clone(),
                session_id: key.session_id.clone(),
                project: session.project.clone(),
                state: session.state,
                seconds_since_event: now
                    .saturating_duration_since(session.last_event_at)
                    .as_secs(),
            })
            .collect();
        snapshots.sort_by(|left, right| {
            left.agent
                .cmp(&right.agent)
                .then_with(|| left.project.cmp(&right.project))
                .then_with(|| left.session_id.cmp(&right.session_id))
        });
        snapshots
    }

    /// Applies an event. Returns false for a duplicate or timestamp-ordered stale event.
    pub fn apply(&mut self, event: SessionEvent, received_at: Instant) -> Result<bool, String> {
        if self.recent_event_ids.contains(&event.event_id) {
            return Ok(false);
        }

        let parsed_timestamp = event
            .occurred_at
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .filter(|value| *value <= Utc::now() + MAX_FUTURE_CLOCK_SKEW);
        let key = SessionKey {
            agent: event.agent.clone(),
            session_id: event.session_id.clone(),
        };

        if let Some(session) = self.sessions.get(&key) {
            if parsed_timestamp
                .zip(session.source_timestamp)
                .is_some_and(|(incoming, current)| incoming <= current)
            {
                self.remember_event_id(event.event_id);
                return Ok(false);
            }
        } else if self.sessions.len() >= MAX_SESSIONS {
            self.evict_inactive_session();
            if self.sessions.len() >= MAX_SESSIONS {
                return Err("session capacity reached".to_string());
            }
        }

        let state = state_for_event(event.kind);
        let session = self.sessions.entry(key).or_insert_with(|| Session {
            project: None,
            state: DisplayState::Idle,
            last_event_at: received_at,
            source_timestamp: None,
            completed_at: None,
        });
        if event.project.is_some() {
            session.project = event.project;
        }
        session.state = state;
        session.last_event_at = received_at;
        if let Some(timestamp) = parsed_timestamp {
            session.source_timestamp = Some(timestamp);
        }
        session.completed_at = (state == DisplayState::Completed).then_some(received_at);

        self.remember_event_id(event.event_id);
        Ok(true)
    }

    pub fn tick(&mut self, now: Instant) -> DisplayState {
        for session in self.sessions.values_mut() {
            if session.state == DisplayState::Completed
                && session.completed_at.is_some_and(|completed_at| {
                    now.duration_since(completed_at) >= COMPLETION_DISPLAY_DURATION
                })
            {
                session.state = DisplayState::Idle;
                session.completed_at = None;
            } else if session.state == DisplayState::Working
                && now.duration_since(session.last_event_at) >= STALE_SESSION_TIMEOUT
            {
                // Stale work becomes inactive; it is never shown as completed.
                session.state = DisplayState::Idle;
            }
        }

        let resolved = self
            .sessions
            .values()
            .map(|session| session.state)
            .max_by_key(|state| priority(*state))
            .unwrap_or(DisplayState::Idle);
        self.status_changed = resolved != self.last_resolved_state;
        self.last_resolved_state = resolved;
        resolved
    }

    pub fn take_status_changed(&mut self) -> bool {
        std::mem::take(&mut self.status_changed)
    }

    fn remember_event_id(&mut self, event_id: String) {
        self.recent_event_ids.insert(event_id.clone());
        self.event_id_order.push_back(event_id);
        while self.event_id_order.len() > MAX_RECENT_EVENT_IDS {
            if let Some(expired) = self.event_id_order.pop_front() {
                self.recent_event_ids.remove(&expired);
            }
        }
    }

    fn evict_inactive_session(&mut self) {
        let candidate = self
            .sessions
            .iter()
            .filter(|(_, session)| session.state == DisplayState::Idle)
            .min_by_key(|(_, session)| session.last_event_at)
            .map(|(key, _)| key.clone());
        if let Some(key) = candidate {
            self.sessions.remove(&key);
        }
    }
}

fn state_for_event(event: EventKind) -> DisplayState {
    match event {
        EventKind::SessionStarted | EventKind::SessionIdle | EventKind::SessionEnded => {
            DisplayState::Idle
        }
        EventKind::AgentWorking | EventKind::Heartbeat => DisplayState::Working,
        EventKind::PermissionRequired | EventKind::UserInputRequired => DisplayState::Waiting,
        EventKind::TaskCompleted => DisplayState::Completed,
        EventKind::SessionError => DisplayState::Error,
    }
}

fn priority(state: DisplayState) -> u8 {
    match state {
        DisplayState::Waiting => 5,
        DisplayState::Error => 4,
        DisplayState::Working => 3,
        DisplayState::Completed => 2,
        DisplayState::Idle => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(
        id: &str,
        session_id: &str,
        kind: EventKind,
        occurred_at: Option<&str>,
    ) -> SessionEvent {
        SessionEvent {
            event_id: id.to_string(),
            agent: "codex".to_string(),
            session_id: session_id.to_string(),
            project: None,
            kind,
            occurred_at: occurred_at.map(str::to_string),
        }
    }

    #[test]
    fn waiting_has_priority_over_working_and_error() {
        let now = Instant::now();
        let mut sessions = SessionManager::default();
        sessions
            .apply(event("1", "work", EventKind::AgentWorking, None), now)
            .unwrap();
        sessions
            .apply(event("2", "error", EventKind::SessionError, None), now)
            .unwrap();
        sessions
            .apply(event("3", "wait", EventKind::PermissionRequired, None), now)
            .unwrap();
        assert_eq!(sessions.tick(now), DisplayState::Waiting);
    }

    #[test]
    fn error_has_priority_over_working_and_completion() {
        let now = Instant::now();
        let mut sessions = SessionManager::default();
        sessions
            .apply(event("work", "work", EventKind::AgentWorking, None), now)
            .unwrap();
        sessions
            .apply(event("done", "done", EventKind::TaskCompleted, None), now)
            .unwrap();
        sessions
            .apply(event("error", "error", EventKind::SessionError, None), now)
            .unwrap();
        assert_eq!(sessions.tick(now), DisplayState::Error);
    }

    #[test]
    fn working_has_priority_over_completion() {
        let now = Instant::now();
        let mut sessions = SessionManager::default();
        sessions
            .apply(event("done", "done", EventKind::TaskCompleted, None), now)
            .unwrap();
        sessions
            .apply(event("work", "work", EventKind::AgentWorking, None), now)
            .unwrap();
        assert_eq!(sessions.tick(now), DisplayState::Working);
    }

    #[test]
    fn completion_expires_to_idle_after_five_seconds() {
        let now = Instant::now();
        let mut sessions = SessionManager::default();
        sessions
            .apply(event("1", "one", EventKind::TaskCompleted, None), now)
            .unwrap();
        assert_eq!(
            sessions.tick(now + Duration::from_secs(4)),
            DisplayState::Completed
        );
        assert_eq!(
            sessions.tick(now + COMPLETION_DISPLAY_DURATION),
            DisplayState::Idle
        );
    }

    #[test]
    fn stale_work_expires_without_becoming_completed() {
        let now = Instant::now();
        let mut sessions = SessionManager::default();
        sessions
            .apply(event("1", "one", EventKind::AgentWorking, None), now)
            .unwrap();
        assert_eq!(
            sessions.tick(now + STALE_SESSION_TIMEOUT),
            DisplayState::Idle
        );
    }

    #[test]
    fn duplicate_event_ids_are_ignored() {
        let now = Instant::now();
        let mut sessions = SessionManager::default();
        assert!(sessions
            .apply(event("same", "one", EventKind::AgentWorking, None), now)
            .unwrap());
        assert!(!sessions
            .apply(
                event("same", "one", EventKind::PermissionRequired, None),
                now
            )
            .unwrap());
        assert_eq!(sessions.tick(now), DisplayState::Working);
    }

    #[test]
    fn older_timestamp_cannot_overwrite_newer_state() {
        let now = Instant::now();
        let mut sessions = SessionManager::default();
        sessions
            .apply(
                event(
                    "new",
                    "one",
                    EventKind::PermissionRequired,
                    Some("2026-01-01T12:01:00Z"),
                ),
                now,
            )
            .unwrap();
        assert!(!sessions
            .apply(
                event(
                    "old",
                    "one",
                    EventKind::AgentWorking,
                    Some("2026-01-01T12:00:00Z")
                ),
                now
            )
            .unwrap());
        assert_eq!(sessions.tick(now), DisplayState::Waiting);
    }

    #[test]
    fn permission_request_remains_until_resolved_by_a_newer_event() {
        let now = Instant::now();
        let mut sessions = SessionManager::default();
        sessions
            .apply(
                event("wait", "one", EventKind::PermissionRequired, None),
                now,
            )
            .unwrap();
        assert_eq!(
            sessions.tick(now + Duration::from_secs(600)),
            DisplayState::Waiting
        );
        sessions
            .apply(
                event("end", "one", EventKind::SessionEnded, None),
                now + Duration::from_secs(601),
            )
            .unwrap();
        assert_eq!(
            sessions.tick(now + Duration::from_secs(601)),
            DisplayState::Idle
        );
    }
}
