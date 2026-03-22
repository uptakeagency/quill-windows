use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

use crate::models::{Alternative, AnalysisMode, AnalysisResult, ResourceLink, TextChange, VocabularyCard};

static FENCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"```(?:json|JSON)?\s*\n?").unwrap()
});

static EXPLANATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#""explanation"\s*:\s*""#).unwrap()
});

/// Log a debug message for parser diagnostics.
#[allow(dead_code)]
pub fn debug_log(msg: &str) {
    log::debug!("{}", msg);
}

/// Private struct matching the expected AI JSON output.
/// Unlike `AnalysisResult`, this omits `mode` and `original`.
#[derive(Debug, Deserialize)]
struct AIResponse {
    corrected: Option<String>,
    #[serde(default)]
    changes: Vec<TextChange>,
    vocabulary: Option<Vec<VocabularyCard>>,
    explanation: Option<String>,
    tldr: Option<String>,
    resources: Option<Vec<ResourceLink>>,
    alternatives: Option<Vec<Alternative>>,
    #[serde(default)]
    levels: Option<HashMap<String, String>>,
    // Top-level level fields (AI may return these instead of nested "levels" object)
    eli5: Option<String>,
    eli15: Option<String>,
    professional: Option<String>,
    samples: Option<String>,
    #[serde(rename = "resources_explanation")]
    resources_text: Option<String>,
    #[serde(rename = "alternatives_text")]
    alternatives_text: Option<String>,
}

/// Parse a raw AI response string into an `AnalysisResult`.
///
/// Uses a multi-layer fallback strategy:
/// 1. Extract JSON from markdown fences or brace matching
/// 2. Direct serde deserialization
/// 3. Sanitize literal newlines/tabs inside JSON strings, retry serde
/// 4. Lenient `serde_json::Value` parsing with manual field extraction
/// 5. Regex extraction of the "explanation" field
/// 6. Final fallback: raw text as explanation or corrected text
pub fn parse_response(raw: &str, mode: AnalysisMode, original_text: &str) -> AnalysisResult {
    // Sanitize BEFORE extracting JSON — literal newlines inside JSON strings
    // break the brace matcher's in_string tracking
    let sanitized_raw = sanitize_json(raw);
    let cleaned = extract_json(&sanitized_raw);

    debug_log(&format!("\n=== NEW PARSE ({} chars) ===\n{}", raw.len(), raw));
    debug_log(&format!("[QUILL] Cleaned JSON ({} chars, first 200): {:?}", cleaned.len(), &cleaned[..cleaned.len().min(200)]));

    // Layer 2: Try direct deserialization
    let mut result = try_decode(&cleaned, mode, original_text);
    debug_log(&format!("[QUILL] Layer 2 (strict serde): {}", if result.is_some() { "OK" } else { "FAIL" }));

    if let Some(ref r) = result {
        debug_log(&format!("[QUILL] Layer 2 levels keys: {:?}", r.levels.as_ref().map(|l| l.keys().collect::<Vec<_>>())));
    }

    // Layer 3 & 4: Lenient parsing
    if result.is_none() {
        result = try_decode_lenient(&cleaned, mode, original_text);
        debug_log(&format!("[QUILL] Layer 4 (lenient): {}", if result.is_some() { "OK" } else { "FAIL" }));
        if let Some(ref r) = result {
            debug_log(&format!("[QUILL] Layer 4 levels keys: {:?}", r.levels.as_ref().map(|l| l.keys().collect::<Vec<_>>())));
        }
    }

    // Layer 5: Regex extraction of explanation field
    if result.is_none() {
        if let Some(explanation) = extract_explanation_field(&cleaned) {
            result = Some(AnalysisResult {
                mode,
                original: original_text.to_string(),
                corrected: original_text.to_string(),
                changes: vec![],
                explanation: Some(explanation),
                tldr: None,
                resources: None,
                alternatives: None,
                vocabulary: vec![],
                levels: None,
            });
        }
    }

    // Layer 6: Final fallback
    let mut final_result = result.unwrap_or_else(|| fallback_result(&cleaned, mode, original_text));

    // Normalize escapes in explanation and tldr
    if let Some(ref explanation) = final_result.explanation {
        final_result.explanation = Some(normalize_escapes(explanation));
    }
    if let Some(ref tldr) = final_result.tldr {
        final_result.tldr = Some(normalize_escapes(tldr));
    }

    // Normalize escapes in all level explanations
    if let Some(ref mut levels) = final_result.levels {
        for value in levels.values_mut() {
            *value = normalize_escapes(value);
        }
    }

    debug_log(&format!("[QUILL] FINAL levels keys: {:?}", final_result.levels.as_ref().map(|l| l.keys().collect::<Vec<_>>())));
    debug_log(&format!("[QUILL] FINAL explanation present: {}", final_result.explanation.is_some()));

    // Mode-specific field filtering
    if final_result.mode != AnalysisMode::Improve {
        final_result.vocabulary = vec![];
    }
    if final_result.mode != AnalysisMode::TechExplain {
        final_result.alternatives = None;
    }

    final_result
}

/// Extract JSON content from a string, stripping markdown fences or finding matching braces.
///
/// Strategy: try brace matching from multiple starting points (original text +
/// text after each markdown fence), then pick the largest result. This handles:
/// - Pure JSON: finds the full object directly
/// - Fenced JSON: strips fence, finds JSON after it
/// - Commentary with `{code}` before fenced JSON: skips small code snippets
/// - JSON containing ``` inside string values: original text yields the full object
fn extract_json(text: &str) -> String {
    // Build candidate starting points: original text + text after each fence
    let mut candidates: Vec<&str> = vec![text];
    let mut search = text;
    while let Some(m) = FENCE_RE.find(search) {
        let after = &search[m.end()..];
        candidates.push(after);
        search = after;
    }

    // Try brace matching from the first { in each candidate, keep the longest
    let mut best: Option<String> = None;
    for content in candidates {
        if let Some(start) = content.find('{') {
            if let Some(end) = find_matching_brace(content, start) {
                let matched = &content[start..=end];
                if best.as_ref().map_or(true, |b| matched.len() > b.len()) {
                    best = Some(matched.to_string());
                }
            }
        }
    }

    best.unwrap_or_else(|| text.to_string())
}

