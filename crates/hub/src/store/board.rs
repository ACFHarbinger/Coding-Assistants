//! Internal Kanban cards and GitHub-issue overlays (U18 / #312).

use super::*;
use crate::github::{BoardCard, ProjectBoard};

impl HubStore {
    pub fn ensure_board_tables(&self) -> Result<(), HubError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS board_internal_tasks (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                assignees_json TEXT NOT NULL DEFAULT '[]',
                labels_json TEXT NOT NULL DEFAULT '[]',
                deadline TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS board_overlays (
                issue_number INTEGER PRIMARY KEY NOT NULL,
                assignees_json TEXT NOT NULL DEFAULT '[]',
                deadline TEXT
            );
            CREATE TABLE IF NOT EXISTS board_github_cache (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                fetched_at TEXT NOT NULL,
                payload_json TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub fn list_internal_board_cards(&self) -> Result<Vec<BoardCard>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, status, assignees_json, labels_json, deadline
             FROM board_internal_tasks ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BoardCard {
                id: row.get(0)?,
                kind: "internal".into(),
                title: row.get(1)?,
                status: row.get(2)?,
                issue_number: None,
                url: None,
                labels: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                github_assignees: Vec::new(),
                roster_assignees: serde_json::from_str(&row.get::<_, String>(3)?)
                    .unwrap_or_default(),
                deadline: row.get(5)?,
                linked_branches: Vec::new(),
                size: None,
            })
        })?;
        Ok(rows.filter_map(|row| row.ok()).collect())
    }

    pub fn upsert_internal_board_card(
        &self,
        title: &str,
        status: &str,
        assignees: &[String],
        deadline: Option<&str>,
    ) -> Result<BoardCard, HubError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(HubError::Invalid("title is required".into()));
        }
        let status = if status.trim().is_empty() {
            "Backlog"
        } else {
            status.trim()
        };
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let assignees_json = serde_json::to_string(assignees).unwrap_or_else(|_| "[]".into());
        self.conn.execute(
            "INSERT INTO board_internal_tasks
             (id, title, status, assignees_json, labels_json, deadline, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, '[]', ?5, ?6, ?6)",
            params![id, title, status, assignees_json, deadline, now],
        )?;
        Ok(BoardCard {
            id,
            kind: "internal".into(),
            title: title.to_string(),
            status: status.to_string(),
            issue_number: None,
            url: None,
            labels: Vec::new(),
            github_assignees: Vec::new(),
            roster_assignees: assignees.to_vec(),
            deadline: deadline.map(|s| s.to_string()),
            linked_branches: Vec::new(),
            size: None,
        })
    }

    pub fn set_internal_board_status(&self, id: &str, status: &str) -> Result<(), HubError> {
        let n = self.conn.execute(
            "UPDATE board_internal_tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.trim(), Utc::now().to_rfc3339(), id],
        )?;
        if n == 0 {
            return Err(HubError::NotFound(format!("internal card {id}")));
        }
        Ok(())
    }

    pub fn set_board_overlay(
        &self,
        issue_number: i64,
        assignees: Option<&[String]>,
        deadline: Option<Option<&str>>,
    ) -> Result<(), HubError> {
        let existing: Option<(String, Option<String>)> = self
            .conn
            .query_row(
                "SELECT assignees_json, deadline FROM board_overlays WHERE issue_number = ?1",
                params![issue_number],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let assignees_json = assignees
            .map(|list| serde_json::to_string(list).unwrap_or_else(|_| "[]".into()))
            .or_else(|| existing.as_ref().map(|(json, _)| json.clone()))
            .unwrap_or_else(|| "[]".into());
        let deadline = match deadline {
            Some(value) => value.map(|s| s.to_string()),
            None => existing.and_then(|(_, d)| d),
        };
        self.conn.execute(
            "INSERT INTO board_overlays(issue_number, assignees_json, deadline)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(issue_number) DO UPDATE SET
               assignees_json = excluded.assignees_json,
               deadline = excluded.deadline",
            params![issue_number, assignees_json, deadline],
        )?;
        Ok(())
    }

    pub fn apply_overlays(&self, board: &mut ProjectBoard) -> Result<(), HubError> {
        let mut stmt = self
            .conn
            .prepare("SELECT issue_number, assignees_json, deadline FROM board_overlays")?;
        let overlays: Vec<(i64, Vec<String>, Option<String>)> = stmt
            .query_map([], |row| {
                let assignees: Vec<String> =
                    serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default();
                Ok((row.get(0)?, assignees, row.get(2)?))
            })?
            .filter_map(|row| row.ok())
            .collect();
        for column in &mut board.columns {
            for card in &mut column.cards {
                if let Some(number) = card.issue_number {
                    if let Some((_, assignees, deadline)) =
                        overlays.iter().find(|(n, _, _)| *n == number)
                    {
                        card.roster_assignees = assignees.clone();
                        if deadline.is_some() {
                            card.deadline = deadline.clone();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn set_internal_board_meta(
        &self,
        id: &str,
        assignees: Option<&[String]>,
        deadline: Option<Option<&str>>,
    ) -> Result<(), HubError> {
        let existing: Option<(String, Option<String>)> = self
            .conn
            .query_row(
                "SELECT assignees_json, deadline FROM board_internal_tasks WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((assignees_json, existing_deadline)) = existing else {
            return Err(HubError::NotFound(format!("internal card {id}")));
        };
        let assignees_json = assignees
            .map(|list| serde_json::to_string(list).unwrap_or_else(|_| "[]".into()))
            .unwrap_or(assignees_json);
        let deadline = match deadline {
            Some(value) => value.map(|s| s.to_string()),
            None => existing_deadline,
        };
        self.conn.execute(
            "UPDATE board_internal_tasks
             SET assignees_json = ?1, deadline = ?2, updated_at = ?3
             WHERE id = ?4",
            params![assignees_json, deadline, Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn save_github_board_cache(&self, board: &ProjectBoard) -> Result<(), HubError> {
        let payload = serde_json::to_string(board)
            .map_err(|error| HubError::Invalid(format!("board cache encode: {error}")))?;
        self.conn.execute(
            "INSERT INTO board_github_cache(id, fetched_at, payload_json)
             VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET
               fetched_at = excluded.fetched_at,
               payload_json = excluded.payload_json",
            params![Utc::now().to_rfc3339(), payload],
        )?;
        Ok(())
    }

    pub fn load_github_board_cache(&self) -> Result<Option<(String, ProjectBoard)>, HubError> {
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT fetched_at, payload_json FROM board_github_cache WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((fetched_at, payload)) = row else {
            return Ok(None);
        };
        let board = serde_json::from_str(&payload)
            .map_err(|error| HubError::Invalid(format!("board cache decode: {error}")))?;
        Ok(Some((fetched_at, board)))
    }

    pub fn merge_internal_cards(&self, board: &mut ProjectBoard) -> Result<(), HubError> {
        for card in self.list_internal_board_cards()? {
            if let Some(col) = board.columns.iter_mut().find(|c| c.name == card.status) {
                col.cards.insert(0, card);
            } else {
                board.columns.push(crate::github::BoardColumn {
                    name: card.status.clone(),
                    cards: vec![card],
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn internal_card_round_trips_and_status_updates() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let card = store
            .upsert_internal_board_card("Spike", "Ready", &["grok".into()], Some("2026-09-20"))
            .unwrap();
        assert_eq!(card.kind, "internal");
        store
            .set_internal_board_status(&card.id, "In progress")
            .unwrap();
        let listed = store.list_internal_board_cards().unwrap();
        assert_eq!(listed[0].status, "In progress");
        assert_eq!(listed[0].deadline.as_deref(), Some("2026-09-20"));
    }

    #[test]
    fn overlay_applies_roster_assignees_to_a_github_card() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .set_board_overlay(312, Some(&["grok".into()]), Some(Some("2026-10-01")))
            .unwrap();
        let items = vec![serde_json::json!({
            "id": "PVTI_1",
            "title": "Kanban",
            "status": "Ready",
            "content": {"number": 312}
        })];
        let mut board = crate::github::board_from_items("o", 21, &items, &[], None);
        store.apply_overlays(&mut board).unwrap();
        let card = &board
            .columns
            .iter()
            .find(|c| c.name == "Ready")
            .unwrap()
            .cards[0];
        assert_eq!(card.roster_assignees, vec!["grok"]);
        assert_eq!(card.deadline.as_deref(), Some("2026-10-01"));
    }

    #[test]
    fn internal_cards_merge_into_matching_column() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .upsert_internal_board_card("Hub-only", "Ready", &["human".into()], None)
            .unwrap();
        let mut board = crate::github::empty_project_board(None);
        store.merge_internal_cards(&mut board).unwrap();
        let ready = board.columns.iter().find(|c| c.name == "Ready").unwrap();
        assert_eq!(ready.cards[0].title, "Hub-only");
        assert_eq!(ready.cards[0].kind, "internal");
    }

    #[test]
    fn github_board_cache_round_trips() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let board = crate::github::empty_project_board(None);
        store.save_github_board_cache(&board).unwrap();
        let (fetched_at, loaded) = store.load_github_board_cache().unwrap().unwrap();
        assert!(!fetched_at.is_empty());
        assert_eq!(loaded.project_number, board.project_number);
        assert_eq!(loaded.columns.len(), board.columns.len());
    }
}
