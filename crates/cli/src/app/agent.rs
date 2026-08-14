use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum AgentCommand {
    /// List known agent identities.
    List,
    /// List agents with persisted Messager/Orchestrate team enrollment.
    Team,
    /// Enroll an existing agent on the team roster.
    Enroll {
        #[arg(long)]
        id: String,
    },
    /// Remove an agent from the team roster (still privately addressable).
    Unenroll {
        #[arg(long)]
        id: String,
    },
    /// Register an A2A Agent Card for discovery.
    RegisterCard {
        #[arg(long)]
        agent: String,
        /// Path to the agent.json card file
        #[arg(long)]
        path: PathBuf,
    },
    /// Set an agent's profile image from a local file. Any identity may
    /// set any agent's avatar, including its own.
    SetAvatar {
        agent_id: String,
        /// Path to a png/jpg/gif/webp (or any file; unknown types are stored as octet-stream).
        path: PathBuf,
    },
    /// Clear an agent's profile-image pointer. The attachment file is kept.
    ClearAvatar { agent_id: String },
}