/// Find the index of the closing brace that matches the opening brace at `start`.
/// Handles nested braces and string literals (with escape sequences).
fn find_matching_brace(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut last_close: Option<usize> = None;
    let mut i = start;

    while i < len {
        let ch = bytes[i];

        if escaped {
            escaped = false;
            i += 1;
            continue;
        }

        if ch == b'\\' && in_string {
            escaped = true;
            i += 1;
            continue;
        }

        if ch == b'"' {
            in_string = !in_string;
            i += 1;
            continue;
        }

        if !in_string {
            if ch == b'{' {
                depth += 1;
            } else if ch == b'}' {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
                last_close = Some(i);
            }
        }

        i += 1;
    }

    last_close
}

/// Sanitize JSON by escaping literal newlines, carriage returns, and tabs inside string values.
fn sanitize_json(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;

    for ch in text.chars() {
        if escaped {
            result.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' && in_string {
            result.push(ch);
            escaped = true;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            result.push(ch);
            continue;
        }

        if in_string {
            match ch {
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                _ => result.push(ch),
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Extract the "explanation" field value from malformed JSON using regex.
fn extract_explanation_field(text: &str) -> Option<String> {
    let mat = EXPLANATION_RE.find(text)?;
    let start = mat.end();

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut end = start;
    let mut escaped = false;

    let mut i = start;
    while i < len {
        let ch = bytes[i];
        if escaped {
            escaped = false;
            end = i + 1;
            i += 1;
            continue;
        }
        if ch == b'\\' {
            escaped = true;
            end = i + 1;
            i += 1;
            continue;
        }
        if ch == b'"' {
            end = i;
            break;
        }
        end = i + 1;
        i += 1;
    }

    if end <= start {
        return None;
    }

    let raw = &text[start..end];
    let unescaped = raw
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\");

    Some(unescaped)
}

/// Normalize escape sequences in explanation/tldr text.
/// Converts literal `\n` → newline and `\t` → tab.
fn normalize_escapes(text: &str) -> String {
    text.replace("\\n", "\n").replace("\\t", "\t")
}

/// Try to deserialize the JSON string directly into `AIResponse`, then convert to `AnalysisResult`.
fn try_decode(json: &str, mode: AnalysisMode, original_text: &str) -> Option<AnalysisResult> {
    match serde_json::from_str::<AIResponse>(json) {
        Ok(response) => Some(ai_response_to_result(response, mode, original_text)),
        Err(e) => {
            debug_log(&format!("[QUILL] serde error: {}", e));
            None
        }
    }
}

/// Lenient parsing using `serde_json::Value` with manual field extraction.
fn try_decode_lenient(json: &str, mode: AnalysisMode, original_text: &str) -> Option<AnalysisResult> {
    let value: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            debug_log(&format!("[QUILL] lenient Value parse error: {}", e));
            return None;
        }
    };
    let obj = value.as_object()?;

    let corrected = obj
        .get("corrected")
        .and_then(|v| v.as_str())
        .unwrap_or(original_text)
        .to_string();

    let explanation = obj
        .get("explanation")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let tldr = obj
        .get("tldr")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let changes = obj
        .get("changes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let o = item.as_object()?;
                    Some(TextChange {
                        original: o.get("original")?.as_str()?.to_string(),
                        replacement: o.get("replacement")?.as_str()?.to_string(),
                        reason: o.get("reason")?.as_str().unwrap_or("").to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let vocabulary = obj
        .get("vocabulary")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let o = item.as_object()?;
                    Some(VocabularyCard {
                        word: o.get("word")?.as_str()?.to_string(),
                        suggestion: o.get("suggestion")?.as_str().unwrap_or("").to_string(),
                        definition: o.get("definition")?.as_str().unwrap_or("").to_string(),
                        example: o.get("example")?.as_str().unwrap_or("").to_string(),
                        level: o.get("level")?.as_str().unwrap_or("").to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let resources = obj
        .get("resources")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let o = item.as_object()?;
                    Some(ResourceLink {
                        title: o.get("title")?.as_str()?.to_string(),
                        url: o.get("url")?.as_str()?.to_string(),
                    })
                })
                .collect()
        });

    let alternatives = obj
        .get("alternatives")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let o = item.as_object()?;
                    let name = o.get("name")?.as_str()?.to_string();
                    let description = o.get("description")?.as_str().unwrap_or("").to_string();
                    // Handle pros/cons as either string or array
                    let pros = extract_string_or_array(o.get("pros"));
                    let cons = extract_string_or_array(o.get("cons"));
                    let url = o.get("url")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(Alternative {
                        name,
                        description,
                        pros,
                        cons,
                        url,
                    })
                })
                .collect()
        });

    // Try nested "levels" object first, then fall back to top-level level fields.
    // AI may return either: {"levels": {"eli5": "...", ...}} or {"eli5": "...", ...}
    // Only collect string-valued top-level fields (skip "resources" and "alternatives"
    // which are arrays, not level text)
    let text_level_keys = ["eli5", "eli15", "professional", "samples"];
    // These keys may be strings (level text) or arrays (structured data) — only take strings
    let ambiguous_keys = ["resources", "alternatives"];

    let levels = {
        // Option 1: nested "levels" object
        let nested = obj
            .get("levels")
            .and_then(|v| v.as_object())
            .map(|lvls| {
                lvls.iter()
                    .filter_map(|(k, v)| {
                        // String value → use directly
                        if let Some(s) = v.as_str() {
                            return Some((k.clone(), s.to_string()));
                        }
                        // Array of strings → join as bullet list
                        if let Some(arr) = v.as_array() {
                            let items: Vec<String> = arr.iter()
                                .filter_map(|item| item.as_str().map(|s| format!("- {}", s)))
                                .collect();
                            if !items.is_empty() {
                                return Some((k.clone(), items.join("\n")));
                            }
                        }
                        None
                    })
                    .collect::<HashMap<String, String>>()
            });

        if nested.as_ref().map_or(true, |m| m.is_empty()) {
            // Option 2: top-level fields (eli5, eli15, professional, samples, etc.)
            let mut top_level: HashMap<String, String> = text_level_keys
                .iter()
                .filter_map(|key| {
                    obj.get(*key)
                        .and_then(|v| v.as_str())
                        .map(|s| (key.to_string(), s.to_string()))
                })
                .collect();

            // For ambiguous keys, only take if they're strings (not arrays)
            for key in &ambiguous_keys {
                if let Some(Value::String(s)) = obj.get(*key) {
                    top_level.insert(key.to_string(), s.clone());
                }
            }

            // Also check "resources_explanation" / "alternatives_text" variants
            if let Some(s) = obj.get("resources_explanation").and_then(|v| v.as_str()) {
                top_level.insert("resources".to_string(), s.to_string());
            }
            if let Some(s) = obj.get("alternatives_text").and_then(|v| v.as_str()) {
                top_level.insert("alternatives".to_string(), s.to_string());
            }
            if let Some(s) = obj.get("alternatives_context").and_then(|v| v.as_str()) {
                top_level.insert("alternatives".to_string(), s.to_string());
            }

            if top_level.is_empty() { nested } else { Some(top_level) }
        } else {
            nested
        }
    };

    // Use eli15 from levels as fallback explanation
    let explanation = explanation.or_else(|| {
        levels.as_ref().and_then(|l| l.get("eli15").cloned())
    });

    Some(AnalysisResult {
        mode,
        original: original_text.to_string(),
        corrected,
        changes,
        explanation,
        tldr,
        resources,
        alternatives,
        vocabulary,
        levels,
    })
}

