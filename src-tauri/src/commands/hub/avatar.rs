//! Agent profile-image commands. The picker in the desktop UI sends the
//! image as base64 (same contract as `hub_save_attachment`);
//! `hub_get_attachment` already resolves the stored bytes for rendering.

use super::store::open_store;
use base64::{engine::general_purpose::STANDARD, Engine};
use hub::AgentRecord;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAgentAvatarArgs {
    pub agent_id: String,
    pub filename: String,
    pub mime: String,
    pub data_base64: String,
}

fn avatar_bytes(args: &SetAgentAvatarArgs) -> Result<(String, Vec<u8>), String> {
    if !args.data_base64.is_empty() {
        let data = STANDARD
            .decode(args.data_base64.as_bytes())
            .map_err(|e| format!("invalid base64 avatar data: {e}"))?;
        return Ok((args.filename.clone(), data));
    }
    // Desktop file-dialog picks return a filesystem path. When the
    // frontend could not base64-encode the bytes (asset protocol
    // unavailable), `filename` is that path and we read it here.
    let path = std::path::Path::new(&args.filename);
    if !path.is_file() {
        return Err("avatar data is empty (provide dataBase64 or an existing file path)".into());
    }
    let data =
        std::fs::read(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("avatar")
        .to_string();
    Ok((filename, data))
}

#[tauri::command]
pub fn hub_set_agent_avatar(args: SetAgentAvatarArgs) -> Result<AgentRecord, String> {
    let (filename, data) = avatar_bytes(&args)?;
    open_store()?
        .set_agent_avatar(&args.agent_id, &filename, &args.mime, &data)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_clear_agent_avatar(agent_id: String) -> Result<AgentRecord, String> {
    open_store()?
        .clear_agent_avatar(&agent_id)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::commands::attachments::hub_get_attachment;
    use crate::commands::commands::tests::CA_HOME_ENV_LOCK;

    fn with_ca_home<T>(prefix: &str, run: impl FnOnce() -> T) -> T {
        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "tauri-avatar-{prefix}-{}-{}",
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
    fn set_and_clear_avatar_round_trips_through_base64() {
        with_ca_home("roundtrip", || {
            let updated = hub_set_agent_avatar(SetAgentAvatarArgs {
                agent_id: "claude".into(),
                filename: "avatar.png".into(),
                mime: "image/png".into(),
                data_base64: STANDARD.encode(b"pretend-png-bytes"),
            })
            .expect("set");
            let attachment_id = updated
                .avatar_attachment_id
                .clone()
                .expect("avatar_attachment_id must be set");

            let fetched = hub_get_attachment(attachment_id)
                .expect("fetch")
                .expect("present");
            assert_eq!(
                STANDARD.decode(fetched.data_base64).unwrap(),
                b"pretend-png-bytes"
            );

            let cleared = hub_clear_agent_avatar("claude".into()).expect("clear");
            assert_eq!(cleared.avatar_attachment_id, None);
        });
    }

    #[test]
    fn invalid_base64_is_rejected_before_touching_the_store() {
        with_ca_home("bad-b64", || {
            let result = hub_set_agent_avatar(SetAgentAvatarArgs {
                agent_id: "claude".into(),
                filename: "x.png".into(),
                mime: "image/png".into(),
                data_base64: "not-valid-base64!!".into(),
            });
            assert!(result.is_err());
        });
    }

    #[test]
    fn empty_base64_reads_filename_as_a_filesystem_path() {
        with_ca_home("from-path", || {
            let dir = std::env::var("CA_HOME").unwrap();
            let image = std::path::Path::new(&dir).join("picked.png");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&image, b"from-disk").unwrap();

            let updated = hub_set_agent_avatar(SetAgentAvatarArgs {
                agent_id: "grok".into(),
                filename: image.to_string_lossy().into(),
                mime: "image/png".into(),
                data_base64: String::new(),
            })
            .expect("set from path");
            let attachment_id = updated.avatar_attachment_id.expect("pointer set");
            let fetched = hub_get_attachment(attachment_id)
                .expect("fetch")
                .expect("present");
            assert_eq!(STANDARD.decode(fetched.data_base64).unwrap(), b"from-disk");
        });
    }

    #[test]
    fn unknown_agent_is_not_found() {
        with_ca_home("missing", || {
            let result = hub_set_agent_avatar(SetAgentAvatarArgs {
                agent_id: "does-not-exist".into(),
                filename: "x.png".into(),
                mime: "image/png".into(),
                data_base64: STANDARD.encode(b"x"),
            });
            assert!(result.is_err());
            assert!(hub_clear_agent_avatar("does-not-exist".into()).is_err());
        });
    }
}
