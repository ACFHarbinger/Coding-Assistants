//! Message attachments (images and other files pasted or picked in the
//! desktop composer). Stored as plain files under `<hub_home>/attachments/`
//! and indexed in the `attachments` table; a message references one by
//! embedding an `attachment://<id>` marker in its body text, the same
//! pattern already used for `[Memory #<id>]` links — no change to the
//! `messages` table or any existing `send_message`/`send_tagged_message`
//! call site.

use super::*;
use std::fs;

/// Desktop uploads above this size are rejected outright rather than
/// silently truncated or slowly written.
const MAX_ATTACHMENT_BYTES: usize = 20 * 1024 * 1024;

fn sanitize_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "attachment".to_string()
    } else {
        trimmed.to_string()
    }
}

fn row_to_attachment(row: &rusqlite::Row, base_dir: &Path) -> rusqlite::Result<AttachmentRecord> {
    let relative_path: String = row.get(4)?;
    Ok(AttachmentRecord {
        id: row.get(0)?,
        filename: row.get(1)?,
        mime: row.get(2)?,
        byte_size: row.get(3)?,
        absolute_path: base_dir.join(&relative_path).to_string_lossy().to_string(),
        created_at: row.get(5)?,
    })
}

const ATTACHMENT_COLUMNS: &str = "id, filename, mime, byte_size, relative_path, created_at";

impl HubStore {
    pub fn attachments_dir(&self) -> PathBuf {
        self.data_dir.join("attachments")
    }

    /// Writes `data` to disk under `attachments_dir()` and indexes it.
    /// Returns the durable record (including the absolute on-disk path)
    /// the caller embeds into a message body as `attachment://<id>`.
    pub fn save_attachment(
        &self,
        filename: &str,
        mime: &str,
        data: &[u8],
    ) -> Result<AttachmentRecord, HubError> {
        if data.is_empty() {
            return Err(HubError::Invalid(
                "attachment data must not be empty".into(),
            ));
        }
        if data.len() > MAX_ATTACHMENT_BYTES {
            return Err(HubError::Invalid(format!(
                "attachment exceeds the {} MiB limit",
                MAX_ATTACHMENT_BYTES / (1024 * 1024)
            )));
        }
        let mime = if mime.trim().is_empty() {
            "application/octet-stream"
        } else {
            mime.trim()
        };
        let id = Uuid::new_v4().to_string();
        let safe_name = sanitize_filename(filename);
        let dir = self.attachments_dir();
        fs::create_dir_all(&dir)?;
        let relative_path = format!("{id}__{safe_name}");
        fs::write(dir.join(&relative_path), data)?;
        let created_at = Utc::now().to_rfc3339();
        self.conn.execute(
            &format!(
                "INSERT INTO attachments ({ATTACHMENT_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            ),
            params![
                id,
                safe_name,
                mime,
                data.len() as i64,
                relative_path,
                created_at
            ],
        )?;
        self.get_attachment_record(&id)?
            .ok_or_else(|| HubError::NotFound(id))
    }

    pub fn get_attachment_record(&self, id: &str) -> Result<Option<AttachmentRecord>, HubError> {
        let dir = self.attachments_dir();
        self.conn
            .query_row(
                &format!("SELECT {ATTACHMENT_COLUMNS} FROM attachments WHERE id = ?1"),
                params![id],
                |row| row_to_attachment(row, &dir),
            )
            .optional()
            .map_err(HubError::from)
    }

    /// Reads the attachment's metadata and raw bytes back from disk,
    /// for the composer/message stream to render or hand to a harness.
    pub fn read_attachment(
        &self,
        id: &str,
    ) -> Result<Option<(AttachmentRecord, Vec<u8>)>, HubError> {
        let Some(record) = self.get_attachment_record(id)? else {
            return Ok(None);
        };
        let data = fs::read(&record.absolute_path)?;
        Ok(Some((record, data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_and_read_attachment_round_trips_bytes_and_metadata() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let record = store
            .save_attachment("../weird/../name.png", "image/png", b"fake-bytes")
            .unwrap();
        assert_eq!(record.filename, "name.png");
        assert_eq!(record.byte_size, 10);
        assert!(Path::new(&record.absolute_path).is_file());

        let (fetched, data) = store.read_attachment(&record.id).unwrap().unwrap();
        assert_eq!(fetched.id, record.id);
        assert_eq!(data, b"fake-bytes");
    }

    #[test]
    fn empty_or_oversized_attachments_are_rejected() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(store.save_attachment("a.txt", "text/plain", b"").is_err());
        let oversized = vec![0u8; MAX_ATTACHMENT_BYTES + 1];
        assert!(store
            .save_attachment("a.bin", "application/octet-stream", &oversized)
            .is_err());
    }

    #[test]
    fn unknown_attachment_id_reads_as_none() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(store.read_attachment("missing").unwrap().is_none());
    }
}
