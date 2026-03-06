use serde_json::{json, Value};

use crate::models::{AnalysisMode, AnalysisResult, ExplanationLevel};
use crate::models::analysis::ToneStyle;

use super::parser;
use super::prompts;

/// Default Gemini model.
pub const DEFAULT_MODEL: &str = "gemini-2.5-flash";

/// Request timeout in seconds.
const TIMEOUT_SECS: u64 = 30;

/// Build the Gemini API endpoint URL for a given model.
fn build_url(model: &str) -> String {
    format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    )
}

/// Build the JSON request body for the Gemini generateContent API.
///
/// This is a pure function, easily testable without HTTP.
fn build_request_body(system_prompt: &str, user_prompt: &str) -> Value {
    json!({
        "system_instruction": {
            "parts": [{"text": system_prompt}]
        },
        "contents": [
            {"parts": [{"text": user_prompt}]}
        ],
        "generationConfig": {
            "temperature": 0.3,
            "maxOutputTokens": 2048
        }
    })
}

/// Extract the text content from a Gemini API response JSON string.
///
/// Navigates: `candidates[0].content.parts[0].text`
fn extract_text_from_response(response_body: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(response_body)
        .map_err(|e| format!("Failed to parse Gemini response JSON: {}", e))?;

    let text = value
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| {
            format!(
                "Unexpected Gemini response structure: {}",
                &response_body[..response_body.len().min(500)]
            )
        })?;

    Ok(text.to_string())
}

