import React from 'react';
import { useParams, Navigate } from 'react-router-dom';
import { DocsSidebar } from './DocsSidebar';
import { TableOfContents } from './TableOfContents';
import { MarkdownArticle } from './MarkdownArticle';
import { PrevNextNav } from './PrevNextNav';
import docsManifestData from '../../content/docs-manifest.json';
import { DocMetadata } from '../../types';

export const DocsLayout: React.FC = () => {
  const { slug } = useParams<{ slug?: string }>();
  const activeSlug = slug || 'documentation_standards';

  const docsRecord = docsManifestData.docs as Record<string, DocMetadata>;
  const doc = docsRecord[activeSlug];

  if (!doc) {
    return <Navigate to="/docs/documentation_standards" replace />;
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 flex gap-8">
      <DocsSidebar />

      <div className="flex-1 min-w-0 py-2">
        <div className="mb-6 flex items-center justify-between border-b border-slate-800 pb-4">
          <div>
            <span className="text-xs font-semibold text-cyan-400 uppercase tracking-wider">
              {doc.category}
            </span>
            <h1 className="text-3xl font-bold text-slate-100 mt-1">{doc.title}</h1>
          </div>
        </div>

        <MarkdownArticle content={doc.content} />

        <PrevNextNav currentSlug={activeSlug} />
      </div>

      <TableOfContents headers={doc.headers} />
    </div>
  );
};
