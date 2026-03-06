import type { VocabularyCard as VocabularyCardType } from '../lib/types';

interface VocabularyCardProps {
  cards: VocabularyCardType[];
}

export default function VocabularyCards({ cards }: VocabularyCardProps) {
  if (cards.length === 0) return null;

  return (
    <div className="space-y-2 mt-3">
      <h3 className="text-xs font-semibold text-gray-400 uppercase tracking-wide">
        Vocabulary Suggestions
      </h3>
      {cards.map((card, index) => (
        <div key={index} className="bg-gray-800/60 rounded-lg p-2.5 space-y-1">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-white">{card.word}</span>
            <span className="text-gray-500">{'\u2192'}</span>
            <span className="text-sm font-medium text-blue-300">{card.suggestion}</span>
            <span className="ml-auto text-[10px] px-1.5 py-0.5 rounded-full bg-blue-500/20 text-blue-300 font-medium">
              {card.level}
            </span>
          </div>
          <p className="text-xs text-gray-400">{card.definition}</p>
          {card.example && (
            <p className="text-xs text-gray-500 italic">&ldquo;{card.example}&rdquo;</p>
          )}
        </div>
      ))}
    </div>
  );
}
