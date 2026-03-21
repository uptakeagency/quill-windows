use serde::{Deserialize, Serialize};

use super::mode::AnalysisMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    pub mode: AnalysisMode,
    pub original: String,
    pub corrected: String,
    #[serde(default)]
    pub changes: Vec<TextChange>,
    pub explanation: Option<String>,
    pub tldr: Option<String>,
    pub resources: Option<Vec<ResourceLink>>,
    pub alternatives: Option<Vec<Alternative>>,
    #[serde(default)]
    pub vocabulary: Vec<VocabularyCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChange {
    pub original: String,
    pub replacement: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLink {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabularyCard {
    pub word: String,
    pub suggestion: String,
    pub definition: String,
    pub example: String,
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub name: String,
    pub description: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToneStyle {
    Formal,
    Casual,
    Professional,
    Friendly,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::mode::AnalysisMode;

    #[test]
    fn analysis_result_serializes_correctly() {
        let result = AnalysisResult {
            mode: AnalysisMode::Improve,
            original: "Hello wrold".to_string(),
            corrected: "Hello world".to_string(),
            changes: vec![TextChange {
                original: "wrold".to_string(),
                replacement: "world".to_string(),
                reason: "Typo fix".to_string(),
            }],
            explanation: Some("Fixed a typo".to_string()),
            tldr: None,
            resources: None,
            alternatives: None,
            vocabulary: vec![],
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["mode"], "improve");
        assert_eq!(json["original"], "Hello wrold");
        assert_eq!(json["corrected"], "Hello world");
        assert_eq!(json["changes"][0]["replacement"], "world");
        assert_eq!(json["explanation"], "Fixed a typo");
        assert!(json["tldr"].is_null());
        assert!(json["resources"].is_null());
        assert!(json["alternatives"].is_null());
        assert_eq!(json["vocabulary"], serde_json::json!([]));
    }

    #[test]
    fn analysis_result_deserializes_with_missing_optional_fields() {
        let json = r#"{
            "mode": "techExplain",
            "original": "async/await",
            "corrected": "async/await",
            "changes": []
        }"#;

        let result: AnalysisResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.mode, AnalysisMode::TechExplain);
        assert_eq!(result.original, "async/await");
        assert!(result.explanation.is_none());
        assert!(result.tldr.is_none());
        assert!(result.resources.is_none());
        assert!(result.alternatives.is_none());
        assert!(result.vocabulary.is_empty());
    }

    #[test]
    fn text_change_round_trip() {
        let change = TextChange {
            original: "teh".to_string(),
            replacement: "the".to_string(),
            reason: "Typo".to_string(),
        };

        let json = serde_json::to_string(&change).unwrap();
        let deserialized: TextChange = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.original, "teh");
        assert_eq!(deserialized.replacement, "the");
        assert_eq!(deserialized.reason, "Typo");
    }

    #[test]
    fn tone_style_serializes_camel_case() {
        assert_eq!(
            serde_json::to_string(&ToneStyle::Formal).unwrap(),
            "\"formal\""
        );
        assert_eq!(
            serde_json::to_string(&ToneStyle::Casual).unwrap(),
            "\"casual\""
        );
        assert_eq!(
            serde_json::to_string(&ToneStyle::Professional).unwrap(),
            "\"professional\""
        );
        assert_eq!(
            serde_json::to_string(&ToneStyle::Friendly).unwrap(),
            "\"friendly\""
        );
    }

    #[test]
    fn resource_link_round_trip() {
        let link = ResourceLink {
            title: "Rust Book".to_string(),
            url: "https://doc.rust-lang.org/book/".to_string(),
        };
        let json = serde_json::to_string(&link).unwrap();
        let deserialized: ResourceLink = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "Rust Book");
        assert_eq!(deserialized.url, "https://doc.rust-lang.org/book/");
    }

    #[test]
    fn vocabulary_card_round_trip() {
        let card = VocabularyCard {
            word: "big".to_string(),
            suggestion: "substantial".to_string(),
            definition: "Of considerable size".to_string(),
            example: "A substantial improvement".to_string(),
            level: "B2".to_string(),
        };
        let json = serde_json::to_string(&card).unwrap();
        let deserialized: VocabularyCard = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.level, "B2");
        assert_eq!(deserialized.suggestion, "substantial");
    }

    #[test]
    fn alternative_with_vec_pros_cons() {
        let alt = Alternative {
            name: "Tokio".to_string(),
            description: "Async runtime for Rust".to_string(),
            pros: vec!["Fast".to_string(), "Mature".to_string()],
            cons: vec!["Complex".to_string()],
            url: Some("https://tokio.rs".to_string()),
        };
        let json = serde_json::to_value(&alt).unwrap();
        assert_eq!(json["pros"], serde_json::json!(["Fast", "Mature"]));
        assert_eq!(json["cons"], serde_json::json!(["Complex"]));
        assert_eq!(json["url"], "https://tokio.rs");
    }

    #[test]
    fn alternative_without_url_deserializes() {
        let json = r#"{"name":"X","description":"Y","pros":[],"cons":[]}"#;
        let alt: Alternative = serde_json::from_str(json).unwrap();
        assert!(alt.url.is_none());
    }
}
