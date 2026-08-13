import MiniSearch from "minisearch";

export interface SearchDoc {
  id: string;
  title: string;
  category: string;
  summary: string;
  content: string;
}

export const SEARCH_FIELDS = ["title", "category", "summary", "content"] as const;

export const SEARCH_BOOST: Record<(typeof SEARCH_FIELDS)[number], number> = {
  title: 4,
  summary: 2,
  category: 1.5,
  content: 1,
};

export function createDocSearch(documents: SearchDoc[]): MiniSearch<SearchDoc> {
  const search = new MiniSearch<SearchDoc>({
    fields: [...SEARCH_FIELDS],
    storeFields: ["title", "category", "summary"],
    searchOptions: {
      boost: SEARCH_BOOST,
      fuzzy: 0.2,
      prefix: true,
    },
  });
  search.addAll(documents);
  return search;
}

export function rankQuery(search: MiniSearch<SearchDoc>, query: string, limit = 8) {
  if (!query.trim()) return [];
  return search.search(query).slice(0, limit);
}
