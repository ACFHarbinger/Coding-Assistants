import React, { useEffect, useState } from 'react';
import { AlignLeft } from 'lucide-react';
import { DocHeader } from '../../types';

export const TableOfContents: React.FC<{ headers: DocHeader[] }> = ({ headers }) => {
  const [activeId, setActiveId] = useState<string>('');

  useEffect(() => {
    if (!headers || headers.length === 0) return;

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            setActiveId(entry.target.id);
          }
        });
      },
      { rootMargin: '0px 0px -70% 0px', threshold: 0.1 }
    );

    headers.forEach((header) => {
      const el = document.getElementById(header.id);
      if (el) observer.observe(el);
    });

    return () => observer.disconnect();
  }, [headers]);

  if (!headers || headers.length === 0) return null;

  return (
    <div className="w-56 flex-shrink-0 hidden xl:block sticky top-20 h-[calc(100vh-5rem)] overflow-y-auto pl-4 border-l border-[var(--glass-border)]">
      <div className="py-2 space-y-3">
        <div className="flex items-center space-x-2 text-xs font-semibold text-[var(--text-secondary)] uppercase tracking-wider">
          <AlignLeft className="w-3.5 h-3.5 text-indigo-400" />
          <span>On this page</span>
        </div>
        <nav className="space-y-1">
          {headers.map((header) => {
            const isActive = activeId === header.id;
            return (
              <a
                key={header.id}
                href={`#${header.id}`}
                onClick={(e) => {
                  e.preventDefault();
                  const el = document.getElementById(header.id);
                  if (el) {
                    el.scrollIntoView({ behavior: 'smooth' });
                    setActiveId(header.id);
                  }
                }}
                className={`block text-xs transition-colors truncate ${
                  header.level === 3 ? 'pl-3' : ''
                } ${
                  isActive
                    ? 'text-indigo-400 font-semibold'
                    : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
                }`}
              >
                {header.text}
              </a>
            );
          })}
        </nav>
      </div>
    </div>
  );
};
