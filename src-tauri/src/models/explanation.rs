use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::analysis::{Alternative, ResourceLink};

// --- ExplanationLevel ---

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExplanationLevel {
    Eli5,
    Eli15,
    Professional,
    Samples,
    Resources,
    Alternatives,
}

impl Default for ExplanationLevel {
    fn default() -> Self {
        Self::Eli15
    }
}

impl ExplanationLevel {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Eli5 => "ELI5",
            Self::Eli15 => "ELI15",
            Self::Professional => "Pro",
            Self::Samples => "Samples",
            Self::Resources => "Resources",
            Self::Alternatives => "Alts",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Eli5 => "\u{1F476}",           // baby
            Self::Eli15 => "\u{1F393}",           // graduation cap
            Self::Professional => "\u{1F4BC}",    // briefcase
            Self::Samples => "\u{1F4BB}",         // laptop
            Self::Resources => "\u{1F4DA}",       // books
            Self::Alternatives => "\u{1F500}",    // shuffle
        }
    }

    pub fn prompt_instruction(&self) -> &'static str {
        match self {
            Self::Eli5 => "Explain like I'm 5 years old. Use very simple words, fun analogies, and everyday examples. Avoid jargon completely.",
            Self::Eli15 => "Explain for a 15-year-old learning to code. Use clear language with some technical terms, relatable examples, and brief code snippets when helpful.",
            Self::Professional => "Explain for an experienced developer. Be precise, use proper terminology, discuss trade-offs, edge cases, and mention relevant design patterns or alternatives.",
            Self::Samples => "Provide 2-3 practical code examples showing how this term/concept is used in real code. Each example should be a short, runnable snippet with a one-line comment explaining what it does. Focus on common use cases from simple to advanced.",
            Self::Resources => "Be very concise. In 3-5 bullet points, list: what to learn first, one common pitfall, and 2-3 related concepts. Keep each bullet to one sentence. Do NOT write long paragraphs.",
            Self::Alternatives => "Focus ONLY on alternatives and competitors for this term. List 3-5 alternatives, each with a one-line description, 1-2 pros, and 1-2 cons. Write the description, pros, and cons in the user's native language.",
        }
    }
}

// --- TechExplanation ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechExplanation {
    pub term: String,
    pub level: ExplanationLevel,
    pub explanation: String,
    pub tldr: Option<String>,
    pub resources: Option<Vec<ResourceLink>>,
    pub alternatives: Option<Vec<Alternative>>,
}

// --- TechDictionaryState ---

#[derive(Debug)]
pub struct TechDictionaryState {
    stack: Vec<TechExplanation>,
    cache: HashMap<(String, ExplanationLevel), TechExplanation>,
}

