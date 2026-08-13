export interface DocHeader {
  id: string;
  text: string;
  level: number;
}

export interface DocMetadata {
  slug: string;
  title: string;
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

export interface DocsManifest {
  categories: CategoryGroup[];
  docs: Record<string, DocMetadata>;
  updatedAt: string;
}
