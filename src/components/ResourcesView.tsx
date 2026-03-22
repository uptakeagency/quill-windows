import type { ResourceLink } from '../lib/types';
import { openUrl } from '../lib/openUrl';

interface ResourcesViewProps {
  resources: ResourceLink[];
}

export default function ResourcesView({ resources }: ResourcesViewProps) {
  if (resources.length === 0) return null;

  return (
    <div className="mt-3 pt-2 border-t border-gray-700/50">
      <h4 className="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-2">
        Resources
      </h4>
      <div className="space-y-1.5">
        {resources.map((resource, index) => (
          <button
            key={index}
            onClick={() => openUrl(resource.url)}
            className="flex items-center gap-2 text-sm text-blue-400 hover:text-blue-300 hover:bg-gray-800/50 rounded px-2 py-1 transition-colors w-full text-left cursor-pointer"
          >
            <span className="text-gray-500 text-xs">&#x2197;</span>
            <span className="truncate">{resource.title}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
