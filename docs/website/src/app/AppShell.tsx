import React, { useState } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { Search, Menu, X, Github } from 'lucide-react';
import { BrandMark } from '../assets/brand/BrandMark';
import { ThemeToggle } from '../components/ThemeToggle';
import { CommandPalette } from '../features/search/CommandPalette';

export const AppShell: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const location = useLocation();
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

  const navLinks = [
    { label: 'Home', path: '/' },
    { label: 'Docs', path: '/docs' },
    { label: 'Roadmap', path: '/docs/moon-roadmaps-documentation' },
    { label: 'Changelog', path: '/docs/changelog' },
  ];

  return (
    <div className="min-h-screen flex flex-col bg-[#020617] text-slate-100 selection:bg-indigo-500/30 selection:text-indigo-100">
      <a href="#main-content" className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-indigo-600 focus:px-3 focus:py-2 focus:text-white">
        Skip to content
      </a>
      <header className="sticky top-0 z-40 w-full glass-panel border-b border-slate-800/80 bg-slate-950/80 backdrop-blur-md">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <Link to="/" className="flex items-center space-x-2.5 group">
              <BrandMark className="h-9 w-9" />
              <span className="font-bold text-lg text-slate-100 tracking-tight group-hover:text-cyan-400 transition-colors">
                Coding-Assistants
              </span>
            </Link>
            <span className="hidden md:inline-block px-2.5 py-0.5 text-xs font-medium rounded-full bg-cyan-950/60 text-cyan-400 border border-cyan-800/40">
              v0.1.0 (Hub-Native)
            </span>
          </div>

          <nav className="hidden md:flex items-center space-x-1">
            {navLinks.map((link) => {
              const isActive = location.pathname === link.path || (link.path !== '/' && location.pathname.startsWith(link.path));
              return (
                <Link
                  key={link.path}
                  to={link.path}
                  className={`px-3.5 py-2 rounded-md text-sm font-medium transition-colors ${
                    isActive
                      ? 'bg-slate-800/80 text-cyan-400 border border-cyan-500/20'
                      : 'text-slate-300 hover:text-white hover:bg-slate-800/50'
                  }`}
                >
                  {link.label}
                </Link>
              );
            })}
          </nav>

          <div className="flex items-center space-x-2.5">
            <button
              onClick={() => setIsSearchOpen(true)}
              className="flex items-center space-x-2 px-3 py-1.5 rounded-lg bg-slate-900/80 text-slate-400 hover:text-slate-200 border border-slate-800 hover:border-slate-700 text-xs transition-all"
              aria-label="Search Documentation"
            >
              <Search className="w-3.5 h-3.5" />
              <span className="hidden sm:inline">Search...</span>
              <kbd className="hidden sm:inline-block px-1.5 py-0.5 text-[10px] rounded bg-slate-800 text-slate-400 border border-slate-700 font-mono">
                ⌘K
              </kbd>
            </button>

            <ThemeToggle />

            <a
              href="https://github.com/ACFHarbinger/Coding-Assistants"
              target="_blank"
              rel="noopener noreferrer"
              className="p-2 rounded-lg bg-slate-900/80 text-slate-400 hover:text-white border border-slate-800 hover:border-slate-700 transition-colors hidden sm:flex"
              aria-label="GitHub Repository"
            >
              <Github className="w-4 h-4" />
            </a>

            <button
              onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
              className="p-2 rounded-lg text-slate-400 hover:text-white md:hidden"
              aria-label="Toggle Mobile Menu"
            >
              {isMobileMenuOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
            </button>
          </div>
        </div>

        {isMobileMenuOpen && (
          <div className="md:hidden border-b border-slate-800 bg-slate-950 px-4 pt-2 pb-4 space-y-1">
            {navLinks.map((link) => (
              <Link
                key={link.path}
                to={link.path}
                onClick={() => setIsMobileMenuOpen(false)}
                className="block px-3 py-2 rounded-md text-base font-medium text-slate-300 hover:text-white hover:bg-slate-800"
              >
                {link.label}
              </Link>
            ))}
          </div>
        )}
      </header>

      <main id="main-content" className="flex-1">{children}</main>

      <CommandPalette
        isOpen={isSearchOpen}
        onClose={() => setIsSearchOpen(false)}
        onOpen={() => setIsSearchOpen(true)}
      />

      <footer className="border-t border-slate-800/80 bg-slate-950/60 py-8 text-center text-xs text-slate-500">
        <div className="max-w-7xl mx-auto px-4 flex flex-col sm:flex-row items-center justify-between gap-4">
          <p>© 2026 Coding-Assistants. Licensed under <a className="underline hover:text-indigo-300" href="https://github.com/ACFHarbinger/Coding-Assistants/blob/main/LICENSE">AGPL-3.0</a>.</p>
          <div className="flex items-center space-x-4">
            <Link to="/docs" className="hover:text-cyan-400 transition-colors">Documentation</Link>
            <Link to="/docs/moon-roadmaps-communication" className="hover:text-cyan-400 transition-colors">Roadmap</Link>
            <a href="https://github.com/ACFHarbinger/Coding-Assistants" target="_blank" rel="noopener noreferrer" className="hover:text-cyan-400 transition-colors">GitHub</a>
          </div>
        </div>
      </footer>
    </div>
  );
};
