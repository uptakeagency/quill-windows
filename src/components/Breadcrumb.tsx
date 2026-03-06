interface BreadcrumbProps {
  items: string[];
  onNavigate: (index: number) => void;
}

export default function Breadcrumb({ items, onNavigate }: BreadcrumbProps) {
  if (items.length <= 1) return null;

  return (
    <div className="flex items-center gap-1 text-xs text-gray-400 overflow-x-auto whitespace-nowrap py-1">
      {items.map((item, index) => (
        <span key={index} className="flex items-center gap-1">
          {index > 0 && <span className="text-gray-600">{'\u2192'}</span>}
          {index < items.length - 1 ? (
            <button
              onClick={() => onNavigate(index)}
              className="text-blue-400 hover:text-blue-300 hover:underline transition-colors"
            >
              {item}
            </button>
          ) : (
            <span className="text-gray-200 font-medium">{item}</span>
          )}
        </span>
      ))}
    </div>
  );
}
