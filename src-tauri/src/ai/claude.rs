use serde_json::{json, Value};

use crate::models::{AnalysisMode, AnalysisResult, ExplanationLevel};
use crate::models::analysis::ToneStyle;

use super::parser;
use super::prompts;

/// Default Claude model.
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// API endpoint for the Anthropic Messages API.
const API_URL: &str = "https://api.anthropic.com/v1/messages";

/// Request timeout in seconds.
const TIMEOUT_SECS: u64 = 30;

/// Build the JSON request body for the Anthropic Messages API.
///
/// Uses `cache_control: {"type": "ephemeral"}` on the system prompt for prompt caching.
/// This is a pure function, easily testable without HTTP.
fn build_request_body(model: &str, system_prompt: &str, user_prompt: &str) -> Value {
    json!({
        "model": model,
        "max_tokens": 2048,
        "system": [
            {
                "type": "text",
                "text": system_prompt,
                "cache_control": {"type": "ephemeral"}
            }
        ],
        "messages": [
            {
                "role": "user",
                "content": user_prompt
            }
        ]
    })
}

/// Extract the text content from a Claude API response JSON string.
///
/// Navigates: `content[0].text`
fn extract_text_from_response(response_body: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(response_body)
        .map_err(|e| format!("Failed to parse Claude response JSON: {}", e))?;

    let text = value
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|block| {
            // Only extract text from blocks with type "text"
            let block_type = block.get("type").and_then(|t| t.as_str());
            if block_type == Some("text") {
                block.get("text").and_then(|t| t.as_str())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            format!(
                "Unexpected Claude response structure: {}",
                &response_body[..response_body.len().min(500)]
            )
        })?;

    Ok(text.to_string())
}

