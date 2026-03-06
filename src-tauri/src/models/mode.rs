use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisMode {
    Improve,
    Translate,
    TechExplain,
}

impl Default for AnalysisMode {
    fn default() -> Self {
        Self::TechExplain
    }
}

impl AnalysisMode {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Improve => "Improve",
            Self::Translate => "Translate",
            Self::TechExplain => "Tech",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Improve => "Fix grammar, improve clarity, vocabulary",
            Self::Translate => "Translate between languages",
            Self::TechExplain => "Explain technical/programming terms",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_tech_explain() {
        let mode = AnalysisMode::default();
        assert_eq!(mode, AnalysisMode::TechExplain);
    }

    #[test]
    fn serializes_to_camel_case() {
        assert_eq!(
            serde_json::to_string(&AnalysisMode::TechExplain).unwrap(),
            "\"techExplain\""
        );
        assert_eq!(
            serde_json::to_string(&AnalysisMode::Improve).unwrap(),
            "\"improve\""
        );
        assert_eq!(
            serde_json::to_string(&AnalysisMode::Translate).unwrap(),
            "\"translate\""
        );
    }

    #[test]
    fn deserializes_from_camel_case() {
        let mode: AnalysisMode = serde_json::from_str("\"techExplain\"").unwrap();
        assert_eq!(mode, AnalysisMode::TechExplain);

        let mode: AnalysisMode = serde_json::from_str("\"improve\"").unwrap();
        assert_eq!(mode, AnalysisMode::Improve);

        let mode: AnalysisMode = serde_json::from_str("\"translate\"").unwrap();
        assert_eq!(mode, AnalysisMode::Translate);
    }

    #[test]
    fn title_returns_correct_string() {
        assert_eq!(AnalysisMode::Improve.title(), "Improve");
        assert_eq!(AnalysisMode::Translate.title(), "Translate");
        assert_eq!(AnalysisMode::TechExplain.title(), "Tech");
    }

    #[test]
    fn description_returns_correct_string() {
        assert_eq!(
            AnalysisMode::Improve.description(),
            "Fix grammar, improve clarity, vocabulary"
        );
        assert_eq!(
            AnalysisMode::Translate.description(),
            "Translate between languages"
        );
        assert_eq!(
            AnalysisMode::TechExplain.description(),
            "Explain technical/programming terms"
        );
    }
}
