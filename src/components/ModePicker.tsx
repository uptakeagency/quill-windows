import type { AnalysisMode } from '../lib/types';

interface ModePickerProps {
  mode: AnalysisMode;
  onModeChange: (mode: AnalysisMode) => void;
}

const modes: { value: AnalysisMode; label: string }[] = [
  { value: 'improve', label: 'Improve' },
  { value: 'translate', label: 'Translate' },
  { value: 'techExplain', label: 'Tech Dictionary' },
];

export default function ModePicker({ mode, onModeChange }: ModePickerProps) {
  return (
    <div className="flex bg-gray-800 rounded-lg p-0.5 gap-0.5">
      {modes.map((m) => (
        <button
          key={m.value}
          onClick={() => onModeChange(m.value)}
          className={`flex-1 px-3 py-1.5 text-xs font-medium rounded-md transition-colors ${
            mode === m.value
              ? 'bg-blue-500 text-white shadow-sm'
              : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700'
          }`}
        >
          {m.label}
        </button>
      ))}
    </div>
  );
}