impl TechDictionaryState {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            cache: HashMap::new(),
        }
    }

    pub fn current(&self) -> Option<&TechExplanation> {
        self.stack.last()
    }

    pub fn breadcrumbs(&self) -> Vec<&str> {
        self.stack.iter().map(|e| e.term.as_str()).collect()
    }

    pub fn push(&mut self, explanation: TechExplanation) {
        let key = (explanation.term.clone(), explanation.level.clone());
        self.cache.insert(key, explanation.clone());
        self.stack.push(explanation);
    }

    pub fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    pub fn pop_to(&mut self, index: usize) {
        if index < self.stack.len() {
            self.stack.truncate(index + 1);
        }
    }

    pub fn replace_top(&mut self, explanation: TechExplanation) {
        if self.stack.is_empty() {
            self.push(explanation);
            return;
        }
        let key = (explanation.term.clone(), explanation.level.clone());
        self.cache.insert(key, explanation.clone());
        let last = self.stack.len() - 1;
        self.stack[last] = explanation;
    }

    pub fn cached(&self, term: &str, level: &ExplanationLevel) -> Option<&TechExplanation> {
        self.cache.get(&(term.to_string(), level.clone()))
    }

    pub fn reset(&mut self) {
        self.stack.clear();
    }

    pub fn cache_count(&self) -> usize {
        self.cache.len()
    }

    pub fn clear_cache(&mut self) {
        self.stack.clear();
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ExplanationLevel tests ---

    #[test]
    fn default_level_is_eli15() {
        let level = ExplanationLevel::default();
        assert_eq!(level, ExplanationLevel::Eli15);
    }

    #[test]
    fn level_serializes_camel_case() {
        assert_eq!(
            serde_json::to_string(&ExplanationLevel::Eli5).unwrap(),
            "\"eli5\""
        );
        assert_eq!(
            serde_json::to_string(&ExplanationLevel::Eli15).unwrap(),
            "\"eli15\""
        );
        assert_eq!(
            serde_json::to_string(&ExplanationLevel::Professional).unwrap(),
            "\"professional\""
        );
        assert_eq!(
            serde_json::to_string(&ExplanationLevel::Samples).unwrap(),
            "\"samples\""
        );
        assert_eq!(
            serde_json::to_string(&ExplanationLevel::Resources).unwrap(),
            "\"resources\""
        );
        assert_eq!(
            serde_json::to_string(&ExplanationLevel::Alternatives).unwrap(),
            "\"alternatives\""
        );
    }

    #[test]
    fn level_deserializes_from_camel_case() {
        let level: ExplanationLevel = serde_json::from_str("\"eli5\"").unwrap();
        assert_eq!(level, ExplanationLevel::Eli5);

        let level: ExplanationLevel = serde_json::from_str("\"professional\"").unwrap();
        assert_eq!(level, ExplanationLevel::Professional);
    }

    #[test]
    fn level_title_returns_correct_string() {
        assert_eq!(ExplanationLevel::Eli5.title(), "ELI5");
        assert_eq!(ExplanationLevel::Eli15.title(), "ELI15");
        assert_eq!(ExplanationLevel::Professional.title(), "Pro");
        assert_eq!(ExplanationLevel::Samples.title(), "Samples");
        assert_eq!(ExplanationLevel::Resources.title(), "Resources");
        assert_eq!(ExplanationLevel::Alternatives.title(), "Alts");
    }

    #[test]
    fn level_icon_returns_emoji() {
        // Each level should return a non-empty emoji string
        for level in &[
            ExplanationLevel::Eli5,
            ExplanationLevel::Eli15,
            ExplanationLevel::Professional,
            ExplanationLevel::Samples,
            ExplanationLevel::Resources,
            ExplanationLevel::Alternatives,
        ] {
            assert!(!level.icon().is_empty(), "icon for {:?} should not be empty", level);
        }
    }

    #[test]
    fn prompt_instruction_contains_keywords() {
        assert!(ExplanationLevel::Eli5.prompt_instruction().contains("5 years old"));
        assert!(ExplanationLevel::Eli15.prompt_instruction().contains("15-year-old"));
        assert!(ExplanationLevel::Professional.prompt_instruction().contains("experienced developer"));
        assert!(ExplanationLevel::Samples.prompt_instruction().contains("code examples"));
        assert!(ExplanationLevel::Resources.prompt_instruction().contains("concise"));
        assert!(ExplanationLevel::Alternatives.prompt_instruction().contains("alternatives"));
    }

    // --- TechDictionaryState tests ---

    fn make_explanation(term: &str, level: ExplanationLevel) -> TechExplanation {
        TechExplanation {
            term: term.to_string(),
            level,
            explanation: format!("Explanation of {}", term),
            tldr: Some(format!("TL;DR: {}", term)),
            resources: None,
            alternatives: None,
        }
    }

    #[test]
    fn drill_down_push_pop() {
        let mut state = TechDictionaryState::new();
        let ex1 = make_explanation("Rust", ExplanationLevel::Eli15);
        let ex2 = make_explanation("Ownership", ExplanationLevel::Eli15);

        state.push(ex1);
        assert_eq!(state.current().unwrap().term, "Rust");

        state.push(ex2);
        assert_eq!(state.current().unwrap().term, "Ownership");

        state.pop();
        assert_eq!(state.current().unwrap().term, "Rust");
    }

    #[test]
    fn cannot_pop_last_item() {
        let mut state = TechDictionaryState::new();
        state.push(make_explanation("Rust", ExplanationLevel::Eli15));

        state.pop(); // should not pop the last item
        assert_eq!(state.current().unwrap().term, "Rust");
    }

    #[test]
    fn pop_to_truncates_stack() {
        let mut state = TechDictionaryState::new();
        state.push(make_explanation("A", ExplanationLevel::Eli15));
        state.push(make_explanation("B", ExplanationLevel::Eli15));
        state.push(make_explanation("C", ExplanationLevel::Eli15));

        state.pop_to(0); // truncate to just "A"
        assert_eq!(state.current().unwrap().term, "A");
        assert_eq!(state.breadcrumbs(), vec!["A"]);
    }

    #[test]
    fn cache_hit_after_push() {
        let mut state = TechDictionaryState::new();
        let ex = make_explanation("Rust", ExplanationLevel::Eli15);
        state.push(ex.clone());

        let cached = state.cached("Rust", &ExplanationLevel::Eli15);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().term, "Rust");
    }

    #[test]
    fn cache_miss_for_different_level() {
        let mut state = TechDictionaryState::new();
        state.push(make_explanation("Rust", ExplanationLevel::Eli15));

        let cached = state.cached("Rust", &ExplanationLevel::Professional);
        assert!(cached.is_none());
    }

    #[test]
    fn replace_top_for_level_switch() {
        let mut state = TechDictionaryState::new();
        state.push(make_explanation("Rust", ExplanationLevel::Eli15));

        let pro_ex = TechExplanation {
            term: "Rust".to_string(),
            level: ExplanationLevel::Professional,
            explanation: "Pro explanation of Rust".to_string(),
            tldr: None,
            resources: None,
            alternatives: None,
        };
        state.replace_top(pro_ex);

        assert_eq!(state.current().unwrap().level, ExplanationLevel::Professional);
        assert_eq!(state.current().unwrap().explanation, "Pro explanation of Rust");
        // Both levels should be cached
        assert!(state.cached("Rust", &ExplanationLevel::Eli15).is_some());
        assert!(state.cached("Rust", &ExplanationLevel::Professional).is_some());
    }

    #[test]
    fn breadcrumbs_returns_term_names() {
        let mut state = TechDictionaryState::new();
        state.push(make_explanation("Rust", ExplanationLevel::Eli15));
        state.push(make_explanation("Ownership", ExplanationLevel::Eli15));
        state.push(make_explanation("Borrow Checker", ExplanationLevel::Eli15));

        assert_eq!(
            state.breadcrumbs(),
            vec!["Rust", "Ownership", "Borrow Checker"]
        );
    }

    #[test]
    fn reset_clears_stack_but_not_cache() {
        let mut state = TechDictionaryState::new();
        state.push(make_explanation("Rust", ExplanationLevel::Eli15));
        state.push(make_explanation("Ownership", ExplanationLevel::Eli15));

        state.reset();
        assert!(state.current().is_none());
        assert_eq!(state.cache_count(), 2); // cache preserved
    }

    #[test]
    fn clear_cache_clears_both() {
        let mut state = TechDictionaryState::new();
        state.push(make_explanation("Rust", ExplanationLevel::Eli15));
        state.push(make_explanation("Ownership", ExplanationLevel::Eli15));

        state.clear_cache();
        assert!(state.current().is_none());
        assert_eq!(state.cache_count(), 0);
    }
}
