import fs from 'fs';
import path from 'path';
import matter from 'gray-matter';
import GithubSlugger from 'github-slugger';

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

/** A Markdown link that resolves to a real file outside the curated corpus
 * (`docs/moon/archive|research|reports/**`). Not a build error — recorded
 * so a future "not published" notice can be rendered instead of a 404. */
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

export interface SearchDoc {
  id: string;
  title: string;
  category: string;
  summary: string;
  content: string;
}

interface Frontmatter {
  title?: string;
  description?: string;
  nav_group?: string;
  order?: number;
  draft?: boolean;
}

const DOCS_ROOT = path.resolve(process.cwd(), '../../docs');
const OUTPUT_DIR = path.resolve(process.cwd(), 'src/content');

/**
 * The curated publish corpus (roadmap: "It excludes docs/moon/archive/,
 * docs/moon/research/, and docs/moon/reports/ ... New Markdown is
 * unpublished until it matches a glob above or is added to this table.").
 * Deliberately explicit rather than "walk everything except a denylist" —
 * a new top-level docs/ subdirectory is unpublished by default, not
 * accidentally public.
 */
export function curatedCorpusFiles(docsRoot: string = DOCS_ROOT): string[] {
  const files: string[] = [];

  for (const entry of fs.readdirSync(docsRoot, { withFileTypes: true })) {
    if (entry.isFile() && entry.name.endsWith('.md')) {
      files.push(entry.name);
    }
  }

  const adrDir = path.join(docsRoot, 'adr');
  if (fs.existsSync(adrDir)) {
    const walk = (dir: string, base: string): void => {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
          walk(full, base);
        } else if (entry.isFile() && entry.name.endsWith('.md')) {
          files.push(path.relative(base, full));
        }
      }
    };
    walk(adrDir, docsRoot);
  }

  for (const named of ['moon/ROADMAP.md', 'moon/CHANGELOG.md']) {
    if (fs.existsSync(path.join(docsRoot, named))) {
      files.push(named);
    }
  }

  const roadmapsDir = path.join(docsRoot, 'moon', 'roadmaps');
  if (fs.existsSync(roadmapsDir)) {
    for (const entry of fs.readdirSync(roadmapsDir, { withFileTypes: true })) {
      if (entry.isFile() && entry.name.endsWith('.md')) {
        files.push(path.join('moon', 'roadmaps', entry.name));
      }
    }
  }

  return files;
}

