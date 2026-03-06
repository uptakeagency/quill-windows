use crate::models::{AnalysisMode, ExplanationLevel};
use crate::models::analysis::ToneStyle;

/// Generates the system prompt for the AI based on the analysis mode and optional explanation level.
pub fn system_prompt(mode: AnalysisMode, level: Option<ExplanationLevel>) -> String {
    match mode {
        AnalysisMode::Improve => IMPROVE_SYSTEM.to_string(),
        AnalysisMode::Translate => TRANSLATE_SYSTEM.to_string(),
        AnalysisMode::TechExplain => {
            tech_explain_system(level.unwrap_or(ExplanationLevel::Eli15))
        }
    }
}

/// Generates the user prompt for the AI based on the analysis mode and parameters.
pub fn user_prompt(
    mode: AnalysisMode,
    text: &str,
    tone: Option<&ToneStyle>,
    context: Option<&str>,
    native_language: Option<&str>,
    target_language: Option<&str>,
) -> String {
    let context_block = match context {
        Some(ctx) if !ctx.is_empty() && ctx != text => {
            let trimmed = if ctx.chars().count() > 1000 {
                format!("{}...", ctx.chars().take(1000).collect::<String>())
            } else {
                ctx.to_string()
            };
            format!(
                "\n\nSurrounding context (use this to make better, context-aware suggestions):\n\"\"\"\n{}\n\"\"\"",
                trimmed
            )
        }
        _ => String::new(),
    };

    match mode {
        AnalysisMode::Improve => {
            let tone_instruction = match tone {
                Some(t) => format!(" Also adjust the tone to be {}.", tone_label(t)),
                None => String::new(),
            };
            format!(
                "Improve the following text \u{2014} fix any grammar, spelling, or punctuation errors and improve clarity and readability.{}\n\n{}{}",
                tone_instruction, text, context_block
            )
        }
        AnalysisMode::Translate => {
            let native = native_language.unwrap_or("English");
            let target = target_language.unwrap_or("English");
            format!(
                "My native language is {}. My target language is {}.\nAuto-detect and translate the following text:\n\n{}{}",
                native, target, text, context_block
            )
        }
        AnalysisMode::TechExplain => {
            let native = native_language.unwrap_or("English");
            format!(
                "My native language is {}. You MUST write your entire explanation in {}. Explain the following technical term or code:\n\n{}{}",
                native, native, text, context_block
            )
        }
    }
}

// --- Helper ---

fn tone_label(tone: &ToneStyle) -> &'static str {
    match tone {
        ToneStyle::Formal => "formal",
        ToneStyle::Casual => "casual",
        ToneStyle::Professional => "professional",
        ToneStyle::Friendly => "friendly",
    }
}

// --- System Prompts ---

const IMPROVE_SYSTEM: &str = "\
You are an expert editor, proofreader, and vocabulary coach. Your job is to:
1. Fix all grammar, spelling, and punctuation errors
2. Improve clarity, flow, and readability
3. If a tone is requested, adjust the text to match that tone
4. Suggest 2-3 richer vocabulary alternatives for key words in the text

Preserve the original meaning. Make minimal changes when the text is already good.
Respond ONLY with valid JSON in this exact format:
{
  \"corrected\": \"the improved text\",
  \"changes\": [
    {\"original\": \"original phrase\", \"replacement\": \"improved phrase\", \"reason\": \"brief explanation\"}
  ],
  \"vocabulary\": [
    {
      \"word\": \"word from the text\",
      \"suggestion\": \"richer/more precise alternative\",
      \"definition\": \"clear definition of the suggested word\",
      \"example\": \"example sentence using the suggested word\",
      \"level\": \"CEFR level (B1/B2/C1/C2)\"
    }
  ]
}
If no changes needed, return the original text as \"corrected\" with empty \"changes\" array.
Include 2-3 vocabulary suggestions for words that have more expressive alternatives. Skip vocabulary if the text is very short (1-2 words).
Do not add any text outside the JSON.";

