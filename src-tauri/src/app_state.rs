use std::sync::Mutex;

use crate::models::{AnalysisMode, ExplanationLevel};
use crate::models::analysis::ToneStyle;
use crate::models::explanation::TechDictionaryState;

/// App-wide shared state managed by Tauri.
///
/// Each field is wrapped in a `Mutex` for interior mutability
/// since Tauri's managed state requires `Send + Sync`.
pub struct AppState {
    pub mode: Mutex<AnalysisMode>,
    pub level: Mutex<ExplanationLevel>,
    pub tone: Mutex<Option<ToneStyle>>,
    pub native_language: Mutex<String>,
    pub target_language: Mutex<String>,
    pub tech_state: Mutex<TechDictionaryState>,
    pub ai_provider: Mutex<String>,
    pub gemini_model: Mutex<String>,
    pub claude_model: Mutex<String>,
    pub hotkey: Mutex<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: Mutex::new(AnalysisMode::default()),
            level: Mutex::new(ExplanationLevel::default()),
            tone: Mutex::new(None),
            native_language: Mutex::new("Turkish".to_string()),
            target_language: Mutex::new("English".to_string()),
            tech_state: Mutex::new(TechDictionaryState::new()),
            ai_provider: Mutex::new("gemini".to_string()),
            gemini_model: Mutex::new("gemini-2.5-flash".to_string()),
            claude_model: Mutex::new("claude-sonnet-4-20250514".to_string()),
            hotkey: Mutex::new("Ctrl+Shift+Q".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_tech_explain() {
        let state = AppState::default();
        assert_eq!(*state.mode.lock().unwrap(), AnalysisMode::TechExplain);
    }

    #[test]
    fn default_level_is_eli15() {
        let state = AppState::default();
        assert_eq!(*state.level.lock().unwrap(), ExplanationLevel::Eli15);
    }

    #[test]
    fn default_tone_is_none() {
        let state = AppState::default();
        assert!(state.tone.lock().unwrap().is_none());
    }

    #[test]
    fn default_native_language_is_turkish() {
        let state = AppState::default();
        assert_eq!(*state.native_language.lock().unwrap(), "Turkish");
    }

    #[test]
    fn default_target_language_is_english() {
        let state = AppState::default();
        assert_eq!(*state.target_language.lock().unwrap(), "English");
    }

    #[test]
    fn default_ai_provider_is_gemini() {
        let state = AppState::default();
        assert_eq!(*state.ai_provider.lock().unwrap(), "gemini");
    }

    #[test]
    fn default_gemini_model() {
        let state = AppState::default();
        assert_eq!(*state.gemini_model.lock().unwrap(), "gemini-2.5-flash");
    }

    #[test]
    fn default_claude_model() {
        let state = AppState::default();
        assert_eq!(
            *state.claude_model.lock().unwrap(),
            "claude-sonnet-4-20250514"
        );
    }

    #[test]
    fn mode_can_be_changed() {
        let state = AppState::default();
        *state.mode.lock().unwrap() = AnalysisMode::Improve;
        assert_eq!(*state.mode.lock().unwrap(), AnalysisMode::Improve);
    }

    #[test]
    fn level_can_be_changed() {
        let state = AppState::default();
        *state.level.lock().unwrap() = ExplanationLevel::Professional;
        assert_eq!(*state.level.lock().unwrap(), ExplanationLevel::Professional);
    }

    #[test]
    fn tone_can_be_set() {
        let state = AppState::default();
        *state.tone.lock().unwrap() = Some(ToneStyle::Formal);
        assert_eq!(*state.tone.lock().unwrap(), Some(ToneStyle::Formal));
    }

    #[test]
    fn ai_provider_can_be_switched() {
        let state = AppState::default();
        *state.ai_provider.lock().unwrap() = "claude".to_string();
        assert_eq!(*state.ai_provider.lock().unwrap(), "claude");
    }

    #[test]
    fn default_hotkey() {
        let state = AppState::default();
        assert_eq!(*state.hotkey.lock().unwrap(), "Ctrl+Shift+Q");
    }

    #[test]
    fn hotkey_can_be_changed() {
        let state = AppState::default();
        *state.hotkey.lock().unwrap() = "Ctrl+Alt+Space".to_string();
        assert_eq!(*state.hotkey.lock().unwrap(), "Ctrl+Alt+Space");
    }

    #[test]
    fn tech_state_starts_empty() {
        let state = AppState::default();
        let ts = state.tech_state.lock().unwrap();
        assert!(ts.current().is_none());
        assert_eq!(ts.cache_count(), 0);
    }
}
