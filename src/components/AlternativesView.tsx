import type { Alternative } from '../lib/types';
import { openUrl } from '../lib/openUrl';

interface AlternativesViewProps {
  alternatives: Alternative[];
  onTermClick: (term: string) => void;
}

export default function AlternativesView({ alternatives, onTermClick }: AlternativesViewProps) {
  if (alternatives.length === 0) return null;

  return (
    <div className="space-y-2.5 mt-3">
      {alternatives.map((alt, index) => (
        <div
          key={index}
          className="bg-gray-800/60 border border-gray-700/50 rounded-lg p-3"
        >
          {/* Name with optional link */}
          <h4 className="text-sm font-semibold mb-1">
            {alt.url ? (
              <button
                onClick={() => openUrl(alt.url!)}
                className="text-blue-400 hover:text-blue-300 underline cursor-pointer"
              >
                {alt.name}
              </button>
            ) : (
              <button
                onClick={() => onTermClick(alt.name)}
                className="text-blue-400 hover:text-blue-300 underline decoration-dotted cursor-pointer"
              >
                {alt.name}
              </button>
            )}
          </h4>

          {/* Description */}
          <p className="text-xs text-gray-300 mb-2 leading-relaxed">
            {alt.description}
          </p>

          {/* Pros and Cons */}
          <div className="flex gap-4 text-xs">
            {alt.pros.length > 0 && (
              <div className="flex-1 space-y-0.5">
                {alt.pros.map((pro, i) => (
                  <div key={i} className="text-green-300/80">
                    <span className="text-green-400 mr-1">+</span>{pro}
                  </div>
                ))}
              </div>
            )}
            {alt.cons.length > 0 && (
              <div className="flex-1 space-y-0.5">
                {alt.cons.map((con, i) => (
                  <div key={i} className="text-red-300/80">
                    <span className="text-red-400 mr-1">-</span>{con}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
