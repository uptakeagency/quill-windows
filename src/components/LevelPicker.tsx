import type { ExplanationLevel } from '../lib/types';

interface LevelPickerProps {
  level: ExplanationLevel;
  onLevelChange: (level: ExplanationLevel) => void;
}

const levels: { value: ExplanationLevel; label: string; emoji: string }[] = [
  { value: 'eli5', label: 'ELI5', emoji: '\u{1F476}' },
  { value: 'eli15', label: 'ELI15', emoji: '\u{1F393}' },
  { value: 'professional', label: 'Pro', emoji: '\u{1F4BC}' },
  { value: 'samples', label: 'Samples', emoji: '\u{1F4BB}' },
  { value: 'resources', label: 'Resources', emoji: '\u{1F4DA}' },
  { value: 'alternatives', label: 'Alts', emoji: '\u{1F500}' },
];

export default function LevelPicker({ level, onLevelChange }: LevelPickerProps) {
  return (
    <div className="flex gap-0.5 bg-gray-800/50 rounded-lg p-0.5">
      {levels.map((l) => (
        <button
          key={l.value}
          onClick={() => onLevelChange(l.value)}
          title={l.label}
          className={`flex-1 px-1.5 py-1 text-xs rounded-md transition-colors text-center ${
            level === l.value
              ? 'bg-blue-500/80 text-white shadow-sm'
              : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700/50'
          }`}
        >
          <span className="block text-sm leading-none">{l.emoji}</span>
          <span className="block text-[10px] mt-0.5 leading-none">{l.label}</span>
        </button>
      ))}
    </div>
  );
}
