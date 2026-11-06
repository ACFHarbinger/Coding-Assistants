import React from 'react';
import { useParams } from 'react-router-dom';
import { DocsSidebar } from './DocsSidebar';
import { TableOfContents } from './TableOfContents';
import { MarkdownArticle } from './MarkdownArticle';
import { PrevNextNav } from './PrevNextNav';
import { NotFoundPage } from '../errors/NotFoundPage';
import docsManifestData from '../../content/docs-manifest.json';
import { DocMetadata, UnpublishedLink } from '../../types';

export const DocsLayout: React.FC = () => {
  // Nested slugs (`moon/roadmaps/ui`) arrive via the `/docs/*` splat route,
  // not a named `:slug` param — react-router puts wildcard matches in `*`.
  const params = useParams();
  const activeSlug = params.slug || params['*'] || 'documentation_standards';

  const docsRecord = docsManifestData.docs as Record<string, DocMetadata>;
  const doc = docsRecord[activeSlug];
  const unpublishedLinks = (docsManifestData.unpublishedLinks as UnpublishedLink[])
    .filter((link) => link.fromSlug === activeSlug)
    .map((link) => link.targetPath);

  if (!doc) {
    return <NotFoundPage />;
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 flex gap-8">
      <DocsSidebar />

      <div className="flex-1 min-w-0 py-2">
        <div className="mb-6 flex items-center justify-between border-b border-[var(--glass-border)] pb-4">
          <div>
            <span className="text-xs font-semibold text-indigo-400 uppercase tracking-wider">
              {doc.category}
            </span>
            <h1 className="text-3xl font-bold text-[var(--text-primary)] mt-1">{doc.title}</h1>
          </div>
        </div>

        <MarkdownArticle
          content={doc.content}
          unpublishedLinks={unpublishedLinks}
        />

        <PrevNextNav currentSlug={activeSlug} />
      </div>

      <TableOfContents headers={doc.headers} />
    </div>
  );
};
