export interface DocHeader {
  id: string;
  text: string;
  level: number;
}

export interface DocMetadata {
  slug: string;
  title: string;
  description?: string;
  category: string;
  order: number;
  filePath: string;
  summary: string;
  headers: DocHeader[];
  content: string;
}

export interface CategoryGroup {
  name: string;
  order: number;
  docs: { slug: string; title: string; summary: string }[];
}

/** A Markdown link that resolves to a real file outside the curated
 * corpus (archive/research/reports) — recorded instead of rewritten. */
export interface UnpublishedLink {
  fromSlug: string;
  targetPath: string;
}

export interface DocsManifest {
  categories: CategoryGroup[];
  docs: Record<string, DocMetadata>;
  unpublishedLinks: UnpublishedLink[];
  updatedAt: string;
}
