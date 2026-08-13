import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Search, X } from "lucide-react";
import searchIndexData from "../../content/search-index.json";
import { createDocSearch, rankQuery, type SearchDoc } from "./searchIndex";

interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  onOpen?: () => void;
}

export function CommandPalette({ isOpen, onClose, onOpen }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();
  const miniSearch = useMemo(
    () => createDocSearch(searchIndexData as SearchDoc[]),
    [],
  );
  const results = useMemo(() => rankQuery(miniSearch, query), [miniSearch, query]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        if (isOpen) onClose();
        else onOpen?.();
      } else if (event.key === "Escape" && isOpen) {
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, onClose, onOpen]);

  useEffect(() => {
    if (isOpen) {
      setQuery("");
      setActive(0);
      inputRef.current?.focus();
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const select = (id: string) => {
    onClose();
    navigate(`/docs/${id}`);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center bg-[#020617]/80 px-4 pt-24 backdrop-blur-sm motion-reduce:backdrop-blur-none">
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Search documentation"
        className="w-full max-w-2xl overflow-hidden rounded-2xl border border-white/10 bg-[rgba(15,23,42,0.96)] shadow-2xl"
      >
        <div className="flex items-center gap-3 border-b border-white/10 px-4 py-3">
          <Search className="h-5 w-5 shrink-0 text-indigo-300" aria-hidden="true" />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setActive(0);
            }}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setActive((index) => Math.min(index + 1, Math.max(results.length - 1, 0)));
              } else if (event.key === "ArrowUp") {
                event.preventDefault();
                setActive((index) => Math.max(index - 1, 0));
              } else if (event.key === "Enter" && results[active]) {
                event.preventDefault();
                select(String(results[active].id));
              }
            }}
            placeholder="Search titles, headings, and body"
            className="w-full bg-transparent text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none"
            aria-controls="search-results"
          />
          <button type="button" onClick={onClose} className="rounded p-1 text-slate-400 hover:text-white" aria-label="Close search">
            <X className="h-4 w-4" />
          </button>
        </div>
        <ul id="search-results" role="listbox" className="max-h-96 overflow-y-auto p-2">
          {query.trim() && results.length === 0 ? (
            <li className="px-3 py-8 text-center text-sm text-slate-500">No matches for “{query}”.</li>
          ) : (
            results.map((item, index) => (
              <li key={String(item.id)} role="option" aria-selected={index === active}>
                <button
                  type="button"
                  onClick={() => select(String(item.id))}
                  className={`w-full rounded-lg px-3 py-2.5 text-left ${
                    index === active ? "bg-indigo-500/15 ring-1 ring-indigo-400/30" : "hover:bg-white/5"
                  }`}
                >
                  <p className="text-xs font-medium text-indigo-300">{String(item.category)}</p>
                  <p className="text-sm font-semibold text-slate-100">{String(item.title)}</p>
                  <p className="line-clamp-1 text-xs text-slate-400">{String(item.summary)}</p>
                </button>
              </li>
            ))
          )}
        </ul>
        <p className="border-t border-white/10 px-4 py-2 text-[11px] text-slate-500">
          Title ranks above headings and body. Offline MiniSearch index. Esc closes.
        </p>
      </div>
    </div>
  );
}
