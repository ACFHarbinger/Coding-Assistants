import fs from 'fs';
import path from 'path';

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

const DOCS_ROOT = path.resolve(process.cwd(), '../../docs');
const OUTPUT_DIR = path.resolve(process.cwd(), 'src/content');

function getCategoryAndOrder(relativePath: string): { category: string; order: number; titleFallback: string } {
  if (relativePath.startsWith('moon/roadmaps/')) {
    return { category: 'Capability Roadmaps', order: 4, titleFallback: path.basename(relativePath, '.md') };
  }
  if (relativePath.startsWith('moon/')) {
    return { category: 'Project Status & Moon', order: 3, titleFallback: path.basename(relativePath, '.md') };
  }
  if (relativePath.startsWith('adr/')) {
    return { category: 'Architecture Decision Records', order: 2, titleFallback: path.basename(relativePath, '.md') };
  }
  return { category: 'Core & Architecture', order: 1, titleFallback: path.basename(relativePath, '.md') };
}

function parseMarkdown(filePath: string, relativePath: string): DocMetadata {
  const content = fs.readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');

  let title = '';
  let summary = '';
  const headers: DocHeader[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (!title && line.startsWith('# ')) {
      title = line.replace('# ', '').trim();
    } else if (line.startsWith('## ') || line.startsWith('### ')) {
      const level = line.startsWith('## ') ? 2 : 3;
      const text = line.replace(/^###?\s+/, '').trim();
      const id = text.toLowerCase().replace(/[^\w\s-]/g, '').replace(/\s+/g, '-');
      headers.push({ id, text, level });
    } else if (!summary && line.length > 20 && !line.startsWith('#') && !line.startsWith('>') && !line.startsWith('!')) {
      summary = line.substring(0, 160) + (line.length > 160 ? '...' : '');
    }
  }

  const { category, order, titleFallback } = getCategoryAndOrder(relativePath);
  if (!title) {
    title = titleFallback.replace(/_/g, ' ').replace(/-/g, ' ');
  }

  const slug = relativePath
    .replace(/\.md$/, '')
    .toLowerCase()
    .replace(/\//g, '-');

  return {
    slug,
    title,
    category,
    order,
    filePath: relativePath,
    summary: summary || `${title} documentation for Coding-Assistants`,
    headers,
    content,
  };
}

function scanDocs(dir: string, baseDir = dir): DocMetadata[] {
  const results: DocMetadata[] = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    const relativePath = path.relative(baseDir, fullPath);

    if (entry.isDirectory()) {
      if (entry.name !== 'website' && entry.name !== 'node_modules' && !entry.name.startsWith('.')) {
        results.push(...scanDocs(fullPath, baseDir));
      }
    } else if (entry.isFile() && entry.name.endsWith('.md')) {
      results.push(parseMarkdown(fullPath, relativePath));
    }
  }

  return results;
}

export function buildContent() {
  console.log('⚡ Building documentation content manifest...');
  if (!fs.existsSync(OUTPUT_DIR)) {
    fs.mkdirSync(OUTPUT_DIR, { recursive: true });
  }

  const docList = scanDocs(DOCS_ROOT);
  const docsRecord: Record<string, DocMetadata> = {};
  const categoryMap: Record<string, { order: number; docs: { slug: string; title: string; summary: string }[] }> = {};

  for (const doc of docList) {
    docsRecord[doc.slug] = doc;
    if (!categoryMap[doc.category]) {
      categoryMap[doc.category] = { order: doc.order, docs: [] };
    }
    categoryMap[doc.category].docs.push({
      slug: doc.slug,
      title: doc.title,
      summary: doc.summary,
    });
  }

  const categories: CategoryGroup[] = Object.entries(categoryMap)
    .map(([name, data]) => ({
      name,
      order: data.order,
      docs: data.docs.sort((a, b) => a.title.localeCompare(b.title)),
    }))
    .sort((a, b) => a.order - b.order);

  const manifest: DocsManifest = {
    categories,
    docs: docsRecord,
    updatedAt: new Date().toISOString(),
  };

  fs.writeFileSync(path.join(OUTPUT_DIR, 'docs-manifest.json'), JSON.stringify(manifest, null, 2));

  const searchDocs = docList.map((doc) => ({
    id: doc.slug,
    title: doc.title,
    category: doc.category,
    summary: doc.summary,
    content: doc.content.substring(0, 5000),
  }));

  fs.writeFileSync(path.join(OUTPUT_DIR, 'search-index.json'), JSON.stringify(searchDocs, null, 2));

  console.log(`✅ Content manifest generated successfully with ${docList.length} documents across ${categories.length} categories.`);
}

buildContent();
