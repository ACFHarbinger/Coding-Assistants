import { invoke } from "../../../lib/tauri";
import type { MemoryRecord, ScoredMemoryRecord } from "./types";

export interface SearchMemoriesOptions {
  limit?: number;
  scope?: string | null;
  tier?: string | null;
  workspace?: string | null;
}

/**
 * Searches memories using hybrid retrieval (lexical + similarity vector scoring).
 * Uses similarity ranking (wording: "smart/similarity").
 */
export async function searchMemoriesHybrid(
  query: string,
  options?: SearchMemoriesOptions,
): Promise<ScoredMemoryRecord[]> {
  return invoke<ScoredMemoryRecord[]>("hub_search_memories_hybrid", {
    query,
    limit: options?.limit ?? 20,
    scope: options?.scope ?? null,
    tier: options?.tier ?? null,
    workspace: options?.workspace ?? null,
  });
}

/**
 * Searches memories using vector similarity scoring.
 */
export async function searchMemoriesSemantic(
  query: string,
  options?: SearchMemoriesOptions,
): Promise<ScoredMemoryRecord[]> {
  return invoke<ScoredMemoryRecord[]>("hub_search_memories_semantic", {
    query,
    limit: options?.limit ?? 20,
    scope: options?.scope ?? null,
    tier: options?.tier ?? null,
    workspace: options?.workspace ?? null,
  });
}

/**
 * Exact lexical search via LIKE query.
 */
export async function searchMemoriesExact(query: string): Promise<MemoryRecord[]> {
  return invoke<MemoryRecord[]>("hub_search_memories", { query });
}

/**
 * Re-indexes missing memory vector embeddings.
 */
export async function reindexMemoryVectors(): Promise<number> {
  return invoke<number>("hub_reindex_memory_vectors");
}
