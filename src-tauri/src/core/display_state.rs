use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DisplayState {
    Waiting,
    Working,
    Completed,
    Idle,
    Error,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DisplaySource {
    ManualPreview,
    Codex,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DisplaySnapshot {
    pub state: DisplayState,
    pub source: DisplaySource,
}

impl DisplaySnapshot {
    pub fn manual(state: DisplayState) -> Self {
        Self {
            state,
            source: DisplaySource::ManualPreview,
        }
    }

    pub fn codex(state: DisplayState) -> Self {
        Self {
            state,
            source: DisplaySource::Codex,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DisplaySnapshot, DisplaySource, DisplayState};

    #[test]
    fn serializes_wire_states_in_uppercase() {
        let json = serde_json::to_string(&DisplaySnapshot::manual(DisplayState::Working)).unwrap();
        assert_eq!(json, r#"{"state":"WORKING","source":"manual_preview"}"#);
    }

    #[test]
    fn all_prototype_states_are_distinct() {
        let states = [
            DisplayState::Waiting,
            DisplayState::Working,
            DisplayState::Completed,
            DisplayState::Idle,
        ];
        assert_eq!(states.len(), 4);
        assert!(states.iter().all(|state| *state != DisplayState::Error));
        assert_eq!(
            DisplaySnapshot::manual(DisplayState::Idle).source,
            DisplaySource::ManualPreview
        );
        assert_eq!(
            DisplaySnapshot::codex(DisplayState::Working).source,
            DisplaySource::Codex
        );
    }
}
