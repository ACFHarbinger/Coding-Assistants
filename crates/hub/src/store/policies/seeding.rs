use super::super::*;

impl HubStore {
    pub(super) fn seed_identities_and_roster(&self) -> Result<(), HubError> {
        // Well-known agent identities. This list only grows: each release that
        // onboards a harness adds a row here, so a Hub created before that
        // release would never learn the new identity's display name if this
        // were a fresh-install-only seed (it was, until `mistral` was added —
        // an existing Hub showed a bare `mistral` id with no name).
        //
        // Re-running it is safe: `upsert_agent` writes only `id` and
        // `display_name` and never touches `team_member`, so this cannot
        // enrol anyone. Team membership stays the explicit user action
        // described below.
        const WELL_KNOWN_AGENTS: &[(&str, &str)] = &[
            ("human", "Human"),
            ("claude", "Claude Code"),
            ("chat", "Codex / Chat"),
            ("gemini", "Gemini / Antigravity"),
            ("grok", "Grok Build"),
            ("muse", "Muse Code"),
            ("cursor", "Cursor Agent"),
            ("mistral", "Mistral Vibe"),
            ("qwen", "Qwen Code"),
            ("opencode", "OpenCode"),
            ("kimi", "Kimi"),
            ("ollama", "Ollama"),
            ("llamacpp", "llama.cpp"),
            ("system", "System"),
        ];
        // Bump when `WELL_KNOWN_AGENTS` gains an entry, so existing Hubs pick
        // it up exactly once instead of on every open.
        const AGENT_IDENTITIES_VERSION: &str = "4";
        let identities_seeded: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'agent_identities_seeded'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if identities_seeded.as_deref() != Some(AGENT_IDENTITIES_VERSION) {
            for (id, name) in WELL_KNOWN_AGENTS {
                self.upsert_agent(id, name)?;
            }
            self.conn.execute(
                "INSERT OR REPLACE INTO meta(key, value)
                 VALUES ('agent_identities_seeded', ?1)",
                params![AGENT_IDENTITIES_VERSION],
            )?;
        }

        // Team membership is an explicit user action. A fresh Hub starts with
        // only its human owner; agents become session members through the
        // Orchestrate "Add to team" control or an explicit wake.
        let roster_seeded: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'team_roster_seeded'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if roster_seeded.is_none() {
            self.conn.execute(
                "UPDATE agents SET team_member = CASE WHEN id = 'human' THEN 1 ELSE 0 END",
                [],
            )?;
            self.conn.execute(
                "INSERT OR REPLACE INTO meta(key, value) VALUES ('team_roster_seeded', '2')",
                [],
            )?;
        } else if roster_seeded.as_deref() == Some("1") {
            // Version 1 silently seeded every primary agent. Migrate only the
            // exact untouched legacy default, preserving any roster that a
            // user has actually changed.
            let legacy_default_count: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM agents
                 WHERE team_member = 1 AND id IN ('human', 'claude', 'chat', 'gemini', 'grok')",
                [],
                |row| row.get(0),
            )?;
            let custom_member_count: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM agents
                 WHERE team_member = 1 AND id NOT IN ('human', 'claude', 'chat', 'gemini', 'grok')",
                [],
                |row| row.get(0),
            )?;
            if legacy_default_count == 5 && custom_member_count == 0 {
                self.conn.execute(
                    "UPDATE agents SET team_member = CASE WHEN id = 'human' THEN 1 ELSE 0 END",
                    [],
                )?;
            }
            self.conn.execute(
                "INSERT OR REPLACE INTO meta(key, value) VALUES ('team_roster_seeded', '2')",
                [],
            )?;
        }

        Ok(())
    }
}
