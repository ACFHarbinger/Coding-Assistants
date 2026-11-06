export interface ActivityFilter {
  agent?: string | null;
  since?: string | null;
  until?: string | null;
  kind?: string | null;
  workspace_path?: string | null;
  limit?: number | null;
}

export interface ActivityFileTouch {
  path: string;
  operation: string;
  observed_at: string;
  content_hash?: string | null;
  status: string;
}

export interface ActivityCommandRun {
  cmdline: string[];
  raw: string;
  exe?: string | null;
  observed_at: string;
  attribution?: string | null;
  source: string;
}

export interface ActivityItem {
  id: string;
  kind: "task" | "work_session" | "workspace" | string;
  title: string;
  status?: string | null;
  workspace_path?: string | null;
  started_at: string;
  updated_at: string;
  agents: string[];
  commands: ActivityCommandRun[];
  files: ActivityFileTouch[];
  message_count: number;
  capture_count: number;
}