export function getCategoryAndOrder(relativePath: string): { category: string; order: number; titleFallback: string } {
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

/** Slug = path relative to docs/ without the `.md` suffix, slashes
 * preserved (`docs/moon/roadmaps/ui.md` -> `moon/roadmaps/ui`), matching
 * the roadmap's locked example. Lowercased for URL/case consistency —
 * the roadmap's own example filenames are already lowercase, so this is
 * a non-breaking clarification, not a deviation. */
export function slugFor(relativePath: string): string {
  return relativePath.replace(/\.md$/, '').split(path.sep).join('/').toLowerCase();
}

export function extractHeaders(body: string): DocHeader[] {
  const slugger = new GithubSlugger();
  const headers: DocHeader[] = [];
  let inCodeFence = false;
  for (const rawLine of body.split('\n')) {
    const line = rawLine.trim();
    if (line.startsWith('```')) {
      inCodeFence = !inCodeFence;
      continue;
    }
    if (inCodeFence) continue;
    const match = /^(#{1,6})\s+(.+)$/.exec(line);
    if (!match) continue;
    const level = match[1].length;
    const text = match[2].replace(/\s+#+$/, '').trim();
    headers.push({ id: slugger.slug(text), text, level });
  }
  return headers;
}

export function extractTitle(body: string, fallback: string): string {
  const h1 = /^#\s+(.+)$/m.exec(body);
  if (h1) return h1[1].trim();
  return fallback.replace(/_/g, ' ').replace(/-/g, ' ');
}

export function extractSummary(body: string, fallback: string): string {
  for (const rawLine of body.split('\n')) {
    const line = rawLine.trim();
    if (line.length > 20 && !line.startsWith('#') && !line.startsWith('>') && !line.startsWith('!') && !line.startsWith('```')) {
      return line.length > 160 ? `${line.slice(0, 160)}...` : line;
    }
  }
  return fallback;
}

const MD_LINK = /\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;

export interface ParsedDoc {
  slug: string;
  relativePath: string;
  dir: string;
  frontmatter: Frontmatter;
  body: string;
  headers: DocHeader[];
  title: string;
  description?: string;
  summary: string;
  category: string;
  order: number;
}

export function parseDoc(docsRoot: string, relativePath: string): ParsedDoc {
  const filePath = path.join(docsRoot, relativePath);
  const raw = fs.readFileSync(filePath, 'utf-8');
  const { data, content: body } = matter(raw);
  const frontmatter = data as Frontmatter;

  const { category, order, titleFallback } = getCategoryAndOrder(relativePath);
  const title = frontmatter.title ?? extractTitle(body, titleFallback);
  const headers = extractHeaders(body);
  const summary = frontmatter.description ?? extractSummary(body, `${title} documentation for Coding-Assistants`);

  return {
    slug: slugFor(relativePath),
    relativePath,
    dir: path.dirname(relativePath),
    frontmatter,
    body,
    headers,
    title,
    description: frontmatter.description,
    summary,
    category: frontmatter.nav_group ?? category,
    order: frontmatter.order ?? order,
  };
}

/** Resolves a Markdown link target to a `docs/`-relative posix path, or
 * `null` for external/mailto/pure-anchor links this validator ignores. */
export function resolveLinkTarget(fromDir: string, target: string): string | null {
  if (/^[a-z][a-z0-9+.-]*:/i.test(target)) return null; // scheme:// or mailto:
  if (target.startsWith('#')) return null; // same-page anchor, checked separately
  const [filePart] = target.split('#');
  if (!filePart || !filePart.endsWith('.md')) return null;
  const resolved = path.normalize(path.join(fromDir, filePart));
  return resolved.split(path.sep).join('/');
}

function linkAnchor(target: string): string | null {
  const hashIndex = target.indexOf('#');
  return hashIndex === -1 ? null : target.slice(hashIndex + 1);
}

export interface ValidationResult {
  brokenLinks: string[];
  brokenAnchors: string[];
  unpublishedLinks: UnpublishedLink[];
  rewritten: Map<string, string>;
}

/** Validates and rewrites in-corpus Markdown links to HashRouter paths.
 * `fileExists` checks a `docs/`-relative path (which may escape `docs/`
 * via `../`, e.g. a link to the repo-root `android/README.md`) against the
 * real filesystem, distinguishing a genuinely broken link from one that
 * points at a real file the site simply doesn't publish (archive/research/
 * reports, or something outside `docs/` entirely). */
export function validateAndRewriteLinks(
  docs: ParsedDoc[],
  slugSet: Set<string>,
  headerIdsBySlug: Map<string, Set<string>>,
  fileExists: (relativePath: string) => boolean,
): ValidationResult {
  const brokenLinks: string[] = [];
  const brokenAnchors: string[] = [];
  const unpublishedLinks: UnpublishedLink[] = [];
  const rewritten = new Map<string, string>();

  for (const doc of docs) {
    let changed = false;

    const body = doc.body.replace(MD_LINK, (full, text, target) => {
      if (target.startsWith('#')) {
        const ownHeaders = headerIdsBySlug.get(doc.slug) ?? new Set<string>();
        if (!ownHeaders.has(target.slice(1))) {
          brokenAnchors.push(`${doc.relativePath}: same-page anchor "${target}" has no matching heading`);
        }
        return full;
      }

      const resolved = resolveLinkTarget(doc.dir, target);
      if (resolved === null) return full;

      const anchor = linkAnchor(target);
      const resolvedSlug = slugFor(resolved);
      if (slugSet.has(resolvedSlug)) {
        if (anchor) {
          const targetHeaders = headerIdsBySlug.get(resolvedSlug) ?? new Set<string>();
          if (!targetHeaders.has(anchor)) {
            brokenAnchors.push(`${doc.relativePath}: "${target}" anchor "#${anchor}" has no matching heading in ${resolvedSlug}`);
          }
        }
        changed = true;
        return `[${text}](/#/docs/${resolvedSlug}${anchor ? `#${anchor}` : ''})`;
      }

      if (fileExists(resolved)) {
        unpublishedLinks.push({ fromSlug: doc.slug, targetPath: resolved });
        return full;
      }

      brokenLinks.push(`${doc.relativePath}: "${target}" does not resolve to any real file`);
      return full;
    });

    if (changed) rewritten.set(doc.slug, body);
  }

  return { brokenLinks, brokenAnchors, unpublishedLinks, rewritten };
}

/** True when `relativePath` (relative to `docsRoot`, may escape it via
 * `../`) resolves to a real file on disk. */
export function makeFileExistsChecker(docsRoot: string = DOCS_ROOT): (relativePath: string) => boolean {
  return (relativePath: string) => fs.existsSync(path.join(docsRoot, relativePath));
}

export function buildContent(): void {
  console.log('⚡ Building documentation content manifest...');
  if (!fs.existsSync(OUTPUT_DIR)) {
    fs.mkdirSync(OUTPUT_DIR, { recursive: true });
  }

  const corpusFiles = curatedCorpusFiles();
  const docs = corpusFiles.map((relativePath) => parseDoc(DOCS_ROOT, relativePath));

  const draftDocs = docs.filter((doc) => doc.frontmatter.draft);
  if (draftDocs.length > 0) {
    console.error('❌ Draft pages are not allowed in the production content build:');
    for (const doc of draftDocs) console.error(`   - ${doc.relativePath}`);
    process.exit(1);
  }

  const slugSet = new Set(docs.map((doc) => doc.slug));
  const headerIdsBySlug = new Map(docs.map((doc) => [doc.slug, new Set(doc.headers.map((h) => h.id))]));

  const { brokenLinks, brokenAnchors, unpublishedLinks, rewritten } = validateAndRewriteLinks(
    docs,
    slugSet,
    headerIdsBySlug,
    makeFileExistsChecker(),
  );

  if (brokenLinks.length > 0 || brokenAnchors.length > 0) {
    console.error('❌ Content validation failed:');
    for (const message of [...brokenLinks, ...brokenAnchors]) console.error(`   - ${message}`);
    process.exit(1);
  }

  const docsRecord: Record<string, DocMetadata> = {};
  const categoryMap: Record<string, { order: number; docs: { slug: string; title: string; summary: string }[] }> = {};
  const searchDocs: SearchDoc[] = [];

  for (const doc of docs) {
    const content = rewritten.get(doc.slug) ?? doc.body;
    docsRecord[doc.slug] = {
      slug: doc.slug,
      title: doc.title,
      description: doc.description,
      category: doc.category,
      order: doc.order,
      filePath: doc.relativePath,
      summary: doc.summary,
      headers: doc.headers,
      content,
    };

    if (!categoryMap[doc.category]) {
      categoryMap[doc.category] = { order: doc.order, docs: [] };
    }
    categoryMap[doc.category].docs.push({ slug: doc.slug, title: doc.title, summary: doc.summary });

    searchDocs.push({
      id: doc.slug,
      title: doc.title,
      category: doc.category,
      summary: doc.summary,
      content: [doc.title, ...doc.headers.map((h) => h.text), content].join('\n').slice(0, 20_000),
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
    unpublishedLinks,
    updatedAt: new Date().toISOString(),
  };

  fs.writeFileSync(path.join(OUTPUT_DIR, 'docs-manifest.json'), JSON.stringify(manifest, null, 2));
  fs.writeFileSync(path.join(OUTPUT_DIR, 'search-index.json'), JSON.stringify(searchDocs, null, 2));

  console.log(`✅ Content manifest generated: ${docs.length} documents across ${categories.length} categories.`);
  if (unpublishedLinks.length > 0) {
    console.log(`ℹ️  ${unpublishedLinks.length} link(s) point at real but unpublished docs (archive/research/reports) — left unrewritten.`);
  }
}

const isMainModule = process.argv[1] !== undefined && path.resolve(process.argv[1]) === path.resolve(new URL(import.meta.url).pathname);
if (isMainModule) {
  buildContent();
}
