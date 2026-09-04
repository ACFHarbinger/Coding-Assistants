import { invoke } from "../../../lib/tauri";
import { loadPersistedRoles } from "../../../app/rolesConfig";
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

export interface ConsolidationReport {
  clusters: number;
  consolidated: number;
  skipped: number;
  notice?: string | null;
}

export interface ConsolidationModelConfig {
  provider: string;
  model: string;
  prompt_file?: string | null;
  rule_file?: string | null;
  workflow_file?: string | null;
  endpoint?: string | null;
}

/**
 * Resolves the default consolidation LLM config from user's configured orchestrator roles.
 */
export function resolveDefaultConsolidationModel(): ConsolidationModelConfig {
  const roles = loadPersistedRoles();
  const configured = roles.find((r) => r.config?.provider && r.config?.model)?.config;
  if (configured) {
    return {
      provider: configured.provider,
      model: configured.model,
      prompt_file: configured.prompt_file ?? null,
      rule_file: configured.rule_file ?? null,
      workflow_file: configured.workflow_file ?? null,
      endpoint: configured.endpoint ?? null,
    };
  }
  return {
    provider: "openai",
    model: "gpt-4o",
    prompt_file: null,
    rule_file: null,
    workflow_file: null,
    endpoint: null,
  };
}

/**
 * Consolidates related short-term memories into episodic summaries via LLM.
 * Uses the user's active/configured orchestrator model by default (#265).
 */
export async function consolidateMemories(
  workspace?: string | null,
  modelConfig?: ConsolidationModelConfig,
): Promise<ConsolidationReport> {
  const config = modelConfig ?? resolveDefaultConsolidationModel();
  return invoke<ConsolidationReport>("hub_consolidate_memories", {
    args: {
      model_config: config,
      workspace: workspace ?? null,
    },
  });
}