/// Extract a `Vec<String>` from a JSON value that may be either a string or an array of strings.
fn extract_string_or_array(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => vec![],
    }
}

/// Convert an `AIResponse` into an `AnalysisResult`, filling in mode and original.
fn ai_response_to_result(response: AIResponse, mode: AnalysisMode, original_text: &str) -> AnalysisResult {
    // Build levels from nested object or top-level fields
    let levels = if response.levels.as_ref().map_or(true, |m| m.is_empty()) {
        // Collect top-level level fields
        let mut map = HashMap::new();
        if let Some(v) = response.eli5 { map.insert("eli5".to_string(), v); }
        if let Some(v) = response.eli15 { map.insert("eli15".to_string(), v); }
        if let Some(v) = response.professional { map.insert("professional".to_string(), v); }
        if let Some(v) = response.samples { map.insert("samples".to_string(), v); }
        if let Some(v) = response.resources_text { map.insert("resources".to_string(), v); }
        if let Some(v) = response.alternatives_text { map.insert("alternatives".to_string(), v); }
        if map.is_empty() { response.levels } else { Some(map) }
    } else {
        response.levels
    };

    let explanation = response.explanation.or_else(|| {
        levels.as_ref().and_then(|l| l.get("eli15").cloned())
    });
    AnalysisResult {
        mode,
        original: original_text.to_string(),
        corrected: response.corrected.unwrap_or_else(|| original_text.to_string()),
        changes: response.changes,
        explanation,
        tldr: response.tldr,
        resources: response.resources,
        alternatives: response.alternatives,
        vocabulary: response.vocabulary.unwrap_or_default(),
        levels,
    }
}

/// Strip JSON structural syntax from text, leaving only human-readable string values.
/// Used when JSON parsing fails but we still want to show meaningful content.
fn strip_json_to_text(text: &str) -> String {
    // Extract all string values from the text using a simple state machine
    let mut values = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            // Collect string content until closing quote
            let mut value = String::new();
            let mut escaped = false;
            for c in chars.by_ref() {
                if escaped {
                    match c {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        _ => { value.push('\\'); value.push(c); }
                    }
                    escaped = false;
                    continue;
                }
                if c == '\\' { escaped = true; continue; }
                if c == '"' { break; }
                value.push(c);
            }
            // Only keep values that look like human text, not JSON keys
            // (keys like "corrected", "explanation" are < 15 chars) or bare URLs
            let trimmed = value.trim();
            if trimmed.len() > 15 && !trimmed.starts_with("http") {
                values.push(trimmed.to_string());
            }
        }
    }
    if values.is_empty() {
        // No string values found, return original text stripped of JSON syntax
        text.chars()
            .filter(|c| !matches!(c, '{' | '}' | '[' | ']'))
            .collect::<String>()
            .trim()
            .to_string()
    } else {
        values.join("\n\n")
    }
}

