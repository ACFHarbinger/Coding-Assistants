import React from 'react';
import { Link } from 'react-router-dom';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import docsManifestData from '../../content/docs-manifest.json';

export const PrevNextNav: React.FC<{ currentSlug: string }> = ({ currentSlug }) => {
  // Flatten docs in category order
  const allDocs = docsManifestData.categories.flatMap((cat) => cat.docs);
  const currentIndex = allDocs.findIndex((doc) => doc.slug === currentSlug);

  if (currentIndex === -1) return null;

  const prevDoc = currentIndex > 0 ? allDocs[currentIndex - 1] : null;
  const nextDoc = currentIndex < allDocs.length - 1 ? allDocs[currentIndex + 1] : null;

  return (
    <div className="mt-12 pt-6 border-t border-[var(--glass-border)] flex items-center justify-between gap-4">
      {prevDoc ? (
        <Link
          to={`/docs/${prevDoc.slug}`}
          className="group flex flex-col items-start p-3 rounded-lg border border-[var(--glass-border)] bg-[var(--glass-bg)] hover:border-indigo-500/30 hover:bg-indigo-500/10 transition-all max-w-[48%]"
        >
          <span className="flex items-center text-xs text-[var(--text-muted)] group-hover:text-indigo-400 mb-1">
            <ChevronLeft className="w-3.5 h-3.5 mr-1" /> Previous
          </span>
          <span className="text-sm font-semibold text-[var(--text-primary)] group-hover:text-indigo-300 truncate w-full">
            {prevDoc.title}
          </span>
        </Link>
      ) : (
        <div />
      )}

      {nextDoc ? (
        <Link
          to={`/docs/${nextDoc.slug}`}
          className="group flex flex-col items-end p-3 rounded-lg border border-[var(--glass-border)] bg-[var(--glass-bg)] hover:border-indigo-500/30 hover:bg-indigo-500/10 transition-all max-w-[48%] ml-auto text-right"
        >
          <span className="flex items-center text-xs text-[var(--text-muted)] group-hover:text-indigo-400 mb-1">
            Next <ChevronRight className="w-3.5 h-3.5 ml-1" />
          </span>
          <span className="text-sm font-semibold text-[var(--text-primary)] group-hover:text-indigo-300 truncate w-full">
            {nextDoc.title}
          </span>
        </Link>
      ) : (
        <div />
      )}
    </div>
  );
};
