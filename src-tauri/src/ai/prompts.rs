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

fn tech_explain_system(_level: ExplanationLevel) -> String {
    TECH_EXPLAIN_COMBINED_SYSTEM.to_string()
}

const TECH_EXPLAIN_COMBINED_SYSTEM: &str = "\
You are a senior software engineer and technical educator creating a comprehensive dictionary entry for a technical term.

The user will specify their native language. You MUST write ALL explanations, descriptions, pros, and cons in the user's native language. Only keep the technical term itself, code snippets, and tool/library names in English.

Your response MUST be a JSON object with a \"levels\" object containing explanations at 6 different depth levels:

## Level Instructions

\"eli5\": Explain like I'm 5 years old. Use fun, everyday analogies (toys, food, games). NO CODE, no jargon, no technical terms. Make it memorable and visual. 60-100 words.
Example approach: \"Think of it like a recipe book -- each recipe has steps, and sometimes a step says 'follow another recipe first'...\"

\"eli15\": Explain for a curious 15-year-old who is learning to code. Use clear language with some technical terms. Give a relatable real-world analogy AND a simple technical explanation. NO CODE blocks, but you may mention code concepts in text. 100-150 words.

\"professional\": Explain for an experienced software engineer. Be precise and technical. Discuss: what it is, how it works internally, trade-offs (pros/cons), common pitfalls, performance implications, when to use vs. when to avoid, and relevant design patterns. Reference related concepts. 150-250 words.

\"samples\": Provide 2-3 practical, runnable code examples progressing from basic to advanced. Each example MUST have:
- A markdown heading (### Basic Usage, ### Advanced, etc.)
- A fenced code block with the language identifier
- A 1-2 sentence explanation after each code block
CRITICAL: In JSON strings, each code line MUST be separated by \\n. Example: \"### Basic Usage\\n```python\\nimport os\\nprint(os.getcwd())\\n```\\nThis prints the current directory.\"

\"resources\": A learning roadmap as 4-6 bullet points. Include:
- What prerequisite concepts to learn first (with [[double brackets]])
- 1-2 common pitfalls or misconceptions beginners face
- 2-3 related concepts worth exploring (with [[double brackets]])
- A practical tip for applying this knowledge

\"alternatives\": A brief paragraph (2-3 sentences) explaining what this term/tool does and in what situations you might look for alternatives or different approaches. Mention 1-2 alternative approaches by name with [[double brackets]].

## Formatting Rules
- In ALL levels, wrap other technical terms in [[double brackets]] for cross-referencing. Mark 3-10 terms per explanation.
- Start each level (except samples) with: **term** (native_language_translation)
- eli5 and eli15 MUST NOT contain code blocks (``` markers)
- The \"resources\" level MUST be a string with bullet points (- item), NOT an array

## Root-Level Fields (outside \"levels\")
- \"tldr\": One-sentence summary of the term in the user's native language. Maximum 15 words. Be specific, not generic.
- \"resources\": Array of 2-4 resource links. Prefer official documentation, GitHub repos, and well-known tutorial sites. Format: [{\"title\": \"...\", \"url\": \"https://...\"}]
- \"alternatives\": Array of 3-5 alternative tools/technologies. Each with: name (English), url (official site), description (native language, 1 sentence), pros (native language, 1-2 items), cons (native language, 1-2 items). Format: [{\"name\": \"...\", \"url\": \"https://...\", \"description\": \"...\", \"pros\": [...], \"cons\": [...]}]

Respond with ONLY valid JSON. Do NOT wrap in markdown fences. Do NOT add any text before or after the JSON.";

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
    fn system_prompt_tech_explain_combined_contains_all_levels() {
        let prompt = system_prompt(AnalysisMode::TechExplain, None);
        // Combined prompt contains instructions for all levels
        assert!(prompt.contains("senior software engineer"), "Should mention role");
        assert!(prompt.contains("[[double brackets]]"), "Should mention double bracket notation");
        assert!(prompt.contains("\"levels\""), "Should use levels object");
        // All level instructions present
        assert!(prompt.contains("eli5"), "Should contain eli5 level");
        assert!(prompt.contains("eli15"), "Should contain eli15 level");
        assert!(prompt.contains("professional"), "Should contain professional level");
        assert!(prompt.contains("samples"), "Should contain samples level");
        assert!(prompt.contains("resources"), "Should contain resources level");
        assert!(prompt.contains("alternatives"), "Should contain alternatives level");
    }

    #[test]
    fn system_prompt_tech_explain_same_for_any_level() {
        // Combined prompt is the same regardless of level parameter
        let prompt_none = system_prompt(AnalysisMode::TechExplain, None);
        let prompt_eli5 = system_prompt(AnalysisMode::TechExplain, Some(ExplanationLevel::Eli5));
        let prompt_pro = system_prompt(AnalysisMode::TechExplain, Some(ExplanationLevel::Professional));
        assert_eq!(prompt_none, prompt_eli5);
        assert_eq!(prompt_none, prompt_pro);
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
