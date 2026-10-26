//! Goal delivery text and tagged-send counters (#380).
//!
//! A Goal keeps the old task contract (enrolled member, no spawn). The body
//! the agent receives starts with `/goal` and carries a `[GOAL]` marker.
//! Stored `[TASK]` rows are rewritten by `migrations/rename-task-marker-to-goal.sh`;
//! this path does not recognize the old marker.

use super::super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaggedSendCounts {
    /// Accepted wakes and goals. A send that is both counts once.
    pub tasks: i64,
    /// Accepted wake-tagged sends.
    pub wakes: i64,
    /// Accepted goal sends, stored on the task flag.
    pub goals: i64,
}

pub(crate) fn goal_delivery_body(body: &str) -> String {
    let mut text = body.trim();
    let mut wake = false;
    if let Some(rest) = text.strip_prefix("[WAKE]") {
        wake = true;
        text = rest.trim_start();
    }
    if let Some(rest) = strip_goal_command(text) {
        text = rest.trim_start();
    }
    if let Some(rest) = text.strip_prefix("[GOAL]") {
        text = rest.trim_start();
    }
    if let Some(rest) = text.strip_prefix("[WAKE]") {
        wake = true;
        text = rest.trim_start();
    }
    let words = text.trim();
    let marker = if wake {
        "/goal [GOAL] [WAKE]"
    } else {
        "/goal [GOAL]"
    };
    if words.is_empty() {
        marker.to_string()
    } else {
        format!("{marker} {words}")
    }
}

fn strip_goal_command(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("/goal")?;
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        Some(rest.trim_start())
    } else {
        None
    }
}

impl HubStore {
    /// Accepted tagged sends. Refusals stay out of every figure.
    pub fn tagged_send_counts(&self) -> Result<TaggedSendCounts, HubError> {
        let row = self.conn.query_row(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN accepted != 0 AND (is_task != 0 OR is_wake != 0) THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN accepted != 0 AND is_wake != 0 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN accepted != 0 AND is_task != 0 THEN 1 ELSE 0 END), 0)
            FROM tagged_send_outcomes
            "#,
            [],
            |row| {
                Ok(TaggedSendCounts {
                    tasks: row.get(0)?,
                    wakes: row.get(1)?,
                    goals: row.get(2)?,
                })
            },
        )?;
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::goal_delivery_body;

    #[test]
    fn goal_body_prefixes_once_and_keeps_a_wake_marker() {
        assert_eq!(
            goal_delivery_body("ship the release"),
            "/goal [GOAL] ship the release"
        );
        assert_eq!(
            goal_delivery_body("/goal [GOAL] ship the release"),
            "/goal [GOAL] ship the release"
        );
        assert_eq!(
            goal_delivery_body("[WAKE] ship the release"),
            "/goal [GOAL] [WAKE] ship the release"
        );
    }
}
