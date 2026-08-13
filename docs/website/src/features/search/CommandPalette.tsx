import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Search, X, ChevronRight } from 'lucide-react';
import MiniSearch from 'minisearch';
import searchIndexData from '../../content/search-index.json';

interface SearchDoc {
  id: string;
  title: string;
  category: string;
  summary: string;
  content: string;
}

interface SearchResultItem {
  id: string;
  title: string;
  category: string;
  summary: string;
  score: number;
}

export const CommandPalette: React.FC<{ isOpen: boolean; onClose: () => void }> = ({ isOpen, onClose }) => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResultItem[]>([]);
  const [miniSearch, setMiniSearch] = useState<MiniSearch<SearchDoc> | null>(null);
  const navigate = useNavigate();

  useEffect(() => {
    const search = new MiniSearch<SearchDoc>({
      fields: ['title', 'category', 'summary', 'content'],
      storeFields: ['title', 'category', 'summary'],
      searchOptions: {
        fuzzy: 0.2,
        prefix: true,
      },
    });

    search.addAll(searchIndexData as SearchDoc[]);
    setMiniSearch(search);
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        if (isOpen) {
          onClose();
        } else {
          const btn = document.querySelector('[aria-label="Search Documentation"]') as HTMLButtonElement;
          if (btn) btn.click();
        }
      } else if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  useEffect(() => {
    if (!query.trim() || !miniSearch) {
      setResults([]);
      return;
    }

    const searchResults = miniSearch.search(query).slice(0, 8);
    setResults(
      searchResults.map((r) => ({
        id: r.id as string,
        title: r.title as string,
        category: r.category as string,
        summary: r.summary as string,
        score: r.score,
      }))
    );
  }, [query, miniSearch]);

  if (!isOpen) return null;

  const handleSelect = (slug: string) => {
    onClose();
    navigate(`/docs/${slug}`);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-20 px-4 bg-slate-950/80 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="relative w-full max-w-2xl glass-panel rounded-xl border border-slate-800 bg-slate-900/95 shadow-2xl overflow-hidden">
        <div className="flex items-center px-4 py-3.5 border-b border-slate-800">
          <Search className="w-5 h-5 text-cyan-400 mr-3 flex-shrink-0" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search documentation, guides, roadmaps..."
            className="w-full bg-transparent text-slate-100 placeholder-slate-500 text-sm focus:outline-none"
            autoFocus
          />
          <button
            onClick={onClose}
            className="p-1 rounded-md text-slate-400 hover:text-slate-200 hover:bg-slate-800"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="max-h-96 overflow-y-auto p-2">
          {query.trim() && results.length === 0 ? (
            <div className="p-8 text-center text-slate-500 text-sm">
              No matching documentation found for &quot;<span className="text-slate-300">{query}</span>&quot;
            </div>
          ) : results.length > 0 ? (
            <div className="space-y-1">
              {results.map((item) => (
                <button
                  key={item.id}
                  onClick={() => handleSelect(item.id)}
                  className="w-full text-left p-3 rounded-lg hover:bg-slate-800/70 border border-transparent hover:border-cyan-500/20 group transition-all"
                >
                  <div className="flex items-center justify-between text-xs text-cyan-400 font-medium mb-1">
                    <span>{item.category}</span>
                    <ChevronRight className="w-3.5 h-3.5 text-slate-600 group-hover:text-cyan-400 transition-colors" />
                  </div>
                  <h4 className="text-sm font-semibold text-slate-200 group-hover:text-white mb-0.5">
                    {item.title}
                  </h4>
                  <p className="text-xs text-slate-400 line-clamp-1">{item.summary}</p>
                </button>
              ))}
            </div>
          ) : (
            <div className="p-6 text-center text-slate-500 text-xs">
              Type keywords to search across all canonical Markdown documentation.
            </div>
          )}
        </div>

        <div className="px-4 py-2.5 bg-slate-950/60 border-t border-slate-800/80 flex items-center justify-between text-[11px] text-slate-500">
          <span>Search index updated build-time</span>
          <div className="flex items-center space-x-2">
            <span>Navigate <kbd className="px-1 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-400">↵</kbd></span>
            <span>Close <kbd className="px-1 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-400">ESC</kbd></span>
          </div>
        </div>
      </div>
    </div>
  );
};
