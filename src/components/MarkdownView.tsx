import { useMemo } from 'react';
import type { ReactNode } from 'react';
import Markdown, { defaultUrlTransform } from 'react-markdown';
import type { Components, UrlTransform } from 'react-markdown';
import { openUrl } from '../lib/openUrl';

interface MarkdownViewProps {
  content: string;
  onTermClick: (term: string) => void;
}

// Convert [[term]] to markdown links with quill:// protocol
function convertTermLinks(content: string): string {
  return content.replace(/\[\[([^\]]+)\]\]/g, (_match, term: string) => {
    const encoded = encodeURIComponent(term);
    return `[${term}](quill://explain/${encoded})`;
  });
}

// Ensure code fences (```) are on their own lines for react-markdown to recognize them.
// AI responses sometimes return ```lang code``` on a single line.
function fixCodeFences(content: string): string {
  // Add newline after opening fence + language if followed by non-newline content
  let result = content.replace(/```(\w+)[ \t]+(?!\n)/g, '```$1\n');
  // Add newline before closing fence if not preceded by newline
  result = result.replace(/([^\n])```/g, '$1\n```');
  // Add newline after closing fence if not followed by newline or end of string
  result = result.replace(/```([^\n\w])/g, '```\n$1');
  return result;
}

// Allow quill:// URLs through the URL transform
const urlTransform: UrlTransform = (url) => {
  if (url.startsWith('quill://')) return url;
  return defaultUrlTransform(url);
};

export default function MarkdownView({ content, onTermClick }: MarkdownViewProps) {
  const processed = useMemo(() => convertTermLinks(fixCodeFences(content)), [content]);

  const components: Components = useMemo(
    () => ({
      a(props: { href?: string; children?: ReactNode }) {
        const { href, children } = props;
        if (href?.startsWith('quill://explain/')) {
          const term = decodeURIComponent(href.replace('quill://explain/', ''));
          return (
            <button
              onClick={() => onTermClick(term)}
              className="text-blue-400 hover:text-blue-300 underline decoration-dotted cursor-pointer"
            >
              {children}
            </button>
          );
        }
        return (
          <button
            onClick={() => href && openUrl(href)}
            className="text-blue-400 hover:text-blue-300 underline cursor-pointer"
          >
            {children}
          </button>
        );
      },
      code(props: { children?: ReactNode; className?: string }) {
        const { children, className } = props;
        const isBlock = className?.includes('language-');
        if (isBlock) {
          return (
            <code className={`block bg-gray-800 rounded-md p-3 text-sm overflow-x-auto ${className ?? ''}`}>
              {children}
            </code>
          );
        }
        return (
          <code className="bg-gray-700/60 text-blue-300 rounded px-1.5 py-0.5 text-sm">
            {children}
          </code>
        );
      },
      pre(props: { children?: ReactNode }) {
        return <pre className="bg-gray-800 rounded-md my-2 overflow-x-auto">{props.children}</pre>;
      },
      h1(props: { children?: ReactNode }) {
        return <h1 className="text-lg font-bold mt-3 mb-1.5">{props.children}</h1>;
      },
      h2(props: { children?: ReactNode }) {
        return <h2 className="text-base font-semibold mt-2.5 mb-1">{props.children}</h2>;
      },
      h3(props: { children?: ReactNode }) {
        return <h3 className="text-sm font-semibold mt-2 mb-1">{props.children}</h3>;
      },
      p(props: { children?: ReactNode }) {
        return <p className="mb-2 leading-relaxed text-sm">{props.children}</p>;
      },
      ul(props: { children?: ReactNode }) {
        return <ul className="list-disc list-inside mb-2 text-sm space-y-0.5">{props.children}</ul>;
      },
      ol(props: { children?: ReactNode }) {
        return <ol className="list-decimal list-inside mb-2 text-sm space-y-0.5">{props.children}</ol>;
      },
      li(props: { children?: ReactNode }) {
        return <li className="leading-relaxed">{props.children}</li>;
      },
      strong(props: { children?: ReactNode }) {
        return <strong className="font-semibold text-white">{props.children}</strong>;
      },
      blockquote(props: { children?: ReactNode }) {
        return (
          <blockquote className="border-l-2 border-blue-500/50 pl-3 my-2 text-gray-300 italic">
            {props.children}
          </blockquote>
        );
      },
    }),
    [onTermClick],
  );

  return (
    <div className="prose prose-invert max-w-none text-gray-200">
      <Markdown urlTransform={urlTransform} components={components}>
        {processed}
      </Markdown>
    </div>
  );
}
