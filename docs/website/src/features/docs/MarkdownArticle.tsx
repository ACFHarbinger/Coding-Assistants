import React, { useEffect, useRef, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeSlug from 'rehype-slug';
import rehypeRaw from 'rehype-raw';
import prism from 'prismjs';
import mermaid from 'mermaid';
import { Copy, Check, AlertTriangle } from 'lucide-react';
import 'prismjs/themes/prism-tomorrow.css';

interface MarkdownArticleProps {
  content: string;
  isDraft?: boolean;
  isUnpublished?: boolean;
  filePath?: string;
}

export const MarkdownArticle: React.FC<MarkdownArticleProps> = ({
  content,
  isDraft,
  isUnpublished,
  filePath,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [copiedCodeId, setCopiedCodeId] = useState<string | null>(null);

  useEffect(() => {
    mermaid.initialize({
      startOnLoad: false,
      theme: 'dark',
      securityLevel: 'loose',
    });
  }, []);

  useEffect(() => {
    if (!containerRef.current) return;

    prism.highlightAllUnder(containerRef.current);

    const mermaidElements = containerRef.current.querySelectorAll('.language-mermaid, pre code.mermaid');
    mermaidElements.forEach((el, index) => {
      const code = el.textContent || '';
      const id = `mermaid-diagram-${index}-${Math.random().toString(36).substring(2, 9)}`;
      const parent = el.parentElement;

      if (parent) {
        mermaid
          .render(id, code)
          .then(({ svg }) => {
            const wrapper = document.createElement('div');
            wrapper.className = 'my-6 p-4 rounded-xl glass-panel flex justify-center overflow-x-auto';
            wrapper.innerHTML = svg;
            parent.replaceWith(wrapper);
          })
          .catch((err) => {
            console.error('Mermaid render error:', err);
            const fallback = document.createElement('pre');
            fallback.className = 'my-4 p-4 rounded-lg bg-slate-900 border border-amber-500/30 text-xs text-amber-200 overflow-x-auto';
            fallback.textContent = `[Mermaid Diagram Source]\n${code}`;
            parent.replaceWith(fallback);
          });
      }
    });
  }, [content]);

  const handleCopy = (codeText: string, id: string) => {
    navigator.clipboard.writeText(codeText);
    setCopiedCodeId(id);
    setTimeout(() => setCopiedCodeId(null), 2000);
  };

  return (
    <article className="max-w-4xl mx-auto w-full space-y-6">
      {(isDraft || isUnpublished) && (
        <div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/30 text-amber-200 flex items-start space-x-3 text-xs leading-relaxed">
          <AlertTriangle className="w-5 h-5 text-amber-400 flex-shrink-0 mt-0.5" />
          <div>
            <h4 className="font-semibold text-amber-300 text-sm mb-0.5">
              Notice: Internal / Not Published Document
            </h4>
            <p>
              This document ({filePath || 'internal document'}) is part of our internal research, status records, or draft specifications. It is excluded from the primary public guide index and may contain active work-in-progress notes.
            </p>
          </div>
        </div>
      )}

      <div ref={containerRef} className="markdown-body">
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          rehypePlugins={[rehypeSlug, rehypeRaw]}
          components={{
            code({ className, children, ...props }) {
              const match = /language-(\w+)/.exec(className || '');
              const isInline = !match && !String(children).includes('\n');
              const codeText = String(children).replace(/\n$/, '');
              const codeId = `code-${Math.random().toString(36).substring(2, 9)}`;

              if (isInline) {
                return (
                  <code className={className} {...props}>
                    {children}
                  </code>
                );
              }

              return (
                <div className="relative group my-4">
                  <button
                    onClick={() => handleCopy(codeText, codeId)}
                    className="absolute top-3 right-3 px-2.5 py-1 rounded-md bg-slate-800/80 hover:bg-slate-700 text-slate-300 hover:text-white border border-slate-700 text-[11px] font-mono flex items-center space-x-1.5 opacity-0 group-hover:opacity-100 transition-opacity z-10"
                    aria-label="Copy Code"
                  >
                    {copiedCodeId === codeId ? (
                      <>
                        <Check className="w-3.5 h-3.5 text-emerald-400" />
                        <span className="text-emerald-400">Copied!</span>
                      </>
                    ) : (
                      <>
                        <Copy className="w-3.5 h-3.5 text-slate-400" />
                        <span>Copy</span>
                      </>
                    )}
                  </button>
                  <pre className={className}>
                    <code className={className} {...props}>
                      {children}
                    </code>
                  </pre>
                </div>
              );
            },
          }}
        >
          {content}
        </ReactMarkdown>
      </div>
    </article>
  );
};