/// Fallback result when all parsing fails.
/// For explanation modes (TechExplain, Translate): raw text becomes explanation.
/// For Improve mode: raw text becomes corrected text.
fn fallback_result(text: &str, mode: AnalysisMode, original_text: &str) -> AnalysisResult {
    let trimmed = text.trim();
    let is_explanation_mode = mode == AnalysisMode::TechExplain || mode == AnalysisMode::Translate;

    // For explanation modes, strip JSON structure from fallback text
    // to prevent raw JSON from appearing in the UI
    let explanation_text = if is_explanation_mode && trimmed.contains('{') {
        strip_json_to_text(trimmed)
    } else {
        trimmed.to_string()
    };

    AnalysisResult {
        mode,
        original: original_text.to_string(),
        corrected: if is_explanation_mode {
            original_text.to_string()
        } else {
            trimmed.to_string()
        },
        changes: vec![],
        explanation: if is_explanation_mode {
            Some(explanation_text)
        } else {
            None
        },
        tldr: None,
        resources: None,
        alternatives: None,
        vocabulary: vec![],
        levels: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Layer 1: Clean valid JSON parses correctly for each mode
    // =========================================================================

    #[test]
    fn layer1_valid_json_improve_mode() {
        let json = r#"{
            "corrected": "Hello world",
            "changes": [{"original": "wrold", "replacement": "world", "reason": "Typo"}],
            "explanation": "Fixed a typo",
            "tldr": "Typo correction",
            "vocabulary": [{"word": "world", "suggestion": "globe", "definition": "The earth", "example": "Hello world", "level": "A1"}]
        }"#;

        let result = parse_response(json, AnalysisMode::Improve, "Hello wrold");
        assert_eq!(result.mode, AnalysisMode::Improve);
        assert_eq!(result.original, "Hello wrold");
        assert_eq!(result.corrected, "Hello world");
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].original, "wrold");
        assert_eq!(result.changes[0].replacement, "world");
        assert_eq!(result.explanation.as_deref(), Some("Fixed a typo"));
        assert_eq!(result.tldr.as_deref(), Some("Typo correction"));
        assert_eq!(result.vocabulary.len(), 1);
        assert_eq!(result.vocabulary[0].word, "world");
    }

    #[test]
    fn layer1_valid_json_tech_explain_mode() {
        let json = r#"{
            "corrected": "async/await",
            "changes": [],
            "explanation": "Async/await is a pattern for handling asynchronous operations.",
            "tldr": "Asynchronous programming pattern",
            "resources": [{"title": "MDN Docs", "url": "https://mdn.io/async"}],
            "alternatives": [{"name": "Promises", "description": "Callback-based", "pros": ["Simple"], "cons": ["Nesting"]}]
        }"#;

        let result = parse_response(json, AnalysisMode::TechExplain, "async/await");
        assert_eq!(result.mode, AnalysisMode::TechExplain);
        assert_eq!(result.original, "async/await");
        assert!(result.explanation.is_some());
        assert_eq!(result.tldr.as_deref(), Some("Asynchronous programming pattern"));
        assert_eq!(result.resources.as_ref().unwrap().len(), 1);
        assert_eq!(result.alternatives.as_ref().unwrap().len(), 1);
        assert_eq!(result.alternatives.as_ref().unwrap()[0].name, "Promises");
    }

    #[test]
    fn layer1_valid_json_translate_mode() {
        let json = r#"{
            "corrected": "Merhaba dunya",
            "changes": [],
            "explanation": "Translation from English to Turkish"
        }"#;

        let result = parse_response(json, AnalysisMode::Translate, "Hello world");
        assert_eq!(result.mode, AnalysisMode::Translate);
        assert_eq!(result.corrected, "Merhaba dunya");
        assert_eq!(result.explanation.as_deref(), Some("Translation from English to Turkish"));
    }

    // =========================================================================
    // Layer 2: JSON with markdown fences
    // =========================================================================

    #[test]
    fn layer2_json_with_markdown_fences() {
        let raw = r#"Here's the analysis:

```json
{
    "corrected": "Hello world",
    "changes": [{"original": "wrold", "replacement": "world", "reason": "Typo"}],
    "explanation": "Fixed a typo"
}
```

Hope that helps!"#;

        let result = parse_response(raw, AnalysisMode::Improve, "Hello wrold");
        assert_eq!(result.corrected, "Hello world");
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.explanation.as_deref(), Some("Fixed a typo"));
    }

    #[test]
    fn layer2_json_with_plain_code_fences() {
        let raw = "```\n{\"corrected\": \"test\", \"changes\": []}\n```";

        let result = parse_response(raw, AnalysisMode::Improve, "tset");
        assert_eq!(result.corrected, "test");
    }

    // =========================================================================
    // Layer 3: JSON with literal newlines inside strings -> sanitized
    // =========================================================================

    #[test]
    fn layer3_json_with_literal_newlines_in_strings() {
        // JSON with actual newlines inside string values (invalid JSON)
        let raw = "{\n    \"corrected\": \"Hello world\",\n    \"changes\": [],\n    \"explanation\": \"Line one\nLine two\nLine three\"\n}";

        let result = parse_response(raw, AnalysisMode::TechExplain, "test");
        // Should succeed via sanitization
        assert!(result.explanation.is_some());
        let explanation = result.explanation.unwrap();
        assert!(explanation.contains("Line one"));
        assert!(explanation.contains("Line two"));
    }

    #[test]
    fn layer3_json_with_literal_tabs_in_strings() {
        let raw = "{\n    \"corrected\": \"test\",\n    \"changes\": [],\n    \"explanation\": \"Before\tAfter\"\n}";

        let result = parse_response(raw, AnalysisMode::TechExplain, "test");
        assert!(result.explanation.is_some());
    }

    // =========================================================================
    // Layer 4: Lenient Value-based parsing with missing fields
    // =========================================================================

    #[test]
    fn layer4_lenient_parsing_missing_corrected() {
        // Valid JSON but missing the "corrected" field
        let json = r#"{"explanation": "This is async/await", "changes": []}"#;

        let result = parse_response(json, AnalysisMode::TechExplain, "async/await");
        // Should fall back to original_text for corrected
        assert_eq!(result.corrected, "async/await");
        assert_eq!(result.explanation.as_deref(), Some("This is async/await"));
    }

    #[test]
    fn layer4_lenient_alternatives_pros_as_string() {
        // AI returns pros/cons as strings instead of arrays
        let json = r#"{
            "corrected": "test",
            "changes": [],
            "explanation": "test explanation",
            "alternatives": [{"name": "Alt1", "description": "Desc", "pros": "Simple to use", "cons": "Limited"}]
        }"#;

        // Direct serde will fail because Alternative expects Vec<String> for pros/cons
        // but lenient parsing should handle it
        let result = parse_response(json, AnalysisMode::TechExplain, "test");
        let alts = result.alternatives.unwrap();
        assert_eq!(alts.len(), 1);
        assert_eq!(alts[0].name, "Alt1");
        assert_eq!(alts[0].pros, vec!["Simple to use"]);
        assert_eq!(alts[0].cons, vec!["Limited"]);
    }

    #[test]
    fn layer4_lenient_partial_fields() {
        let json = r#"{
            "corrected": "fixed text",
            "explanation": "Some explanation"
        }"#;

        let result = parse_response(json, AnalysisMode::Improve, "original");
        assert_eq!(result.corrected, "fixed text");
        assert_eq!(result.explanation.as_deref(), Some("Some explanation"));
        assert!(result.changes.is_empty());
        assert!(result.vocabulary.is_empty());
    }

    // =========================================================================
    // Layer 5: Regex extraction of explanation field from malformed JSON
    // =========================================================================

    #[test]
    fn layer5_regex_explanation_extraction() {
        // Malformed JSON that can't be parsed even leniently
        let raw = r#"{"explanation": "This explains the concept", "broken: field"#;

        let result = parse_response(raw, AnalysisMode::TechExplain, "test");
        assert_eq!(result.explanation.as_deref(), Some("This explains the concept"));
        assert_eq!(result.corrected, "test"); // original_text used
    }

    #[test]
    fn layer5_regex_explanation_with_escaped_quotes() {
        let raw = r#"{"explanation": "He said \"hello\" to the world", bad_json"#;

        let result = parse_response(raw, AnalysisMode::TechExplain, "test");
        let explanation = result.explanation.unwrap();
        assert!(explanation.contains("hello"));
    }

    // =========================================================================
    // Layer 6: Complete garbage -> fallback result
    // =========================================================================

    #[test]
    fn layer6_garbage_fallback_tech_explain() {
        let raw = "This is just plain text, not JSON at all.";

        let result = parse_response(raw, AnalysisMode::TechExplain, "some term");
        assert_eq!(result.mode, AnalysisMode::TechExplain);
        assert_eq!(result.original, "some term");
        assert_eq!(result.corrected, "some term"); // original preserved
        assert_eq!(
            result.explanation.as_deref(),
            Some("This is just plain text, not JSON at all.")
        );
        assert!(result.changes.is_empty());
    }

    #[test]
    fn layer6_garbage_fallback_improve() {
        let raw = "The corrected text is here.";

        let result = parse_response(raw, AnalysisMode::Improve, "original text");
        assert_eq!(result.mode, AnalysisMode::Improve);
        assert_eq!(result.original, "original text");
        assert_eq!(result.corrected, "The corrected text is here.");
        assert!(result.explanation.is_none()); // Improve mode doesn't get explanation fallback
    }

    #[test]
    fn layer6_garbage_fallback_translate() {
        let raw = "Bonjour le monde";

        let result = parse_response(raw, AnalysisMode::Translate, "Hello world");
        assert_eq!(result.corrected, "Hello world"); // original preserved for translate
        assert_eq!(result.explanation.as_deref(), Some("Bonjour le monde"));
    }

    // =========================================================================
    // extract_json tests
    // =========================================================================

    #[test]
    fn extract_json_from_markdown_fences() {
        let input = "Some text\n```json\n{\"key\": \"value\"}\n```\nMore text";
        let result = extract_json(input);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn extract_json_brace_matching_with_prefix() {
        let input = "Here's the result: {\"key\": \"value\"}";
        let result = extract_json(input);
        assert_eq!(result, "{\"key\": \"value\"}");
    }

    #[test]
    fn extract_json_nested_braces() {
        let input = r#"{"outer": {"inner": "value"}, "other": "test"}"#;
        let result = extract_json(input);
        assert_eq!(result, input);
    }

    #[test]
    fn extract_json_braces_in_strings() {
        let input = r#"{"code": "if (x) { return; }", "name": "test"}"#;
        let result = extract_json(input);
        assert_eq!(result, input);
    }

    #[test]
    fn extract_json_ignores_backticks_inside_json_strings() {
        // JSON contains ``` inside a string value (code samples) — must NOT
        // be treated as a markdown fence wrapping the JSON.
        let json = serde_json::json!({
            "levels": {
                "samples": "### Example\n```js\nfoo({ bar: 1 });\n```"
            }
        }).to_string();
        let result = extract_json(&json);
        assert_eq!(result, json, "should return entire JSON, not a substring from inside the code sample");
    }

    #[test]
    fn extract_json_commentary_with_braces_before_fenced_json() {
        // AI puts commentary with code snippets (containing {}) before the JSON fence.
        // extract_json should find the real JSON (largest match), not the code snippet.
        let input = "TypeScript allows types like { id: number; name: string; }\n\n```json\n{\"tldr\": \"Type system\", \"levels\": {\"eli5\": \"Simple\", \"eli15\": \"Medium\"}}\n```";
        let result = extract_json(input);
        assert!(result.contains("\"tldr\""), "should extract the full JSON, not the TypeScript snippet");
        assert!(result.contains("\"levels\""), "should contain levels");
        assert!(!result.contains("id: number"), "should NOT contain the TypeScript code snippet");
    }

    #[test]
    fn extract_json_handles_fence_before_json() {
        // Real markdown fence wrapping JSON should still be stripped
        let input = "```json\n{\"key\": \"value with ```code``` inside\"}\n```";
        let result = extract_json(input);
        assert!(result.contains("\"key\""), "should extract the JSON object");
        assert!(result.starts_with('{'), "should start with opening brace");
    }

    #[test]
    fn extract_json_no_json_present() {
        let input = "Just plain text with no JSON";
        let result = extract_json(input);
        assert_eq!(result, input);
    }

    // =========================================================================
    // find_matching_brace tests
    // =========================================================================

    #[test]
    fn find_matching_brace_simple() {
        let text = r#"{"key": "value"}"#;
        assert_eq!(find_matching_brace(text, 0), Some(text.len() - 1));
    }

    #[test]
    fn find_matching_brace_nested() {
        let text = r#"{"a": {"b": "c"}}"#;
        assert_eq!(find_matching_brace(text, 0), Some(text.len() - 1));
    }

    #[test]
    fn find_matching_brace_with_string_braces() {
        let text = r#"{"code": "function() { return {}; }"}"#;
        // The braces inside the string should be ignored
        assert_eq!(find_matching_brace(text, 0), Some(text.len() - 1));
    }

    #[test]
    fn find_matching_brace_with_escaped_quotes() {
        let text = r#"{"msg": "He said \"hi\""}"#;
        assert_eq!(find_matching_brace(text, 0), Some(text.len() - 1));
    }

    #[test]
    fn find_matching_brace_unclosed() {
        let text = r#"{"key": "value""#;
        assert_eq!(find_matching_brace(text, 0), None);
    }

    #[test]
    fn find_matching_brace_partial_returns_last_close() {
        // Outer object has one complete nested object but outer is not closed
        let text = r#"{"a": {"b": "c"}, "d": "#;
        let result = find_matching_brace(text, 0);
        // Should return the position of the } after "c"
        assert!(result.is_some());
        let pos = result.unwrap();
        assert_eq!(text.as_bytes()[pos], b'}');
    }

    // =========================================================================
    // sanitize_json tests
    // =========================================================================

    #[test]
    fn sanitize_json_preserves_valid_json() {
        let valid = r#"{"key": "value", "num": 42}"#;
        assert_eq!(sanitize_json(valid), valid);
    }

    #[test]
    fn sanitize_json_escapes_literal_newlines_in_strings() {
        let broken = "{\"key\": \"line1\nline2\"}";
        let fixed = sanitize_json(broken);
        assert_eq!(fixed, r#"{"key": "line1\nline2"}"#);
    }

    #[test]
    fn sanitize_json_escapes_literal_tabs_in_strings() {
        let broken = "{\"key\": \"before\tafter\"}";
        let fixed = sanitize_json(broken);
        assert_eq!(fixed, r#"{"key": "before\tafter"}"#);
    }

    #[test]
    fn sanitize_json_does_not_escape_newlines_outside_strings() {
        let json = "{\n    \"key\": \"value\"\n}";
        let result = sanitize_json(json);
        assert_eq!(result, json); // Structural newlines preserved
    }

    #[test]
    fn sanitize_json_handles_escaped_quotes() {
        let json = r#"{"msg": "He said \"hello\""}"#;
        let result = sanitize_json(json);
        assert_eq!(result, json); // Already valid, no changes
    }

    #[test]
    fn sanitize_json_preserves_already_escaped_newlines() {
        let json = r#"{"key": "line1\nline2"}"#;
        let result = sanitize_json(json);
        assert_eq!(result, json); // Already escaped, no changes
    }

    // =========================================================================
    // normalize_escapes tests
    // =========================================================================

    #[test]
    fn normalize_escapes_converts_backslash_n() {
        assert_eq!(normalize_escapes("line1\\nline2"), "line1\nline2");
    }

    #[test]
    fn normalize_escapes_converts_backslash_t() {
        assert_eq!(normalize_escapes("before\\tafter"), "before\tafter");
    }

    #[test]
    fn normalize_escapes_mixed() {
        assert_eq!(
            normalize_escapes("first\\nsecond\\tthird"),
            "first\nsecond\tthird"
        );
    }

    #[test]
    fn normalize_escapes_no_escapes() {
        assert_eq!(normalize_escapes("plain text"), "plain text");
    }

    // =========================================================================
    // extract_explanation_field tests
    // =========================================================================

    #[test]
    fn extract_explanation_field_valid() {
        let text = r#"{"explanation": "This is the explanation", "other": "field"}"#;
        let result = extract_explanation_field(text);
        assert_eq!(result, Some("This is the explanation".to_string()));
    }

    #[test]
    fn extract_explanation_field_with_newlines() {
        let text = r#"{"explanation": "Line 1\nLine 2"}"#;
        let result = extract_explanation_field(text);
        assert_eq!(result, Some("Line 1\nLine 2".to_string()));
    }

    #[test]
    fn extract_explanation_field_with_escaped_quotes() {
        let text = r#"{"explanation": "He said \"hello\""}"#;
        let result = extract_explanation_field(text);
        assert_eq!(result, Some("He said \"hello\"".to_string()));
    }

    #[test]
    fn extract_explanation_field_missing() {
        let text = r#"{"other": "value"}"#;
        let result = extract_explanation_field(text);
        assert_eq!(result, None);
    }

    #[test]
    fn extract_explanation_field_with_spaces_around_colon() {
        let text = r#"{"explanation"  :  "spaced out"}"#;
        let result = extract_explanation_field(text);
        assert_eq!(result, Some("spaced out".to_string()));
    }

    // =========================================================================
    // Integration / edge case tests
    // =========================================================================

    #[test]
    fn empty_input_returns_fallback() {
        let result = parse_response("", AnalysisMode::TechExplain, "test");
        assert_eq!(result.original, "test");
        assert_eq!(result.corrected, "test");
        assert_eq!(result.explanation.as_deref(), Some(""));
    }

    #[test]
    fn whitespace_only_input_returns_fallback() {
        let result = parse_response("   \n\t  ", AnalysisMode::Improve, "original");
        assert_eq!(result.corrected, "");
        assert!(result.explanation.is_none());
    }

    #[test]
    fn explanation_with_literal_backslash_n_gets_normalized() {
        let json = r#"{"corrected": "test", "changes": [], "explanation": "First\\nSecond"}"#;

        let result = parse_response(json, AnalysisMode::TechExplain, "test");
        let explanation = result.explanation.unwrap();
        assert!(explanation.contains('\n'));
        assert!(explanation.contains("First"));
        assert!(explanation.contains("Second"));
    }

    #[test]
    fn tldr_with_literal_backslash_n_gets_normalized() {
        let json = r#"{"corrected": "test", "changes": [], "tldr": "Summary\\nDetails"}"#;

        let result = parse_response(json, AnalysisMode::TechExplain, "test");
        let tldr = result.tldr.unwrap();
        assert!(tldr.contains('\n'));
    }

    #[test]
    fn deeply_nested_json_with_surrounding_text() {
        let raw = r#"Sure! Here's the analysis:

{
    "corrected": "The quick brown fox",
    "changes": [
        {
            "original": "quik",
            "replacement": "quick",
            "reason": "Spelling correction"
        }
    ],
    "explanation": "Fixed spelling",
    "vocabulary": [
        {
            "word": "quick",
            "suggestion": "rapid",
            "definition": "Moving fast",
            "example": "A quick response",
            "level": "A2"
        }
    ]
}

Let me know if you need anything else!"#;

        let result = parse_response(raw, AnalysisMode::Improve, "The quik brown fox");
        assert_eq!(result.corrected, "The quick brown fox");
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.vocabulary.len(), 1);
    }

    #[test]
    fn resources_parsed_correctly() {
        let json = r#"{
            "corrected": "test",
            "changes": [],
            "explanation": "Explanation here",
            "resources": [
                {"title": "Rust Book", "url": "https://doc.rust-lang.org/book/"},
                {"title": "Tokio", "url": "https://tokio.rs"}
            ]
        }"#;

        let result = parse_response(json, AnalysisMode::TechExplain, "test");
        let resources = result.resources.unwrap();
        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].title, "Rust Book");
        assert_eq!(resources[1].url, "https://tokio.rs");
    }

    #[test]
    fn alternatives_with_array_pros_cons() {
        let json = r#"{
            "corrected": "test",
            "changes": [],
            "alternatives": [
                {
                    "name": "React",
                    "description": "UI library",
                    "pros": ["Virtual DOM", "Large ecosystem"],
                    "cons": ["JSX learning curve", "Frequent updates"]
                }
            ]
        }"#;

        let result = parse_response(json, AnalysisMode::TechExplain, "test");
        let alts = result.alternatives.unwrap();
        assert_eq!(alts[0].pros.len(), 2);
        assert_eq!(alts[0].cons.len(), 2);
    }

    // =========================================================================
    // Mode-specific field filtering tests
    // =========================================================================

    #[test]
    fn parse_strips_vocabulary_for_non_improve_mode() {
        let json = r#"{
            "corrected": "async/await",
            "changes": [],
            "explanation": "Async/await is a pattern",
            "vocabulary": [{"word": "async", "suggestion": "asynchronous", "definition": "Non-blocking", "example": "async fn", "level": "B2"}]
        }"#;

        let result = parse_response(json, AnalysisMode::TechExplain, "async/await");
        assert!(result.vocabulary.is_empty(), "vocabulary should be empty for TechExplain mode");
    }

    #[test]
    fn parse_strips_alternatives_for_non_tech_explain_mode() {
        let json = r#"{
            "corrected": "Hello world",
            "changes": [{"original": "wrold", "replacement": "world", "reason": "Typo"}],
            "explanation": "Fixed a typo",
            "alternatives": [{"name": "Alt1", "description": "Desc", "pros": ["Fast"], "cons": ["Complex"]}]
        }"#;

        let result = parse_response(json, AnalysisMode::Improve, "Hello wrold");
        assert!(result.alternatives.is_none(), "alternatives should be None for Improve mode");
    }

    // =========================================================================
    // BUG REPRODUCTION: Combined mode (levels) parsing
    // =========================================================================

    fn make_valid_combined_json() -> String {
        serde_json::json!({
            "tldr": "Bun is a fast JS runtime",
            "levels": {
                "eli5": "Bun is like a toy box",
                "eli15": "Bun is a fast alternative to Node.js",
                "professional": "Bun is built on JavaScriptCore engine",
                "samples": "### Basic\n```js\nBun.serve({ fetch(req) { return new Response('Hi'); } });\n```",
                "resources": "- Learn [[Node.js]] first\n- Understand [[TypeScript]]",
                "alternatives": "Consider [[Node.js]] or [[Deno]] instead."
            },
            "resources": [{"title": "Bun Docs", "url": "https://bun.sh/docs"}],
            "alternatives": [{"name": "Node.js", "description": "Classic runtime", "pros": ["Mature ecosystem"], "cons": ["Slower"]}]
        }).to_string()
    }



    #[test]
    fn combined_mode_valid_json_extracts_all_levels() {
        let json = make_valid_combined_json();
        let result = parse_response(&json, AnalysisMode::TechExplain, "bun");

        // All 6 levels should be present
        let levels = result.levels.as_ref().expect("levels should be Some");
        assert_eq!(levels.len(), 6, "should have all 6 levels");
        assert!(levels.contains_key("eli5"));
        assert!(levels.contains_key("eli15"));
        assert!(levels.contains_key("professional"));
        assert!(levels.contains_key("samples"));
        assert!(levels.contains_key("resources"));
        assert!(levels.contains_key("alternatives"));

        // Level content should be clean text, not contain JSON syntax
        let eli15 = &levels["eli15"];
        assert!(eli15.contains("Node.js"), "eli15 should contain explanation text");
        assert!(!eli15.contains("\"resources\""), "eli15 must NOT contain raw JSON keys");

        // Root-level resources and alternatives should be parsed separately
        assert_eq!(result.resources.as_ref().unwrap().len(), 1);
        assert_eq!(result.alternatives.as_ref().unwrap().len(), 1);
        assert_eq!(result.tldr.as_deref(), Some("Bun is a fast JS runtime"));
    }

    #[test]
    fn combined_mode_fallback_must_not_leak_raw_json_as_level_content() {
        // BUG SCENARIO: When JSON parsing completely fails, the fallback puts
        // entire raw JSON into explanation field. Frontend then creates
        // { eli15: raw_json_text } — user sees raw JSON in UI.
        //
        // Build malformed JSON at runtime: eli15 has a missing closing quote
        // and a literal newline, which breaks all parsing layers.
        let malformed = format!(
            "{}\n{{\n{}\n{}\n{}\n{}\n{}\n}}",
            "Here is the explanation:",
            "  \"tldr\": \"Fast runtime\",",
            "  \"levels\": {",
            "    \"eli5\": \"Simple box analogy\",",
            // eli15 value is broken: has literal newline + missing closing quote
            "    \"eli15\": \"Bun is a fast runtime\n  },",
            "  \"resources\": [{\"title\": \"Docs\", \"url\": \"https://bun.sh\"}]",
        );

        let result = parse_response(&malformed, AnalysisMode::TechExplain, "bun");

        // Even in fallback, explanation should not contain raw JSON array syntax
        if let Some(ref explanation) = result.explanation {
            assert!(
                !explanation.contains("\"resources\": ["),
                "Fallback explanation must not contain raw JSON array syntax"
            );
        }
    }

    #[test]
    fn combined_mode_with_code_in_samples_level() {
        // AI returns samples with code blocks containing braces
        let json = serde_json::json!({
            "tldr": "HTTP server",
            "levels": {
                "eli5": "Simple explanation",
                "eli15": "Mid-level explanation",
                "professional": "Detailed technical explanation",
                "samples": "### Basic\n```js\nBun.serve({ fetch(req) { return new Response(\"Hello\"); } });\n```\nThis starts an HTTP server.",
                "resources": "- Learn [[Node.js]]",
                "alternatives": "[[Deno]] is an alternative"
            },
            "resources": [{"title": "Bun Docs", "url": "https://bun.sh/docs"}],
            "alternatives": [{"name": "Deno", "description": "Alt runtime", "pros": ["Secure"], "cons": ["Smaller ecosystem"]}]
        }).to_string();

        let result = parse_response(&json, AnalysisMode::TechExplain, "bun");
        let levels = result.levels.as_ref().expect("levels should exist");

        // samples should contain the code block intact
        let samples = &levels["samples"];
        assert!(samples.contains("Bun.serve"), "samples should contain the code");
        assert!(samples.contains("Response"), "samples should preserve code content");

        // Other levels should not be contaminated
        let eli15 = &levels["eli15"];
        assert_eq!(eli15, "Mid-level explanation");
        assert!(!eli15.contains("resources"), "eli15 must not contain other level data");
    }

    #[test]
    fn combined_mode_ai_returns_levels_with_literal_newlines() {
        // AI returns JSON with literal newlines inside level string values
        // (common with Gemini models). sanitize_json should handle this.
        // Start with valid JSON, then inject literal newlines inside string values
        let valid = serde_json::json!({
            "tldr": "Summary",
            "levels": {
                "eli5": "First line SPLIT Second line",
                "eli15": "Explanation SPLIT Continued"
            }
        }).to_string();
        // Replace SPLIT with actual newline characters (simulating AI output with literal newlines)
        let json = valid.replace("SPLIT", "\n");

        let result = parse_response(&json, AnalysisMode::TechExplain, "test");
        let levels = result.levels.as_ref().expect("levels should be parsed");
        assert!(levels.contains_key("eli5"), "eli5 should be present");
        assert!(levels.contains_key("eli15"), "eli15 should be present");

        let eli5 = &levels["eli5"];
        assert!(eli5.contains("First"), "should contain first line");
        assert!(eli5.contains("Second"), "should contain second line");
    }

    #[test]
    fn combined_mode_resources_name_collision_handled() {
        // AI returns "resources" both as a level string AND as a root-level array.
        // Parser must keep them separate: level string in levels, array in resources field.
        let json = serde_json::json!({
            "tldr": "Test term",
            "levels": {
                "eli5": "Simple",
                "eli15": "Medium",
                "professional": "Detailed",
                "samples": "```js\nconsole.log('hi');\n```",
                "resources": "- Learn [[Node.js]] first\n- Understand [[V8]] engine",
                "alternatives": "Use [[Deno]] instead"
            },
            "resources": [
                {"title": "Official Docs", "url": "https://example.com"}
            ],
            "alternatives": [
                {"name": "Alt", "description": "Desc", "pros": ["Pro1"], "cons": ["Con1"]}
            ]
        }).to_string();

        let result = parse_response(&json, AnalysisMode::TechExplain, "test");
        let levels = result.levels.as_ref().expect("levels should exist");

        // The "resources" level should be the string content
        let res_level = &levels["resources"];
        assert!(res_level.contains("Node.js"), "resources level should be the learning roadmap string");
        assert!(!res_level.contains("https://"), "resources level should NOT contain URL from the array");

        // The root-level resources should be the array
        let resources = result.resources.as_ref().expect("root resources should exist");
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].title, "Official Docs");

        // Same for alternatives
        let alt_level = &levels["alternatives"];
        assert!(alt_level.contains("Deno"), "alternatives level should be context text");
        let alts = result.alternatives.as_ref().expect("root alternatives should exist");
        assert_eq!(alts.len(), 1);
    }
}
