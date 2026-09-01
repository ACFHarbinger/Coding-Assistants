export interface ModelConfig {
  provider: string;
  model: string;
  endpoint?: string;
  prompt_file?: string;
  rule_file?: string;
  workflow_file?: string;
  skill_file?: string;
}

export interface RoleConfig {
  name: string;
  config: ModelConfig;
  origin?: "spawned" | "existing";
  process_pid?: number;
}

export interface AgentConfig {
  roles: RoleConfig[];
  work_dir: string;
  mcp_config: string;
}

export interface AgentResources {
  prompts: string[];
  rules: string[];
  workflows: string[];
  skills?: string[];
}

export interface TeamMember {
  id: string;
  target_id: string;
  name: string;
  provider: string;
  model: string;
  origin: "spawned" | "existing";
}

export interface WorkSession {
  id: string;
  name: string;
  created_at: string;
  member_ids: string[];
}

export interface DetectedProcess {
  pid: number;
  agent: string;
  provider: string;
  model: string;
  command: string;
}

export function processTargetId(process: Pick<DetectedProcess, "agent" | "pid">): string {
  const normalized = process.agent.toLowerCase();
  if (normalized === "codex" || normalized === "chatgpt") return "chat";
  if (normalized === "claude") return "claude";
  if (normalized === "gemini") return "gemini";
  if (normalized === "grok") return "grok";
  return `process:${process.pid}`;
}
