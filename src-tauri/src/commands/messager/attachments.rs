//! Message attachment upload/download commands. Pastes/file picks in the
//! Chat & Memory composer go through `hub_save_attachment` (base64 over the
//! IPC boundary, decoded and written to disk), which returns the record the
//! frontend embeds into the message body as an `attachment://<id>` marker;
//! `hub_get_attachment` resolves that marker back into bytes for inline
//! rendering.

use super::store::open_store;
use base64::{engine::general_purpose::STANDARD, Engine};
use hub::AttachmentRecord;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAttachmentArgs {
    pub filename: String,
    pub mime: String,
    pub data_base64: String,
}

#[tauri::command]
pub async fn hub_save_attachment(args: SaveAttachmentArgs) -> Result<AttachmentRecord, String> {
    tauri::async_runtime::spawn_blocking(move || hub_save_attachment_blocking(args))
        .await
        .map_err(|e| format!("hub_save_attachment worker panic: {e}"))?
}

pub fn hub_save_attachment_blocking(args: SaveAttachmentArgs) -> Result<AttachmentRecord, String> {
    let data = STANDARD
        .decode(args.data_base64.as_bytes())
        .map_err(|e| format!("invalid base64 attachment data: {e}"))?;
    open_store()?
        .save_attachment(&args.filename, &args.mime, &data)
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct AttachmentPayload {
    pub record: AttachmentRecord,
    pub data_base64: String,
}

#[tauri::command]
pub async fn hub_get_attachment(id: String) -> Result<Option<AttachmentPayload>, String> {
    tauri::async_runtime::spawn_blocking(move || hub_get_attachment_blocking(id))
        .await
        .map_err(|e| format!("hub_get_attachment worker panic: {e}"))?
}

pub fn hub_get_attachment_blocking(id: String) -> Result<Option<AttachmentPayload>, String> {
    let found = open_store()?
        .read_attachment(&id)
        .map_err(|e| e.to_string())?;
    Ok(found.map(|(record, data)| AttachmentPayload {
        record,
        data_base64: STANDARD.encode(data),
    }))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAttachmentToPathArgs {
    pub id: String,
    pub target_path: String,
}

#[tauri::command]
pub async fn hub_save_attachment_to_path(args: SaveAttachmentToPathArgs) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || hub_save_attachment_to_path_blocking(args))
        .await
        .map_err(|e| format!("hub_save_attachment_to_path worker panic: {e}"))?
}

pub fn hub_save_attachment_to_path_blocking(args: SaveAttachmentToPathArgs) -> Result<(), String> {
    let found = open_store()?
        .read_attachment(&args.id)
        .map_err(|e| e.to_string())?;
    match found {
        Some((_record, data)) => std::fs::write(&args.target_path, data)
            .map_err(|e| format!("Failed to write attachment to {}: {e}", args.target_path)),
        None => Err(format!("Attachment not found: {}", args.id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::commands::tests::CA_HOME_ENV_LOCK;

    fn with_ca_home<T>(prefix: &str, run: impl FnOnce() -> T) -> T {
        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "tauri-attachments-{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::env::set_var("CA_HOME", &dir);
        let result = run();
        std::env::remove_var("CA_HOME");
        let _ = std::fs::remove_dir_all(&dir);
        result
    }

    #[test]
    fn save_and_fetch_round_trips_through_base64() {
        with_ca_home("roundtrip", || {
            let saved = hub_save_attachment_blocking(SaveAttachmentArgs {
                filename: "screenshot.png".into(),
                mime: "image/png".into(),
                data_base64: STANDARD.encode(b"pretend-png-bytes"),
            })
            .expect("save");
            assert_eq!(saved.filename, "screenshot.png");
            assert_eq!(saved.mime, "image/png");

            let fetched = hub_get_attachment_blocking(saved.id.clone())
                .expect("fetch")
                .expect("present");
            assert_eq!(fetched.record.id, saved.id);
            assert_eq!(
                STANDARD.decode(fetched.data_base64).unwrap(),
                b"pretend-png-bytes"
            );
        });
    }

    #[test]
    fn invalid_base64_is_rejected_before_touching_the_store() {
        with_ca_home("bad-b64", || {
            let result = hub_save_attachment_blocking(SaveAttachmentArgs {
                filename: "x.png".into(),
                mime: "image/png".into(),
                data_base64: "not-valid-base64!!".into(),
            });
            assert!(result.is_err());
        });
    }

    #[test]
    fn unknown_id_fetches_as_none() {
        with_ca_home("missing", || {
            assert!(hub_get_attachment_blocking("does-not-exist".into())
                .unwrap()
                .is_none());
        });
    }

    #[test]
    fn save_attachment_to_path_writes_file_to_disk() {
        with_ca_home("save-to-path", || {
            let saved = hub_save_attachment_blocking(SaveAttachmentArgs {
                filename: "export.txt".into(),
                mime: "text/plain".into(),
                data_base64: STANDARD.encode(b"hello attachment"),
            })
            .expect("save");

            let target = std::env::temp_dir().join(format!(
                "test-export-{}.txt",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));

            hub_save_attachment_to_path_blocking(SaveAttachmentToPathArgs {
                id: saved.id.clone(),
                target_path: target.to_string_lossy().to_string(),
            })
            .expect("save to path");

            assert_eq!(
                std::fs::read_to_string(&target).unwrap(),
                "hello attachment"
            );
            let _ = std::fs::remove_file(target);
        });
    }
}
