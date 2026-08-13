//! Coding-Assistants shared memory, messaging, and wake primitives.
//!
//! Implements the M1/M2/M3/M4 and C1/C3 spine from
//! `docs/moon/roadmaps/memory.md` and `docs/moon/roadmaps/communication.md`.
//! See [`store::HubStore`] for the concrete SQLite + file-backed
//! implementation; this module just re-exports it as the crate's public API.

mod bridge;
mod harness;
mod paths;
mod settings;
mod store;

pub use bridge::channels::claude::{
    delete_channel_workspace, get_permission_request, is_channel_session_live,
    launch_claude_channel_session, list_channel_workspaces, poll_channel_events,
    poll_quiet_channel_events, record_channel_reply, record_permission_request,
    rename_channel_workspace, resolve_permission_request, setup_claude_channel, ChannelEvent,
    ChannelWorkspace, PermissionVerdict, CLAUDE_AGENT_ID,
};
pub use bridge::claude::{
    claude_control_socket_path, deliver_claude_task, find_active_claude_session,
    list_active_claude_sessions, ClaudeAgentSession,
};
pub use bridge::codex::{deliver_codex_task, latest_codex_thread_id};
pub use bridge::gemini::{deliver_gemini_task, gemini_brain_dir, latest_gemini_session_id};
pub use bridge::grok::{
    acp_initialize, acp_session_load, acp_session_prompt, active_grok_session_for,
    connect_grok_leader_session, default_leader_socket, deliver_grok_task, grok_leader_status,
    latest_grok_session_id, leader_socket_available, list_active_grok_sessions, ActiveGrokSession,
    GrokConnectResult,
};
pub use harness::{
    claude_spawn_args, codex_spawn_args, gemini_managed_spawn_args, gemini_spawn_args,
    grok_spawn_args, inject_harness, inject_harness_with_store, opencode_spawn_args, start_harness,
    vibe_spawn_args, HarnessId, HarnessInjectRequest, HarnessInjectResult, HarnessStartRequest,
    HarnessStartResult,
};
pub use paths::default_hub_home;
pub use settings::{
    EffectiveHarnessSettings, EffectiveOrchestrationPolicy, EffectiveSettings, FieldStatus,
    HarnessSettings, LoadStatus, OrchestrationOverride, OrchestrationPolicy, ProfileSnapshot,
    ProviderProfile, SandboxStrictness, SecretReference, SecretSourceKind, SettingsError,
    SettingsField, SettingsLoad, SettingsSnapshot, SettingsStore, WorkspaceOverride,
    CURRENT_SETTINGS_SCHEMA, DEFAULT_BACKUP_RETENTION, MAX_BACKUP_RETENTION, MIN_BACKUP_RETENTION,
};
pub use store::{
    parse_memory_references, AgentCard, AgentMetrics, AgentRecord, AuditEvent, BudgetPauseOutcome,
    BudgetStatus, ChannelRecord, CompactReport, EffectiveAgentPermissions, GateVerdict,
    GitExportOutcome, HarnessSessionMode, HarnessSessionRegistration, HarnessSessionState,
    HubError, HubStore, MemoryRecord, MemoryScope, MemoryTier, MessageKind, MessageRecord,
    MessageStatus, PendingGateApproval, ReadMarker, Role, RoleProviderDefault, SendOutcome,
    ShutdownOutcome, TaskRecord, TaskStatus, WakePolicy, WakeRecord, WakeStatus, WorkSessionRecord,
    WorkflowStep,
};
