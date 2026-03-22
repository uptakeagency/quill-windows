import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAnalysis } from '../hooks/useAnalysis';
import { useDrillDown } from '../hooks/useDrillDown';
import ModePicker from './ModePicker';
import LevelPicker from './LevelPicker';
import Breadcrumb from './Breadcrumb';
import MarkdownView from './MarkdownView';
import SuggestionView from './SuggestionView';
import VocabularyCards from './VocabularyCard';
import AlternativesView from './AlternativesView';
import ResourcesView from './ResourcesView';
import type { AnalysisMode, ExplanationLevel, TechExplanation } from '../lib/types';

export default function FloatingPanel() {
  const {
    isAnalyzing,
    result,
    error,
    capturedText,
    analyze,
    applyText,
    changeMode,
    changeLevel,
  } = useAnalysis();

  const drillDown = useDrillDown();

  const [mode, setMode] = useState<AnalysisMode>('techExplain');
  const [level, setLevel] = useState<ExplanationLevel>('eli15');

  // Native window drag — uses OS drag loop, no IPC overhead
  const handleDragStart = (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest('button')) return;
    e.preventDefault();
    getCurrentWindow().startDragging().catch(() => {});
  };

  // Reset drill-down stack on new text capture (new hotkey press)
  useEffect(() => {
    if (capturedText) {
      drillDown.reset();
    }
  }, [capturedText, drillDown.reset]);

  // Sync result to drill-down stack
  // Combined mode: result.levels contains all 6 level explanations in one response
  useEffect(() => {
    if (result && result.mode === 'techExplain' && (result.levels || result.explanation)) {
      const levels = result.levels ?? { eli15: result.explanation ?? '' };
      const explanation: TechExplanation = {
        term: result.original,
        levels,
        tldr: result.tldr,
        resources: result.resources,
        alternatives: result.alternatives,
      };
      const currentTerm = drillDown.current?.term;
      if (currentTerm && currentTerm.toLowerCase() === explanation.term.toLowerCase()) {
        drillDown.replaceTop(explanation);
      } else {
        drillDown.push(explanation);
      }
    }
  }, [result, drillDown.push, drillDown.replaceTop, drillDown.current?.term]);

  // ESC key to dismiss
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        invoke('hide_panel_cmd').catch(() => {});
      }
    }
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const handleModeChange = (newMode: AnalysisMode) => {
    setMode(newMode);
    changeMode(newMode);
    drillDown.reset();
    if (capturedText) {
      analyze(capturedText, newMode);
    }
  };

  const handleLevelChange = (newLevel: ExplanationLevel) => {
    setLevel(newLevel);
    changeLevel(newLevel);
    // No API call — just switch displayed content from pre-fetched levels
  };

  const handleTermClick = (term: string) => {
    analyze(term, 'techExplain');
  };

  const handleApply = async (text: string) => {
    await applyText(text);
  };

  const handleDismiss = async () => {
    await invoke('hide_panel_cmd');
  };

  // Determine what content to show
  const isTechMode = mode === 'techExplain';
  const currentExplanation = drillDown.current;
  const displayTerm = currentExplanation?.term ?? capturedText;
  const tldr = currentExplanation?.tldr ?? result?.tldr;
  const levelContent = currentExplanation?.levels[level];

  return (
    <div className="w-full h-screen bg-gray-900/95 backdrop-blur-sm rounded-xl shadow-2xl flex flex-col overflow-hidden border border-gray-700/50">
      {/* Mode picker (drag handle) */}
      <div
        className="px-3 pt-3 pb-2 cursor-grab active:cursor-grabbing select-none"
        onMouseDown={handleDragStart}
      >
        <ModePicker mode={mode} onModeChange={handleModeChange} />
      </div>

      {/* Selected term/text header */}
      {displayTerm && (
        <div className="px-3 pb-1">
          <h2 className="text-base font-bold text-white truncate">{displayTerm}</h2>
        </div>
      )}

      {/* TL;DR (tech mode only) */}
      {isTechMode && tldr && (
        <div className="px-3 pb-2">
          <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg px-3 py-2">
            <p className="text-xs text-blue-200 leading-relaxed">
              <span className="font-semibold text-blue-300">TL;DR: </span>
              {tldr}
            </p>
          </div>
        </div>
      )}

      {/* Level picker + breadcrumb (tech mode only) */}
      {isTechMode && (
        <div className="px-3 pb-1 space-y-1">
          <LevelPicker level={level} onLevelChange={handleLevelChange} />
          <Breadcrumb items={drillDown.breadcrumbs} onNavigate={drillDown.popTo} />
        </div>
      )}

      {/* Scrollable content area */}
      <div className="flex-1 overflow-y-auto px-3 py-2 min-h-0">
        {isAnalyzing && (
          <div className="flex items-center justify-center py-12">
            <div className="flex items-center gap-3 text-gray-400">
              <svg className="animate-spin h-5 w-5" viewBox="0 0 24 24" fill="none">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              <span className="text-sm">Analyzing...</span>
            </div>
          </div>
        )}

        {error && !isAnalyzing && (
          <div className="bg-red-500/10 border border-red-500/30 rounded-lg p-3 my-2">
            <p className="text-sm text-red-300">{error}</p>
          </div>
        )}

        {!isAnalyzing && !error && result && (
          <>
            {/* Tech Dictionary mode */}
            {isTechMode && currentExplanation && (
              <>
                {(() => {
                  const hasAltsData = level === 'alternatives' && currentExplanation.alternatives && currentExplanation.alternatives.length > 0;
                  const hasResData = level === 'resources' && currentExplanation.resources && currentExplanation.resources.length > 0;
                  const hasContent = levelContent || hasAltsData || hasResData;

                  if (!hasContent) {
                    return (
                      <div className="text-sm text-gray-500 py-8 text-center">
                        This level is not available for the current term.
                      </div>
                    );
                  }

                  return (
                    <>
                      {levelContent && (
                        <MarkdownView
                          content={levelContent}
                          onTermClick={handleTermClick}
                        />
                      )}
                      {hasAltsData && (
                        <AlternativesView
                          alternatives={currentExplanation.alternatives!}
                          onTermClick={handleTermClick}
                        />
                      )}
                      {hasResData && (
                        <ResourcesView resources={currentExplanation.resources!} />
                      )}
                    </>
                  );
                })()}
              </>
            )}

            {/* Improve mode */}
            {mode === 'improve' && (
              <>
                <SuggestionView
                  changes={result.changes}
                  correctedText={result.corrected}
                  onApply={handleApply}
                />
                <VocabularyCards cards={result.vocabulary} />
              </>
            )}

            {/* Translate mode */}
            {mode === 'translate' && result.explanation && (
              <MarkdownView
                content={result.explanation}
                onTermClick={handleTermClick}
              />
            )}
          </>
        )}

        {!isAnalyzing && !error && !result && !currentExplanation && (
          <div className="flex items-center justify-center py-12 text-gray-500 text-sm">
            Select text and press Ctrl+Alt+Q
          </div>
        )}
      </div>

      {/* Action bar */}
      <div className="px-3 py-2 border-t border-gray-700/50 flex gap-2">
        {mode === 'improve' && result && result.changes.length > 0 && (
          <button
            onClick={() => handleApply(result.corrected)}
            className="flex-1 py-1.5 bg-blue-500 hover:bg-blue-600 text-white text-sm font-medium rounded-lg transition-colors"
          >
            Apply
          </button>
        )}
        <button
          onClick={handleDismiss}
          className={`py-1.5 text-gray-400 hover:text-gray-200 text-sm font-medium rounded-lg hover:bg-gray-800 transition-colors ${
            mode === 'improve' && result && result.changes.length > 0 ? 'flex-1' : 'w-full'
          }`}
        >
          Dismiss
        </button>
      </div>
    </div>
  );
}
