// TypeScript types matching Rust models (serde camelCase serialization)

// Matches Rust AnalysisMode
export type AnalysisMode = 'improve' | 'translate' | 'techExplain';

// Matches Rust ExplanationLevel
export type ExplanationLevel = 'eli5' | 'eli15' | 'professional' | 'samples' | 'resources' | 'alternatives';

// Matches Rust ToneStyle
export type ToneStyle = 'formal' | 'casual' | 'professional' | 'friendly';

// Matches Rust TextChange
export interface TextChange {
  original: string;
  replacement: string;
  reason: string;
}

// Matches Rust ResourceLink
export interface ResourceLink {
  title: string;
  url: string;
}

// Matches Rust VocabularyCard
export interface VocabularyCard {
  word: string;
  suggestion: string;
  definition: string;
  example: string;
  level: string;
}

// Matches Rust Alternative
export interface Alternative {
  name: string;
  description: string;
  pros: string[];
  cons: string[];
  url?: string;
}

// Matches Rust AnalysisResult
export interface AnalysisResult {
  mode: AnalysisMode;
  original: string;
  corrected: string;
  changes: TextChange[];
  explanation?: string;
  tldr?: string;
  resources?: ResourceLink[];
  alternatives?: Alternative[];
  vocabulary: VocabularyCard[];
  levels?: Partial<Record<ExplanationLevel, string>>;
}

// Tech dictionary entry with all levels pre-fetched
export interface TechExplanation {
  term: string;
  levels: Partial<Record<ExplanationLevel, string>>;
  tldr?: string;
  resources?: ResourceLink[];
  alternatives?: Alternative[];
}

// Settings interface for get_settings/save_settings commands
export interface AppSettings {
  mode: AnalysisMode;
  level: ExplanationLevel;
  tone?: ToneStyle;
  nativeLanguage: string;
  targetLanguage: string;
  aiProvider: string;
  geminiModel: string;
  claudeModel: string;
  hotkey?: string;
}

// Event payloads from Rust backend
export interface TextCapturedPayload {
  text: string;
  mode: AnalysisMode;
  context?: string;
}

export interface AnalysisResultPayload {
  result: AnalysisResult;
}

export interface AnalysisErrorPayload {
  error: string;
}

export interface AnalyzingPayload {
  status: boolean;
}
