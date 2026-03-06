import type { TextChange } from '../lib/types';

interface SuggestionViewProps {
  changes: TextChange[];
  correctedText: string;
  onApply: (text: string) => void;
}

export default function SuggestionView({ changes, correctedText, onApply }: SuggestionViewProps) {
  if (changes.length === 0) {
    return (
      <div className="text-center py-6 text-gray-400 text-sm">
        No changes suggested. Your text looks good!
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <div className="text-xs text-gray-400 mb-2">
        {changes.length} change{changes.length !== 1 ? 's' : ''} suggested
      </div>

      {changes.map((change, index) => (
        <div key={index} className="bg-gray-800/60 rounded-lg p-2.5 space-y-1">
          <div className="flex items-start gap-2 text-sm">
            <span className="line-through text-red-400/80 break-words">{change.original}</span>
            <span className="text-gray-500 shrink-0">{'\u2192'}</span>
            <span className="font-medium text-green-400 break-words">{change.replacement}</span>
          </div>
          {change.reason && (
            <div className="text-xs text-gray-500 italic">{change.reason}</div>
          )}
        </div>
      ))}

      <button
        onClick={() => onApply(correctedText)}
        className="w-full mt-2 py-1.5 px-3 bg-blue-500 hover:bg-blue-600 text-white text-sm font-medium rounded-lg transition-colors"
      >
        Apply All Changes
      </button>
    </div>
  );
}
