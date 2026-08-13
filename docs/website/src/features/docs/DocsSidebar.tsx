import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { BookOpen, ChevronRight } from 'lucide-react';
import docsManifestData from '../../content/docs-manifest.json';

export const DocsSidebar: React.FC = () => {
  const location = useLocation();
  const currentSlug = location.pathname.replace(/^\/docs\/?/, '') || 'documentation_standards';

  return (
    <aside className="w-64 flex-shrink-0 hidden lg:block sticky top-20 h-[calc(100vh-5rem)] overflow-y-auto pr-4 border-r border-slate-800/80">
      <div className="space-y-6 py-2">
        <div className="flex items-center space-x-2 text-xs font-semibold text-slate-400 uppercase tracking-wider px-2">
          <BookOpen className="w-3.5 h-3.5 text-cyan-400" />
          <span>Documentation</span>
        </div>

        {docsManifestData.categories.map((category) => (
          <div key={category.name} className="space-y-1.5">
            <h3 className="px-2 text-xs font-semibold text-slate-400 uppercase tracking-wider flex items-center justify-between">
              <span>{category.name}</span>
              <span className="text-[10px] text-slate-600 font-mono">{category.docs.length}</span>
            </h3>
            <div className="space-y-0.5">
              {category.docs.map((doc) => {
                const isActive = currentSlug === doc.slug;
                return (
                  <Link
                    key={doc.slug}
                    to={`/docs/${doc.slug}`}
                    className={`group flex items-center justify-between px-2.5 py-1.5 rounded-md text-xs font-medium transition-all ${
                      isActive
                        ? 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/30 shadow-sm shadow-cyan-500/10'
                        : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
                    }`}
                  >
                    <span className="truncate">{doc.title}</span>
                    <ChevronRight
                      className={`w-3 h-3 transition-transform ${
                        isActive ? 'text-cyan-400 translate-x-0.5' : 'text-slate-600 opacity-0 group-hover:opacity-100'
                      }`}
                    />
                  </Link>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </aside>
  );
};