/// Analyze text using the Google Gemini API.
///
/// 1. Builds system and user prompts from mode/level/options
/// 2. Calls the Gemini generateContent endpoint via HTTP
/// 3. Parses the AI response with the multi-layer parser
pub async fn analyze(
    api_key: &str,
    model: &str,
    text: &str,
    mode: AnalysisMode,
    tone: Option<&ToneStyle>,
    context: Option<&str>,
    native_language: Option<&str>,
    target_language: Option<&str>,
    level: Option<ExplanationLevel>,
) -> Result<AnalysisResult, String> {
    let system = prompts::system_prompt(mode, level);
    let user = prompts::user_prompt(mode, text, tone, context, native_language, target_language);

    let url = build_url(model);
    let body = build_request_body(&system, &user);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Gemini API request failed: {}", e))?;

    let status = response.status();
    let response_body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Gemini response body: {}", e))?;

    if !status.is_success() {
        return Err(format!("Gemini API {}: {}", status.as_u16(), response_body));
    }

    let ai_text = extract_text_from_response(&response_body)?;

    Ok(parser::parse_response(&ai_text, mode, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // build_url tests
    // =========================================================================

    #[test]
    fn build_url_with_default_model() {
        let url = build_url(DEFAULT_MODEL);
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn build_url_with_custom_model() {
        let url = build_url("gemini-2.5-pro");
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent"
        );
    }

    // =========================================================================
    // build_request_body tests
    // =========================================================================

    #[test]
    fn build_request_body_has_system_instruction() {
        let body = build_request_body("You are an expert.", "Improve this text");

        let system_text = body["system_instruction"]["parts"][0]["text"]
            .as_str()
            .unwrap();
        assert_eq!(system_text, "You are an expert.");
    }

    #[test]
    fn build_request_body_has_contents() {
        let body = build_request_body("System prompt", "User prompt here");

        let user_text = body["contents"][0]["parts"][0]["text"]
            .as_str()
            .unwrap();
        assert_eq!(user_text, "User prompt here");
    }

    #[test]
    fn build_request_body_has_generation_config() {
        let body = build_request_body("sys", "user");

        let config = &body["generationConfig"];
        assert_eq!(config["temperature"].as_f64().unwrap(), 0.3);
        assert_eq!(config["maxOutputTokens"].as_u64().unwrap(), 2048);
    }

    #[test]
    fn build_request_body_structure_is_complete() {
        let body = build_request_body("sys", "user");

        // Verify all top-level keys exist
        assert!(body.get("system_instruction").is_some());
        assert!(body.get("contents").is_some());
        assert!(body.get("generationConfig").is_some());

        // Verify system_instruction.parts is an array with one element
        let parts = body["system_instruction"]["parts"].as_array().unwrap();
        assert_eq!(parts.len(), 1);

        // Verify contents is an array with one element
        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);

        // Verify contents[0].parts is an array with one element
        let content_parts = contents[0]["parts"].as_array().unwrap();
        assert_eq!(content_parts.len(), 1);
    }

    #[test]
    fn build_request_body_with_real_prompts() {
        let system = prompts::system_prompt(AnalysisMode::Improve, None);
        let user = prompts::user_prompt(
            AnalysisMode::Improve,
            "Hello wrold",
            None,
            None,
            None,
            None,
        );

        let body = build_request_body(&system, &user);

        let system_text = body["system_instruction"]["parts"][0]["text"]
            .as_str()
            .unwrap();
        assert!(system_text.contains("expert editor"));

        let user_text = body["contents"][0]["parts"][0]["text"]
            .as_str()
            .unwrap();
        assert!(user_text.contains("Hello wrold"));
    }

    // =========================================================================
    // extract_text_from_response tests
    // =========================================================================

    #[test]
    fn extract_text_from_valid_response() {
        let response = r#"{
            "candidates": [
                {
                    "content": {
                        "parts": [{"text": "Hello world"}]
                    }
                }
            ]
        }"#;

        let result = extract_text_from_response(response);
        assert_eq!(result.unwrap(), "Hello world");
    }

    #[test]
    fn extract_text_from_response_with_json_content() {
        let response = r#"{
            "candidates": [
                {
                    "content": {
                        "parts": [{"text": "{\"corrected\": \"Hello world\", \"changes\": []}"}]
                    }
                }
            ]
        }"#;

        let result = extract_text_from_response(response);
        let text = result.unwrap();
        assert!(text.contains("corrected"));
        assert!(text.contains("Hello world"));
    }

    #[test]
    fn extract_text_from_response_no_candidates() {
        let response = r#"{"error": "something went wrong"}"#;

        let result = extract_text_from_response(response);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unexpected Gemini response structure"));
    }

    #[test]
    fn extract_text_from_response_empty_candidates() {
        let response = r#"{"candidates": []}"#;

        let result = extract_text_from_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn extract_text_from_response_no_content() {
        let response = r#"{"candidates": [{"finishReason": "STOP"}]}"#;

        let result = extract_text_from_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn extract_text_from_response_no_parts() {
        let response = r#"{"candidates": [{"content": {}}]}"#;

        let result = extract_text_from_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn extract_text_from_response_empty_parts() {
        let response = r#"{"candidates": [{"content": {"parts": []}}]}"#;

        let result = extract_text_from_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn extract_text_from_response_part_without_text() {
        let response = r#"{"candidates": [{"content": {"parts": [{"inlineData": "..."}]}}]}"#;

        let result = extract_text_from_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn extract_text_from_response_invalid_json() {
        let response = "not json at all";

        let result = extract_text_from_response(response);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse Gemini response JSON"));
    }

    // =========================================================================
    // Integration: prompt + body + response extraction + parser
    // =========================================================================

    #[test]
    fn integration_improve_mode_end_to_end() {
        // Simulate the full flow without HTTP:
        // 1. Build prompts
        let system = prompts::system_prompt(AnalysisMode::Improve, None);
        let user = prompts::user_prompt(
            AnalysisMode::Improve,
            "Hello wrold",
            Some(&ToneStyle::Formal),
            None,
            None,
            None,
        );

        // 2. Build request body
        let body = build_request_body(&system, &user);
        assert!(body["system_instruction"]["parts"][0]["text"]
            .as_str()
            .unwrap()
            .contains("expert editor"));
        assert!(body["contents"][0]["parts"][0]["text"]
            .as_str()
            .unwrap()
            .contains("formal"));

        // 3. Simulate API response
        let api_response = r#"{
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "{\"corrected\": \"Hello world\", \"changes\": [{\"original\": \"wrold\", \"replacement\": \"world\", \"reason\": \"Typo\"}], \"vocabulary\": []}"
                    }]
                }
            }]
        }"#;

        // 4. Extract text and parse
        let ai_text = extract_text_from_response(api_response).unwrap();
        let result = parser::parse_response(&ai_text, AnalysisMode::Improve, "Hello wrold");

        assert_eq!(result.mode, AnalysisMode::Improve);
        assert_eq!(result.corrected, "Hello world");
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].replacement, "world");
    }

    #[test]
    fn integration_tech_explain_mode_end_to_end() {
        let system = prompts::system_prompt(
            AnalysisMode::TechExplain,
            Some(ExplanationLevel::Eli5),
        );
        let user = prompts::user_prompt(
            AnalysisMode::TechExplain,
            "Docker",
            None,
            None,
            Some("Turkish"),
            None,
        );

        let body = build_request_body(&system, &user);
        assert!(body["system_instruction"]["parts"][0]["text"]
            .as_str()
            .unwrap()
            .contains("5 years old"));
        assert!(body["contents"][0]["parts"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Turkish"));

        let api_response = r#"{
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "{\"corrected\": \"Docker\", \"changes\": [], \"tldr\": \"Container platform\", \"explanation\": \"Docker is a containerization tool.\", \"resources\": [{\"title\": \"Docker Docs\", \"url\": \"https://docs.docker.com\"}]}"
                    }]
                }
            }]
        }"#;

        let ai_text = extract_text_from_response(api_response).unwrap();
        let result = parser::parse_response(&ai_text, AnalysisMode::TechExplain, "Docker");

        assert_eq!(result.mode, AnalysisMode::TechExplain);
        assert_eq!(result.corrected, "Docker");
        assert_eq!(result.tldr.as_deref(), Some("Container platform"));
        assert!(result.explanation.unwrap().contains("containerization"));
        assert_eq!(result.resources.unwrap().len(), 1);
    }

    #[test]
    fn integration_translate_mode_end_to_end() {
        let system = prompts::system_prompt(AnalysisMode::Translate, None);
        let user = prompts::user_prompt(
            AnalysisMode::Translate,
            "Merhaba",
            None,
            None,
            Some("Turkish"),
            Some("English"),
        );

        let body = build_request_body(&system, &user);
        assert!(body["contents"][0]["parts"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Turkish"));

        let api_response = r#"{
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "{\"corrected\": \"Hello\", \"changes\": [{\"original\": \"Merhaba\", \"replacement\": \"Hello\", \"reason\": \"Turkish to English\"}], \"explanation\": \"Merhaba means Hello in Turkish.\"}"
                    }]
                }
            }]
        }"#;

        let ai_text = extract_text_from_response(api_response).unwrap();
        let result = parser::parse_response(&ai_text, AnalysisMode::Translate, "Merhaba");

        assert_eq!(result.mode, AnalysisMode::Translate);
        assert_eq!(result.corrected, "Hello");
        assert_eq!(result.changes.len(), 1);
        assert!(result.explanation.unwrap().contains("Turkish"));
    }

    #[test]
    fn integration_gemini_response_with_markdown_fences() {
        // Gemini sometimes wraps JSON in markdown code fences
        let api_response = r#"{
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "```json\n{\"corrected\": \"Hello world\", \"changes\": []}\n```"
                    }]
                }
            }]
        }"#;

        let ai_text = extract_text_from_response(api_response).unwrap();
        let result = parser::parse_response(&ai_text, AnalysisMode::Improve, "Hello wrold");

        assert_eq!(result.corrected, "Hello world");
    }
}
