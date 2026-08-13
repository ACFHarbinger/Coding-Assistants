import { useEffect, useState, type ReactNode } from "react";
import { Link, useLocation } from "react-router-dom";
import { Search, Menu, X, Github } from "lucide-react";
import { BrandMark } from "../assets/svgs/BrandMark";
import { ThemeToggle } from "../components/ThemeToggle";
import { CommandPalette } from "../features/navigation/CommandPalette";

const NAV_LINKS = [
  { label: "Home", path: "/" },
  { label: "Docs", path: "/docs" },
  { label: "Roadmap", path: "/docs/moon/roadmaps/documentation" },
  { label: "Changelog", path: "/docs/changelog" },
] as const;

function linkActive(pathname: string, path: string) {
  return pathname === path || (path !== "/" && pathname.startsWith(path));
}

export function AppShell({ children }: { children: ReactNode }) {
  const location = useLocation();
  const [searchOpen, setSearchOpen] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    setMenuOpen(false);
  }, [location.pathname]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && menuOpen && !searchOpen) {
        setMenuOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [menuOpen, searchOpen]);

  return (
    <div className="flex min-h-screen flex-col bg-[var(--bg-primary)] text-[var(--text-primary)] selection:bg-indigo-500/30 selection:text-indigo-100">
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-indigo-600 focus:px-3 focus:py-2 focus:text-white"
      >
        Skip to content
      </a>
      <header className="glass-panel sticky top-0 z-40 w-full border-b motion-reduce:backdrop-blur-none">
        <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8">
          <div className="flex items-center space-x-3">
            <Link to="/" className="group flex items-center space-x-2.5">
              <BrandMark className="h-9 w-9" />
              <span className="text-lg font-bold tracking-tight group-hover:text-indigo-400">
                Coding-Assistants
              </span>
            </Link>
            <span className="hidden rounded-full border border-indigo-500/30 bg-indigo-500/10 px-2.5 py-0.5 text-xs font-medium text-indigo-300 md:inline-block">
              v0.1.0 (Hub-Native)
            </span>
          </div>

          <nav className="hidden items-center space-x-1 md:flex" aria-label="Primary">
            {NAV_LINKS.map((link) => {
              const isActive = linkActive(location.pathname, link.path);
              return (
                <Link
                  key={link.path}
                  to={link.path}
                  className={`rounded-md px-3.5 py-2 text-sm font-medium transition-colors motion-reduce:transition-none ${
                    isActive
                      ? "border border-indigo-500/30 bg-indigo-500/10 text-indigo-300"
                      : "text-[var(--text-secondary)] hover:bg-white/5 hover:text-[var(--text-primary)]"
                  }`}
                >
                  {link.label}
                </Link>
              );
            })}
          </nav>

          <div className="flex items-center space-x-2.5">
            <button
              type="button"
              onClick={() => {
                setSearchOpen(true);
                setMenuOpen(false);
              }}
              className="flex items-center space-x-2 rounded-lg border border-[var(--glass-border)] px-3 py-1.5 text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
              aria-label="Search documentation"
              aria-keyshortcuts="Control+K Meta+K"
            >
              <Search className="h-3.5 w-3.5" aria-hidden="true" />
              <span className="hidden sm:inline">Search...</span>
              <kbd className="hidden rounded border border-[var(--glass-border)] px-1.5 py-0.5 font-mono text-[10px] sm:inline-block">
                ⌘K
              </kbd>
            </button>
            <ThemeToggle />
            <a
              href="https://github.com/ACFHarbinger/Coding-Assistants"
              target="_blank"
              rel="noopener noreferrer"
              className="hidden rounded-lg border border-[var(--glass-border)] p-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] sm:flex"
              aria-label="GitHub repository"
            >
              <Github className="h-4 w-4" />
            </a>
            <button
              type="button"
              onClick={() => setMenuOpen((open) => !open)}
              className="rounded-lg p-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] md:hidden"
              aria-label="Toggle navigation menu"
              aria-expanded={menuOpen}
              aria-controls="mobile-nav"
            >
              {menuOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
            </button>
          </div>
        </div>

        {menuOpen && (
          <nav id="mobile-nav" aria-label="Mobile" className="space-y-1 border-t border-[var(--glass-border)] px-4 pb-4 pt-2 md:hidden">
            {NAV_LINKS.map((link) => {
              const isActive = linkActive(location.pathname, link.path);
              return (
                <Link
                  key={link.path}
                  to={link.path}
                  className={`block rounded-md px-3 py-2 text-base font-medium ${
                    isActive
                      ? "bg-indigo-500/10 text-indigo-300"
                      : "text-[var(--text-secondary)] hover:bg-white/5 hover:text-[var(--text-primary)]"
                  }`}
                >
                  {link.label}
                </Link>
              );
            })}
          </nav>
        )}
      </header>

      <main id="main-content" className="flex-1">
        {children}
      </main>

      <CommandPalette
        isOpen={searchOpen}
        onClose={() => setSearchOpen(false)}
        onOpen={() => setSearchOpen(true)}
      />

      <footer className="border-t border-[var(--glass-border)] py-8 text-center text-xs text-[var(--text-muted)]">
        <div className="mx-auto flex max-w-7xl flex-col items-center justify-between gap-4 px-4 sm:flex-row">
          <p>
            © 2026 Coding-Assistants. Licensed under{" "}
            <a className="underline hover:text-indigo-400" href="https://github.com/ACFHarbinger/Coding-Assistants/blob/main/LICENSE">
              AGPL-3.0
            </a>
            .
          </p>
          <div className="flex items-center space-x-4">
            <Link to="/docs" className="hover:text-indigo-400">Documentation</Link>
            <Link to="/docs/moon/roadmaps/documentation" className="hover:text-indigo-400">Roadmap</Link>
            <a href="https://github.com/ACFHarbinger/Coding-Assistants" target="_blank" rel="noopener noreferrer" className="hover:text-indigo-400">
              GitHub
            </a>
          </div>
        </div>
      </footer>
    </div>
  );
}