const TRANSLATE_SYSTEM: &str = "\
You are a bilingual translation assistant.
The user has a native language and a target language. Auto-detect the language of the given text:
- If the text is in the native language \u{2192} translate it to the target language.
- If the text is in the target language \u{2192} translate it to the native language.
- If the text is in a third language \u{2192} translate it to the native language.

CRITICAL: The \"explanation\" field MUST contain ONLY the direct, complete translation. Nothing else. No commentary, no notes, no analysis. Just the translated text.

Put any brief translation notes (idioms, nuances) in the \"changes\" array only if there are notable choices.

Respond ONLY with valid JSON in this exact format:
{
  \"corrected\": \"the translated text (same as explanation)\",
  \"changes\": [
    {\"original\": \"source phrase\", \"replacement\": \"translated phrase\", \"reason\": \"brief note in native language\"}
  ],
  \"explanation\": \"The direct, complete translation. ONLY the translation, nothing else.\"
}
Do not add any text outside the JSON.";

fn tech_explain_system(level: ExplanationLevel) -> String {
    match level {
        ExplanationLevel::Alternatives => TECH_EXPLAIN_ALTERNATIVES_SYSTEM.to_string(),
        ExplanationLevel::Samples => TECH_EXPLAIN_SAMPLES_SYSTEM.to_string(),
        _ => format!(
            "\
You are a senior software engineer explaining technical terms, commands, and concepts.
The user will specify their native language. Write your ENTIRE explanation in the user's native language. Only keep the technical term itself and code snippets in English.
Start your explanation with the term followed by its native language translation in parentheses, e.g. \"**database** (veritaban\u{0131})\".

Explanation style: {}

CRITICAL: Keep the explanation concise \u{2014} maximum 150 words. Be brief and to the point.

Cover:
1. What it is (1-2 sentences)
2. How it's used (1-2 sentences)
3. A quick example
4. Related concepts

IMPORTANT: When you mention other technical terms in your explanation, wrap them in [[double brackets]]. For example: \"Bir [[REST API]], bir [[server]] ile ileti\u{015f}im kurmak i\u{00e7}in [[HTTP]] methodlar\u{0131}n\u{0131} kullan\u{0131}r.\" Mark 3-8 terms per explanation. Only mark terms that would benefit from their own explanation.

Also include 2-4 resource links: official documentation, tutorials, or authoritative references for this term. Prefer official sites (e.g. python.org for Python, developer.mozilla.org for web APIs, docs.docker.com for Docker). Use well-known, stable URLs only.

Respond ONLY with valid JSON in this exact format (no markdown fences, no extra text):
{{
  \"corrected\": \"the original term unchanged\",
  \"changes\": [],
  \"tldr\": \"One-sentence summary of what this term means, in the user's native language. Maximum 15 words.\",
  \"explanation\": \"**term** (native translation)\\n\\nExplanation in the user's native language with [[technical terms]] in double brackets.\",
  \"resources\": [{{\"title\": \"Official Docs\", \"url\": \"https://example.com/docs\"}}, {{\"title\": \"Tutorial\", \"url\": \"https://example.com/tutorial\"}}]
}}",
            level.prompt_instruction()
        ),
    }
}

const TECH_EXPLAIN_SAMPLES_SYSTEM: &str = "\
You are a senior software engineer providing practical code examples.
The user will specify their native language. Write comments and explanations in the user's native language. Keep code in English.

