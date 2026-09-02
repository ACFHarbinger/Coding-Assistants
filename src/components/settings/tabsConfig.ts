export type TabId = "general" | "workspace" | "agents" | "creative" | "orchestration" | "memory" | "diagnostics" | "danger";

export interface TabDef {
  id: TabId;
  label: string;
  summary: string;
  dangerous?: boolean;
  implemented: boolean;
}

export const TABS: TabDef[] = [
  { id: "general", label: "General", summary: "App-wide defaults, such as which workspace opens on launch.", implemented: true },
  { id: "workspace", label: "Workspace & sessions", summary: "Global default vs. this workspace's default chat session.", implemented: true },
  { id: "agents", label: "Agents & harnesses", summary: "Named provider profiles and per-harness settings.", implemented: true },
  { id: "creative", label: "Creative Tools", summary: "Expose local creative app MCP bridges (Blender, Krita, Godot, etc.) to coding agents.", implemented: true },
  { id: "orchestration", label: "Orchestration", summary: "Task/wake confirmation, auto-enrollment, budgets, tool/sandbox policy.", implemented: true },
  { id: "memory", label: "Memory & storage", summary: "Retention, export, and settings-backup policy.", implemented: true },
  { id: "diagnostics", label: "Diagnostics", summary: "Log level, configuration health, redacted diagnostics export.", implemented: true },
  { id: "danger", label: "Danger zone", summary: "Confirmed reset, removal, and purge operations.", dangerous: true, implemented: true },
];
