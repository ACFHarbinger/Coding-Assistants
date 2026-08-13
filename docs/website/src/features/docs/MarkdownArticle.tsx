import React, { useEffect, useRef } from 'react';
import { marked } from 'marked';
import prism from 'prismjs';
import mermaid from 'mermaid';
import 'prismjs/themes/prism-tomorrow.css';

interface MarkdownArticleProps {
  content: string;
}

export const MarkdownArticle: React.FC<MarkdownArticleProps> = ({ content }) => {
  const containerRef = useRef<HTMLDivElement>(null);

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
      const id = `mermaid-diagram-${index}-${Math.random().toString(36).substr(2, 9)}`;
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

  const parsedHtml = marked.parse(content) as string;

  return (
    <article className="max-w-4xl mx-auto w-full">
      <div
        ref={containerRef}
        className="markdown-body"
        dangerouslySetInnerHTML={{ __html: parsedHtml }}
      />
    </article>
  );
};