/// Analyze text using the Anthropic Claude API.
///
/// 1. Builds system and user prompts from mode/level/options
/// 2. Calls the Anthropic Messages endpoint via HTTP
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

    let body = build_request_body(model, &system, &user);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(API_URL)
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Claude API request failed: {}", e))?;

    let status = response.status();
    let response_body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Claude response body: {}", e))?;

    if !status.is_success() {
        return Err(format!("Claude API {}: {}", status.as_u16(), response_body));
    }

    let ai_text = extract_text_from_response(&response_body)?;

    Ok(parser::parse_response(&ai_text, mode, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // build_request_body tests
    // =========================================================================

    #[test]
    fn build_request_body_has_model() {
        let body = build_request_body(DEFAULT_MODEL, "System prompt", "User prompt");

        assert_eq!(
            body["model"].as_str().unwrap(),
            "claude-sonnet-4-20250514"
        );
    }

    #[test]
    fn build_request_body_has_max_tokens() {
        let body = build_request_body(DEFAULT_MODEL, "sys", "user");

        assert_eq!(body["max_tokens"].as_u64().unwrap(), 2048);
    }

    #[test]
    fn build_request_body_has_system_with_cache_control() {
        let body = build_request_body(DEFAULT_MODEL, "You are an expert.", "Improve this text");

        let system = body["system"].as_array().unwrap();
        assert_eq!(system.len(), 1);

        let block = &system[0];
        assert_eq!(block["type"].as_str().unwrap(), "text");
        assert_eq!(block["text"].as_str().unwrap(), "You are an expert.");
        assert_eq!(
            block["cache_control"]["type"].as_str().unwrap(),
            "ephemeral"
        );
    }

    #[test]
    fn build_request_body_has_messages() {
        let body = build_request_body(DEFAULT_MODEL, "System prompt", "User prompt here");

        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"].as_str().unwrap(), "user");
        assert_eq!(messages[0]["content"].as_str().unwrap(), "User prompt here");
    }

    #[test]
    fn build_request_body_structure_is_complete() {
        let body = build_request_body("custom-model", "sys", "user");

        // Verify all top-level keys exist
        assert!(body.get("model").is_some());
        assert!(body.get("max_tokens").is_some());
        assert!(body.get("system").is_some());
        assert!(body.get("messages").is_some());

        // Verify model is the custom one
        assert_eq!(body["model"].as_str().unwrap(), "custom-model");
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

        let body = build_request_body(DEFAULT_MODEL, &system, &user);

        let system_text = body["system"][0]["text"].as_str().unwrap();
        assert!(system_text.contains("expert editor"));

        let user_text = body["messages"][0]["content"].as_str().unwrap();
        assert!(user_text.contains("Hello wrold"));
    }

    // =========================================================================
    // extract_text_from_response tests
    // =========================================================================

    #[test]
    fn extract_text_from_valid_response() {
        let response = r#"{
            "content": [
                {
                    "type": "text",
                    "text": "Hello world"
                }
            ]
        }"#;

        let result = extract_text_from_response(response);
        assert_eq!(result.unwrap(), "Hello world");
    }

    #[test]
    fn extract_text_from_response_with_json_content() {
        let response = r#"{
            "content": [
                {
                    "type": "text",
                    "text": "{\"corrected\": \"Hello world\", \"changes\": []}"
                }
            ]
        }"#;

        let result = extract_text_from_response(response);
        let text = result.unwrap();
        assert!(text.contains("corrected"));
        assert!(text.contains("Hello world"));
    }

    #[test]
    fn extract_text_from_response_missing_content() {
        let response = r#"{"error": {"type": "invalid_request_error", "message": "bad request"}}"#;

        let result = extract_text_from_response(response);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unexpected Claude response structure"));
    }

    #[test]
    fn extract_text_from_response_empty_content() {
        let response = r#"{"content": []}"#;

        let result = extract_text_from_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn extract_text_from_response_non_text_block() {
        let response = r#"{"content": [{"type": "tool_use", "id": "123", "name": "test"}]}"#;

        let result = extract_text_from_response(response);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unexpected Claude response structure"));
    }

    #[test]
    fn extract_text_from_response_invalid_json() {
        let response = "not json at all";

        let result = extract_text_from_response(response);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse Claude response JSON"));
    }

    #[test]
    fn extract_text_from_response_content_without_type() {
        // A content block without the "type" field should not match
        let response = r#"{"content": [{"text": "Hello"}]}"#;

        let result = extract_text_from_response(response);
        assert!(result.is_err());
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
        let body = build_request_body(DEFAULT_MODEL, &system, &user);
        assert!(body["system"][0]["text"]
            .as_str()
            .unwrap()
            .contains("expert editor"));
        assert!(body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("formal"));
        // Verify cache_control is present
        assert_eq!(
            body["system"][0]["cache_control"]["type"].as_str().unwrap(),
            "ephemeral"
        );

        // 3. Simulate API response
        let api_response = r#"{
            "content": [{
                "type": "text",
                "text": "{\"corrected\": \"Hello world\", \"changes\": [{\"original\": \"wrold\", \"replacement\": \"world\", \"reason\": \"Typo\"}], \"vocabulary\": []}"
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

        let body = build_request_body(DEFAULT_MODEL, &system, &user);
        assert!(body["system"][0]["text"]
            .as_str()
            .unwrap()
            .contains("5 years old"));
        assert!(body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Turkish"));

        let api_response = r#"{
            "content": [{
                "type": "text",
                "text": "{\"corrected\": \"Docker\", \"changes\": [], \"tldr\": \"Container platform\", \"explanation\": \"Docker is a containerization tool.\", \"resources\": [{\"title\": \"Docker Docs\", \"url\": \"https://docs.docker.com\"}]}"
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

        let body = build_request_body(DEFAULT_MODEL, &system, &user);
        assert!(body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Turkish"));

        let api_response = r#"{
            "content": [{
                "type": "text",
                "text": "{\"corrected\": \"Hello\", \"changes\": [{\"original\": \"Merhaba\", \"replacement\": \"Hello\", \"reason\": \"Turkish to English\"}], \"explanation\": \"Merhaba means Hello in Turkish.\"}"
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
    fn integration_claude_response_with_markdown_fences() {
        // Claude might also wrap JSON in markdown code fences
        let api_response = r#"{
            "content": [{
                "type": "text",
                "text": "```json\n{\"corrected\": \"Hello world\", \"changes\": []}\n```"
            }]
        }"#;

        let ai_text = extract_text_from_response(api_response).unwrap();
        let result = parser::parse_response(&ai_text, AnalysisMode::Improve, "Hello wrold");

        assert_eq!(result.corrected, "Hello world");
    }

    // =========================================================================
    // Constants and configuration tests
    // =========================================================================

    #[test]
    fn default_model_is_claude_sonnet() {
        assert_eq!(DEFAULT_MODEL, "claude-sonnet-4-20250514");
    }

    #[test]
    fn api_url_is_anthropic_messages() {
        assert_eq!(API_URL, "https://api.anthropic.com/v1/messages");
    }
}