Provide 2-3 practical, runnable code examples showing how this term/concept is used in real code.
Go from simple to advanced. Each example should have:
- A short heading (e.g. \"### Basic Usage\", \"### Advanced: With Error Handling\")
- A code block with the snippet
- A 1-2 sentence explanation of what the code does, in the user's native language

IMPORTANT: When you mention other technical terms, wrap them in [[double brackets]].

Respond ONLY with valid JSON in this exact format (no markdown fences, no extra text):
{
  \"corrected\": \"the original term unchanged\",
  \"changes\": [],
  \"tldr\": \"One-sentence summary of what this term means, in the user's native language. Maximum 15 words.\",
  \"explanation\": \"**term** (native translation)\\n\\n### Basic Usage\\n```language\\ncode here\\n```\\nExplanation of this example.\\n\\n### Advanced\\n```language\\nmore code\\n```\\nExplanation.\",
  \"resources\": []
}";

const TECH_EXPLAIN_ALTERNATIVES_SYSTEM: &str = "\
You are a senior software engineer comparing technical tools and technologies.
The user will specify their native language. Write ALL descriptions, pros, and cons in the user's native language. Keep tool/library names in English.

For the given term, list 3-5 alternatives or competitors. For each alternative provide:
- name: The tool/library/technology name (English)
- description: One-line description of what it is (native language)
- pros: 1-2 key advantages (native language)
- cons: 1-2 key disadvantages (native language)

Also include a brief explanation of the original term for context.

IMPORTANT: When you mention other technical terms in your explanation, wrap them in [[double brackets]].

Respond ONLY with valid JSON in this exact format (no markdown fences, no extra text):
{
  \"corrected\": \"the original term unchanged\",
  \"changes\": [],
  \"tldr\": \"One-sentence summary of what this term means, in the user's native language. Maximum 15 words.\",
  \"explanation\": \"**term** (native translation) \u{2014} brief context about what this tool does and why you might look for alternatives.\",
  \"resources\": [],
  \"alternatives\": [
    {\"name\": \"AlternativeName\", \"description\": \"What it is in native language\", \"pros\": [\"Key advantage 1\", \"Key advantage 2\"], \"cons\": [\"Key disadvantage 1\"]}
  ]
}";

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // system_prompt tests
    // ========================================================================

    #[test]
    fn system_prompt_improve_contains_editor_role() {
        let prompt = system_prompt(AnalysisMode::Improve, None);
        assert!(prompt.contains("expert editor"), "Improve system prompt should mention 'expert editor'");
        assert!(prompt.contains("grammar"), "Improve system prompt should mention 'grammar'");
        assert!(prompt.contains("JSON"), "Improve system prompt should require JSON response");
    }

    #[test]
    fn system_prompt_translate_contains_bilingual() {
        let prompt = system_prompt(AnalysisMode::Translate, None);
        assert!(prompt.contains("bilingual translation"), "Translate system prompt should mention 'bilingual translation'");
        assert!(prompt.contains("native language"), "Translate system prompt should mention 'native language'");
        assert!(prompt.contains("JSON"), "Translate system prompt should require JSON response");
    }

    #[test]
    fn system_prompt_tech_explain_default_level() {
        let prompt = system_prompt(AnalysisMode::TechExplain, None);
        // Default level is Eli15
        assert!(prompt.contains("senior software engineer"), "TechExplain system prompt should mention 'senior software engineer'");
        assert!(prompt.contains(ExplanationLevel::Eli15.prompt_instruction()));
        assert!(prompt.contains("[[double brackets]]"), "Should mention double bracket notation");
    }

    #[test]
    fn system_prompt_tech_explain_eli5() {
        let prompt = system_prompt(AnalysisMode::TechExplain, Some(ExplanationLevel::Eli5));
        assert!(prompt.contains(ExplanationLevel::Eli5.prompt_instruction()));
        assert!(prompt.contains("senior software engineer"));
    }

    #[test]
    fn system_prompt_tech_explain_professional() {
        let prompt = system_prompt(AnalysisMode::TechExplain, Some(ExplanationLevel::Professional));
        assert!(prompt.contains(ExplanationLevel::Professional.prompt_instruction()));
    }

    #[test]
    fn system_prompt_tech_explain_samples_uses_dedicated_prompt() {
        let prompt = system_prompt(AnalysisMode::TechExplain, Some(ExplanationLevel::Samples));
        // Samples has its own dedicated system prompt, NOT the default template
        assert!(prompt.contains("practical code examples"), "Samples should use dedicated prompt with 'practical code examples'");
        // Should NOT contain the default template's "maximum 150 words"
        assert!(!prompt.contains("maximum 150 words"), "Samples should NOT use the default template");
    }

    #[test]
    fn system_prompt_tech_explain_alternatives_uses_dedicated_prompt() {
        let prompt = system_prompt(AnalysisMode::TechExplain, Some(ExplanationLevel::Alternatives));
        // Alternatives has its own dedicated system prompt
        assert!(prompt.contains("comparing technical tools"), "Alternatives should use dedicated prompt with 'comparing technical tools'");
        // Should NOT contain the default template's "maximum 150 words"
        assert!(!prompt.contains("maximum 150 words"), "Alternatives should NOT use the default template");
    }

    #[test]
    fn system_prompt_tech_explain_resources_uses_default_template() {
        let prompt = system_prompt(AnalysisMode::TechExplain, Some(ExplanationLevel::Resources));
        // Resources uses the default template (not a dedicated prompt)
        assert!(prompt.contains("senior software engineer"));
        assert!(prompt.contains(ExplanationLevel::Resources.prompt_instruction()));
        assert!(prompt.contains("maximum 150 words"));
    }

    #[test]
    fn system_prompt_improve_contains_vocabulary_section() {
        let prompt = system_prompt(AnalysisMode::Improve, None);
        assert!(prompt.contains("vocabulary"), "Improve system prompt should mention vocabulary suggestions");
        assert!(prompt.contains("CEFR"), "Improve system prompt should mention CEFR levels");
    }

    #[test]
    fn system_prompt_translate_requires_direct_translation() {
        let prompt = system_prompt(AnalysisMode::Translate, None);
        assert!(prompt.contains("\"explanation\""), "Translate system prompt should include explanation field in JSON format");
        assert!(prompt.contains("ONLY the direct, complete translation"), "Translate should require direct translation in explanation field");
    }

    // ========================================================================
    // user_prompt tests
    // ========================================================================

    #[test]
    fn user_prompt_improve_without_tone() {
        let prompt = user_prompt(
            AnalysisMode::Improve,
            "Hello wrold",
            None,
            None,
            None,
            None,
        );
        assert!(prompt.contains("Improve the following text"));
        assert!(prompt.contains("Hello wrold"));
        // No tone instruction when tone is None
        assert!(!prompt.contains("adjust the tone"));
    }

    #[test]
    fn user_prompt_improve_with_tone() {
        let prompt = user_prompt(
            AnalysisMode::Improve,
            "Hello wrold",
            Some(&ToneStyle::Formal),
            None,
            None,
            None,
        );
        assert!(prompt.contains("Improve the following text"));
        assert!(prompt.contains("Hello wrold"));
        assert!(prompt.contains("adjust the tone to be formal"), "Should include tone instruction with 'formal'");
    }

    #[test]
    fn user_prompt_improve_with_casual_tone() {
        let prompt = user_prompt(
            AnalysisMode::Improve,
            "some text",
            Some(&ToneStyle::Casual),
            None,
            None,
            None,
        );
        assert!(prompt.contains("adjust the tone to be casual"));
    }

    #[test]
    fn user_prompt_translate_with_languages() {
        let prompt = user_prompt(
            AnalysisMode::Translate,
            "Merhaba",
            None,
            None,
            Some("Turkish"),
            Some("English"),
        );
        assert!(prompt.contains("My native language is Turkish"));
        assert!(prompt.contains("My target language is English"));
        assert!(prompt.contains("Merhaba"));
    }

    #[test]
    fn user_prompt_translate_default_language_is_english() {
        let prompt = user_prompt(
            AnalysisMode::Translate,
            "Bonjour",
            None,
            None,
            None,
            None,
        );
        assert!(prompt.contains("My native language is English"));
        assert!(prompt.contains("My target language is English"));
    }

    #[test]
    fn user_prompt_tech_explain_with_native_language() {
        let prompt = user_prompt(
            AnalysisMode::TechExplain,
            "async/await",
            None,
            None,
            Some("Turkish"),
            None,
        );
        assert!(prompt.contains("My native language is Turkish"));
        assert!(prompt.contains("MUST write your entire explanation in Turkish"));
        assert!(prompt.contains("async/await"));
    }

    #[test]
    fn user_prompt_tech_explain_default_language() {
        let prompt = user_prompt(
            AnalysisMode::TechExplain,
            "Docker",
            None,
            None,
            None,
            None,
        );
        assert!(prompt.contains("My native language is English"));
        assert!(prompt.contains("MUST write your entire explanation in English"));
    }

    #[test]
    fn user_prompt_context_included_when_different_from_text() {
        let prompt = user_prompt(
            AnalysisMode::Improve,
            "Hello wrold",
            None,
            Some("This is a greeting. Hello wrold. Nice to meet you."),
            None,
            None,
        );
        assert!(prompt.contains("Surrounding context"));
        assert!(prompt.contains("This is a greeting"));
    }

    #[test]
    fn user_prompt_context_excluded_when_same_as_text() {
        let prompt = user_prompt(
            AnalysisMode::Improve,
            "Hello wrold",
            None,
            Some("Hello wrold"),
            None,
            None,
        );
        assert!(!prompt.contains("Surrounding context"), "Context should not be included when it equals the text");
    }

    #[test]
    fn user_prompt_context_excluded_when_empty() {
        let prompt = user_prompt(
            AnalysisMode::Improve,
            "Hello wrold",
            None,
            Some(""),
            None,
            None,
        );
        assert!(!prompt.contains("Surrounding context"), "Context should not be included when empty");
    }

    #[test]
    fn user_prompt_context_excluded_when_none() {
        let prompt = user_prompt(
            AnalysisMode::Improve,
            "Hello wrold",
            None,
            None,
            None,
            None,
        );
        assert!(!prompt.contains("Surrounding context"), "Context should not be included when None");
    }

    #[test]
    fn user_prompt_context_truncated_at_1000_chars() {
        let long_context = "a".repeat(1500);
        let prompt = user_prompt(
            AnalysisMode::Improve,
            "Hello",
            None,
            Some(&long_context),
            None,
            None,
        );
        assert!(prompt.contains("Surrounding context"));
        // The context should be truncated to 1000 chars + "..."
        assert!(prompt.contains("..."), "Long context should be truncated with '...'");
        // Should NOT contain the full 1500 char string
        assert!(!prompt.contains(&long_context), "Should not contain the full untruncated context");
    }

    #[test]
    fn user_prompt_context_not_truncated_at_999_chars() {
        let short_context = "b".repeat(999);
        let prompt = user_prompt(
            AnalysisMode::TechExplain,
            "Docker",
            None,
            Some(&short_context),
            Some("English"),
            None,
        );
        assert!(prompt.contains("Surrounding context"));
        assert!(prompt.contains(&short_context), "Context under 1000 chars should not be truncated");
        // Check that '...' is not appended
        assert!(!prompt.contains(&format!("{}...", short_context)));
    }

    #[test]
    fn user_prompt_all_tone_styles() {
        let tones = [
            (ToneStyle::Formal, "formal"),
            (ToneStyle::Casual, "casual"),
            (ToneStyle::Professional, "professional"),
            (ToneStyle::Friendly, "friendly"),
        ];

        for (tone, expected_str) in &tones {
            let prompt = user_prompt(
                AnalysisMode::Improve,
                "test",
                Some(tone),
                None,
                None,
                None,
            );
            assert!(
                prompt.contains(&format!("adjust the tone to be {}", expected_str)),
                "Tone {:?} should produce '{}'",
                tone,
                expected_str
            );
        }
    }
}
