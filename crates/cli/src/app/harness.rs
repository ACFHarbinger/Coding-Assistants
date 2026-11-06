use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum HarnessCommand {
    /// Read a harness's on-disk session transcript and record any new
    /// assistant-authored text into the shared hub (C12), the same way the
    /// desktop's periodic refresh does — but usable headless, so a C13 live
    /// acceptance run does not require the Tauri app to be open.
    Capture {
        /// grok | claude | chat (Codex) | gemini
        #[arg(long)]
        harness: String,
        /// Absolute path to the workspace the harness session ran in.
        #[arg(long)]
        workspace: PathBuf,
        /// The harness's own on-disk session/conversation id. Locates one
        /// specific transcript; omit to use the most recently modified one.
        #[arg(long)]
        disk_session: Option<String>,
        /// The Chat & Memory work-session uuid to scope this capture into
        /// (`channel:session:<id>:capture`). Omit to post to the team feed.
        #[arg(long)]
        hub_session: Option<String>,
    },
}
