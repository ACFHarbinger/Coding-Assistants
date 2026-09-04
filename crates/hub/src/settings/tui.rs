use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuiSettings {
    pub prefix_chord: String,
    pub unicode_fallback: bool,
    pub bell_notification: bool,
    pub high_contrast: bool,
}

impl Default for TuiSettings {
    fn default() -> Self {
        Self {
            prefix_chord: "ctrl+b".to_string(),
            unicode_fallback: false,
            bell_notification: true,
            high_contrast: true,
        }
    }
}
