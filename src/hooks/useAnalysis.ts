import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type {
  AnalysisResult,
  AnalysisMode,
  ExplanationLevel,
  TextCapturedPayload,
  AnalysisResultPayload,
  AnalysisErrorPayload,
  AnalyzingPayload,
} from '../lib/types';

interface UseAnalysisState {
  isAnalyzing: boolean;
  result: AnalysisResult | null;
  error: string | null;
  capturedText: string | null;
  capturedMode: AnalysisMode | null;
  capturedContext: string | null;
}

export function useAnalysis() {
  const [state, setState] = useState<UseAnalysisState>({
    isAnalyzing: false,
    result: null,
    error: null,
    capturedText: null,
    capturedMode: null,
    capturedContext: null,
  });

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      listen<AnalyzingPayload>('analyzing', (event) => {
        setState((prev) => ({ ...prev, isAnalyzing: event.payload.status }));
      }),
    );

    unlisteners.push(
      listen<TextCapturedPayload>('text-captured', (event) => {
        setState((prev) => ({
          ...prev,
          capturedText: event.payload.text,
          capturedMode: event.payload.mode,
          capturedContext: event.payload.context ?? null,
          result: null,
          error: null,
        }));
      }),
    );

    unlisteners.push(
      listen<AnalysisResultPayload>('analysis-result', (event) => {
        setState((prev) => ({
          ...prev,
          result: event.payload.result,
          error: null,
          isAnalyzing: false,
        }));
      }),
    );

    unlisteners.push(
      listen<AnalysisErrorPayload>('analysis-error', (event) => {
        setState((prev) => ({
          ...prev,
          error: event.payload.error,
          isAnalyzing: false,
        }));
      }),
    );

    return () => {
      unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
    };
  }, []);

  const analyze = async (text: string, mode: AnalysisMode, level?: ExplanationLevel) => {
    setState((prev) => ({ ...prev, isAnalyzing: true, error: null }));
    try {
      const result = await invoke<AnalysisResult>('analyze', {
        text,
        mode: mode as string,
        level: level as string | undefined,
      });
      setState((prev) => ({ ...prev, result, isAnalyzing: false }));
    } catch (e) {
      setState((prev) => ({ ...prev, error: String(e), isAnalyzing: false }));
    }
  };

  const applyText = async (text: string) => {
    await invoke('apply_text', { text });
  };

  const hidePanel = async () => {
    await invoke('hide_panel_cmd');
  };

  const changeMode = async (mode: AnalysisMode) => {
    await invoke('change_mode', { mode: mode as string });
  };

  const changeLevel = async (level: ExplanationLevel) => {
    await invoke('change_level', { level: level as string });
  };

  return {
    ...state,
    analyze,
    applyText,
    hidePanel,
    changeMode,
    changeLevel,
  };
}
