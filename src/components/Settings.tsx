import { invoke } from '@tauri-apps/api/core';
import { useState, useEffect, useCallback } from 'react';
import type { AppSettings, AnalysisMode, ExplanationLevel, ToneStyle } from '../lib/types';

type AiProvider = 'gemini' | 'claude';

export default function Settings() {
  // Form state
  const [aiProvider, setAiProvider] = useState<AiProvider>('gemini');
  const [geminiModel, setGeminiModel] = useState('gemini-2.5-flash');
  const [claudeModel, setClaudeModel] = useState('claude-sonnet-4-20250514');
  const [nativeLanguage, setNativeLanguage] = useState('Turkish');
  const [targetLanguage, setTargetLanguage] = useState('English');
  const [mode, setMode] = useState<AnalysisMode>('techExplain');
  const [level, setLevel] = useState<ExplanationLevel>('eli15');
  const [tone, setTone] = useState<ToneStyle | ''>('');
  const [hotkey, setHotkey] = useState('Ctrl+Shift+Q');
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);

  // API key state
  const [geminiKeyInput, setGeminiKeyInput] = useState('');
  const [claudeKeyInput, setClaudeKeyInput] = useState('');
  const [hasGeminiKey, setHasGeminiKey] = useState(false);
  const [hasClaudeKey, setHasClaudeKey] = useState(false);

  // Feedback state
  const [saveStatus, setSaveStatus] = useState<'idle' | 'saved' | 'error'>('idle');
  const [keyStatus, setKeyStatus] = useState<Record<string, string>>({});

  // Load settings and API key status on mount
  useEffect(() => {
    invoke<AppSettings>('get_settings').then((settings) => {
      setAiProvider(settings.aiProvider as AiProvider);
      setGeminiModel(settings.geminiModel);
      setClaudeModel(settings.claudeModel);
      setNativeLanguage(settings.nativeLanguage);
      setTargetLanguage(settings.targetLanguage);
      setMode(settings.mode);
      setLevel(settings.level);
      setTone(settings.tone ?? '');
      setHotkey(settings.hotkey ?? 'Ctrl+Shift+Q');
    });

    invoke<string | null>('get_gemini_key').then((key) => {
      setHasGeminiKey(!!key);
    });
    invoke<string | null>('get_claude_key').then((key) => {
      setHasClaudeKey(!!key);
    });
  }, []);

  // Save settings to AppState
  const handleSave = useCallback(async () => {
    const settings: AppSettings = {
      aiProvider,
      geminiModel,
      claudeModel,
      nativeLanguage,
      targetLanguage,
      mode,
      level,
      tone: tone || undefined,
    };

    try {
      await invoke('save_settings', { settings });
      setSaveStatus('saved');
      setTimeout(() => setSaveStatus('idle'), 2000);
    } catch {
      setSaveStatus('error');
      setTimeout(() => setSaveStatus('idle'), 3000);
    }
  }, [aiProvider, geminiModel, claudeModel, nativeLanguage, targetLanguage, mode, level, tone]);

  // API key handlers
  const showKeyFeedback = (key: string, message: string) => {
    setKeyStatus((prev) => ({ ...prev, [key]: message }));
    setTimeout(() => setKeyStatus((prev) => ({ ...prev, [key]: '' })), 2000);
  };

  const handleSaveGeminiKey = async () => {
    if (!geminiKeyInput.trim()) return;
    try {
      await invoke('save_gemini_key', { key: geminiKeyInput });
      setHasGeminiKey(true);
      setGeminiKeyInput('');
      showKeyFeedback('gemini', 'Saved');
    } catch {
      showKeyFeedback('gemini', 'Failed to save');
    }
  };

  const handleDeleteGeminiKey = async () => {
    try {
      await invoke('delete_gemini_key');
      setHasGeminiKey(false);
      showKeyFeedback('gemini', 'Deleted');
    } catch {
      showKeyFeedback('gemini', 'Failed to delete');
    }
  };

  const handleSaveClaudeKey = async () => {
    if (!claudeKeyInput.trim()) return;
    try {
      await invoke('save_claude_key', { key: claudeKeyInput });
      setHasClaudeKey(true);
      setClaudeKeyInput('');
      showKeyFeedback('claude', 'Saved');
    } catch {
      showKeyFeedback('claude', 'Failed to save');
    }
  };

  const handleDeleteClaudeKey = async () => {
    try {
      await invoke('delete_claude_key');
      setHasClaudeKey(false);
      showKeyFeedback('claude', 'Deleted');
    } catch {
      showKeyFeedback('claude', 'Failed to delete');
    }
  };

  const inputClass = 'w-full border border-gray-300 rounded px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-400 focus:border-transparent';
  const selectClass = 'w-full border border-gray-300 rounded px-3 py-2 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-400 focus:border-transparent';
  const labelClass = 'block text-sm font-medium text-gray-700 mb-1';

  return (
    <div className="min-h-screen bg-white text-gray-900">
      <div className="max-w-lg mx-auto px-6 py-5">
        <h1 className="text-lg font-semibold mb-4">Quill Settings</h1>

        {/* AI Backend */}
        <section className="py-4 border-b border-gray-200">
          <h2 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3">AI Backend</h2>
          <div className="space-y-3">
            {/* Provider */}
            <div>
              <label className={labelClass}>AI Provider</label>
              <div className="flex gap-4">
                <label className="flex items-center gap-1.5 text-sm cursor-pointer">
                  <input
                    type="radio"
                    name="aiProvider"
                    value="gemini"
                    checked={aiProvider === 'gemini'}
                    onChange={() => setAiProvider('gemini')}
                    className="accent-blue-500"
                  />
                  Gemini
                </label>
                <label className="flex items-center gap-1.5 text-sm cursor-pointer">
                  <input
                    type="radio"
                    name="aiProvider"
                    value="claude"
                    checked={aiProvider === 'claude'}
                    onChange={() => setAiProvider('claude')}
                    className="accent-blue-500"
                  />
                  Claude
                </label>
              </div>
            </div>

            {/* Gemini API Key */}
            <div>
              <label className={labelClass}>
                Gemini API Key
                {hasGeminiKey && <span className="ml-2 text-green-600 text-xs font-normal">(configured)</span>}
              </label>
              <div className="flex gap-2">
                <input
                  type="password"
                  value={geminiKeyInput}
                  onChange={(e) => setGeminiKeyInput(e.target.value)}
                  placeholder={hasGeminiKey ? '••••••••••••' : 'Enter API key'}
                  className={inputClass}
                />
                <button
                  onClick={handleSaveGeminiKey}
                  disabled={!geminiKeyInput.trim()}
                  className="px-3 py-2 bg-blue-500 text-white text-sm rounded hover:bg-blue-600 disabled:opacity-40 disabled:cursor-not-allowed transition-colors whitespace-nowrap"
                >
                  Save
                </button>
                {hasGeminiKey && (
                  <button
                    onClick={handleDeleteGeminiKey}
                    className="px-3 py-2 bg-red-500 text-white text-sm rounded hover:bg-red-600 transition-colors whitespace-nowrap"
                  >
                    Delete
                  </button>
                )}
              </div>
              {keyStatus.gemini && (
                <p className="text-xs mt-1 text-gray-500">{keyStatus.gemini}</p>
              )}
            </div>

            {/* Gemini Model */}
            <div>
              <label className={labelClass}>Gemini Model</label>
              <input
                type="text"
                value={geminiModel}
                onChange={(e) => setGeminiModel(e.target.value)}
                className={inputClass}
              />
            </div>

            {/* Claude API Key */}
            <div>
              <label className={labelClass}>
                Claude API Key
                {hasClaudeKey && <span className="ml-2 text-green-600 text-xs font-normal">(configured)</span>}
              </label>
              <div className="flex gap-2">
                <input
                  type="password"
                  value={claudeKeyInput}
                  onChange={(e) => setClaudeKeyInput(e.target.value)}
                  placeholder={hasClaudeKey ? '••••••••••••' : 'Enter API key'}
                  className={inputClass}
                />
                <button
                  onClick={handleSaveClaudeKey}
                  disabled={!claudeKeyInput.trim()}
                  className="px-3 py-2 bg-blue-500 text-white text-sm rounded hover:bg-blue-600 disabled:opacity-40 disabled:cursor-not-allowed transition-colors whitespace-nowrap"
                >
                  Save
                </button>
                {hasClaudeKey && (
                  <button
                    onClick={handleDeleteClaudeKey}
                    className="px-3 py-2 bg-red-500 text-white text-sm rounded hover:bg-red-600 transition-colors whitespace-nowrap"
                  >
                    Delete
                  </button>
                )}
              </div>
              {keyStatus.claude && (
                <p className="text-xs mt-1 text-gray-500">{keyStatus.claude}</p>
              )}
            </div>

            {/* Claude Model */}
            <div>
              <label className={labelClass}>Claude Model</label>
              <input
                type="text"
                value={claudeModel}
                onChange={(e) => setClaudeModel(e.target.value)}
                className={inputClass}
              />
            </div>
          </div>
        </section>

        {/* Language */}
        <section className="py-4 border-b border-gray-200">
          <h2 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3">Language</h2>
          <div className="space-y-3">
            <div>
              <label className={labelClass}>Native Language</label>
              <input
                type="text"
                value={nativeLanguage}
                onChange={(e) => setNativeLanguage(e.target.value)}
                className={inputClass}
              />
            </div>
            <div>
              <label className={labelClass}>Target Language</label>
              <input
                type="text"
                value={targetLanguage}
                onChange={(e) => setTargetLanguage(e.target.value)}
                className={inputClass}
              />
            </div>
          </div>
        </section>

        {/* Keyboard Shortcut */}
        <section className="py-4 border-b border-gray-200">
          <h2 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3">Keyboard Shortcut</h2>
          <div>
            <label className={labelClass}>Activation Shortcut</label>
            <div
              tabIndex={0}
              onClick={() => setIsRecordingHotkey(true)}
              onBlur={() => setIsRecordingHotkey(false)}
              onKeyDown={(e) => {
                if (!isRecordingHotkey) return;
                e.preventDefault();
                e.stopPropagation();

                const parts: string[] = [];
                if (e.ctrlKey) parts.push('Ctrl');
                if (e.altKey) parts.push('Alt');
                if (e.shiftKey) parts.push('Shift');
                if (e.metaKey) parts.push('Super');

                // Ignore modifier-only presses
                if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) return;

                // Map physical key code to Tauri shortcut name
                const code = e.code;
                let tauriKey: string | null = null;
                if (code.startsWith('Key')) tauriKey = code.slice(3);
                else if (code.startsWith('Digit')) tauriKey = code.slice(5);
                else if (/^F\d+$/.test(code)) tauriKey = code;
                else if (code === 'Space') tauriKey = 'Space';
                else if (code === 'Escape') tauriKey = 'Escape';
                else if (code === 'Tab') tauriKey = 'Tab';
                else if (code === 'Enter') tauriKey = 'Enter';
                else if (code === 'Backspace') tauriKey = 'Backspace';
                else if (code === 'Delete') tauriKey = 'Delete';

                if (!tauriKey || parts.length === 0) return;
                parts.push(tauriKey);
                const newHotkey = parts.join('+');

                invoke('update_hotkey', { hotkey: newHotkey })
                  .then(() => {
                    setHotkey(newHotkey);
                    setIsRecordingHotkey(false);
                    showKeyFeedback('hotkey', 'Updated');
                  })
                  .catch((err) => {
                    setIsRecordingHotkey(false);
                    showKeyFeedback('hotkey', `Failed: ${err}`);
                  });
              }}
              className={`${inputClass} cursor-pointer text-center font-mono ${
                isRecordingHotkey
                  ? 'ring-2 ring-blue-400 border-transparent bg-blue-50'
                  : ''
              }`}
            >
              {isRecordingHotkey ? 'Press a key combination...' : hotkey}
            </div>
            {keyStatus.hotkey && (
              <p className="text-xs mt-1 text-gray-500">{keyStatus.hotkey}</p>
            )}
            <p className="text-xs text-gray-400 mt-1">
              Click the field and press your desired key combination
            </p>
          </div>
        </section>

        {/* Preferences */}
        <section className="py-4 border-b border-gray-200">
          <h2 className="text-sm font-semibold text-gray-500 uppercase tracking-wide mb-3">Preferences</h2>
          <div className="space-y-3">
            <div>
              <label className={labelClass}>Default Mode</label>
              <select
                value={mode}
                onChange={(e) => setMode(e.target.value as AnalysisMode)}
                className={selectClass}
              >
                <option value="improve">Improve</option>
                <option value="translate">Translate</option>
                <option value="techExplain">Tech Dictionary</option>
              </select>
            </div>
            <div>
              <label className={labelClass}>Default Level</label>
              <select
                value={level}
                onChange={(e) => setLevel(e.target.value as ExplanationLevel)}
                className={selectClass}
              >
                <option value="eli5">ELI5</option>
                <option value="eli15">ELI15</option>
                <option value="professional">Pro</option>
                <option value="samples">Samples</option>
                <option value="resources">Resources</option>
                <option value="alternatives">Alternatives</option>
              </select>
            </div>
            <div>
              <label className={labelClass}>Default Tone</label>
              <select
                value={tone}
                onChange={(e) => setTone(e.target.value as ToneStyle | '')}
                className={selectClass}
              >
                <option value="">None</option>
                <option value="formal">Formal</option>
                <option value="casual">Casual</option>
                <option value="professional">Professional</option>
                <option value="friendly">Friendly</option>
              </select>
            </div>
          </div>
        </section>

        {/* Actions */}
        <div className="py-4 flex items-center gap-3">
          <button
            onClick={handleSave}
            className="px-4 py-2 bg-blue-500 text-white text-sm font-medium rounded hover:bg-blue-600 transition-colors"
          >
            Save Settings
          </button>
          {saveStatus === 'saved' && (
            <span className="text-sm text-green-600 font-medium">Saved!</span>
          )}
          {saveStatus === 'error' && (
            <span className="text-sm text-red-600 font-medium">Failed to save</span>
          )}
        </div>
      </div>
    </div>
  );
}
