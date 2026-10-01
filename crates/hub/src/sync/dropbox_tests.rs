use super::super::google_http::{HttpResponse, ScriptedTransport};
use super::*;
use crate::sync::client::{ReplicaAdvance, ReplicaPut};
use crate::sync::layout::{device_prefix, replica_prefix};
use crate::sync::types::DeviceId;

fn blob(seed: u8) -> BlobId {
    BlobId::parse(&format!("{seed:064x}")).unwrap()
}

fn forbidden_key_name() -> String {
    format!("{}{}{}", "cloud-sync", ".", "key")
}

fn file_entry(path: &str, rev: &str) -> String {
    let name = path.rsplit('/').next().unwrap();
    format!(r#"{{".tag":"file","name":"{name}","path_display":"{path}","rev":"{rev}","size":3}}"#)
}

fn list_body(path: &str, rev: &str) -> String {
    format!(r#"{{"entries":[{}]}}"#, file_entry(path, rev))
}

#[test]
fn list_keeps_hashed_names_only() {
    let hashed = blob(0xaa);
    let body = format!(
        r#"{{"entries":[{},{{".tag":"file","name":"claude.md","path_display":"/journals/claude.md","rev":"2","size":9}}]}}"#,
        file_entry(&format!("/replica/{}", hashed.as_str()), "1")
    );
    let files = parse_entry_list(body.as_bytes()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].object.blob_id, hashed);
}

#[test]
fn private_app_folder_uses_hashed_names_and_no_secrets() {
    let id = blob(0xaa);
    let created = blob(0xbb);
    let listed = format!("/replica/{}", id.as_str());
    let created_path = format!("/replica/{}", created.as_str());
    let transport = ScriptedTransport::new(vec![
        HttpResponse {
            status: 200,
            etag: None,
            body: list_body(&listed, "v1").into_bytes(),
        },
        HttpResponse {
            status: 409,
            etag: None,
            body: Vec::new(),
        },
        HttpResponse {
            status: 200,
            etag: None,
            body: file_entry(&created_path, "v2").into_bytes(),
        },
    ]);
    let mut storage = DropboxStorage::new(transport);
    assert_eq!(storage.list(&replica_prefix()).unwrap()[0].blob_id, id);
    storage
        .put_if_unmodified(&replica_prefix(), &created, &[0xff], None)
        .unwrap();
    let recorded = storage.transport.recorded.lock().unwrap().clone();
    assert!(recorded.iter().any(|line| {
        line.contains("content.dropboxapi.com")
            && line.contains("files/upload")
            && line.contains(created.as_str())
            && line.contains("Dropbox-API-Arg")
    }));
    assert!(recorded.iter().all(|line| !line.contains("Bearer")
        && !line.contains("access_token")
        && !line.contains(&forbidden_key_name())));
}

#[test]
fn stale_rev_does_not_upload() {
    let id = blob(0xcc);
    let path = format!("/replica/{}", id.as_str());
    let transport = ScriptedTransport::new(vec![HttpResponse {
        status: 200,
        etag: None,
        body: file_entry(&path, "old").into_bytes(),
    }]);
    let mut storage = DropboxStorage::new(transport);
    assert_eq!(
        storage
            .put_if_unmodified(&replica_prefix(), &id, &[1], Some(&ETag::new("stale")))
            .unwrap_err(),
        SyncError::Precondition
    );
    assert_eq!(storage.transport.recorded.lock().unwrap().len(), 1);
}

#[test]
fn storage_http_409_is_precondition_and_secret_free() {
    let transport = ScriptedTransport::new(vec![
        HttpResponse {
            status: 409,
            etag: None,
            body: Vec::new(),
        },
        HttpResponse {
            status: 409,
            etag: None,
            body: b"{\"error_summary\":\"secret-token\"}".to_vec(),
        },
    ]);
    let mut storage = DropboxStorage::new(transport);
    let err = storage
        .put_if_unmodified(&replica_prefix(), &blob(0xdd), &[0xff], None)
        .unwrap_err();
    assert_eq!(err, SyncError::Precondition);
    assert!(!format!("{err}").contains("secret-token"));
    assert!(!format!("{err}").contains("Bearer"));
}

#[test]
fn failed_put_does_not_prune_device_prefix() {
    let replica = blob(0x11);
    let prefix = device_prefix(&DeviceId::generate()).unwrap();
    let transport = ScriptedTransport::new(vec![
        HttpResponse {
            status: 409,
            etag: None,
            body: Vec::new(),
        },
        HttpResponse {
            status: 409,
            etag: None,
            body: Vec::new(),
        },
    ]);
    let mut storage = DropboxStorage::new(transport);
    let result = storage.advance_replica(ReplicaAdvance {
        puts: vec![ReplicaPut {
            prefix: replica_prefix(),
            blob_id: replica,
            bytes: vec![1],
            expected_etag: None,
        }],
        prune_prefix: Some(prefix),
    });
    assert_eq!(result.unwrap_err(), SyncError::Precondition);
    assert!(storage
        .transport
        .recorded
        .lock()
        .unwrap()
        .iter()
        .all(|line| !line.contains("delete_v2")));
}

#[test]
fn debug_is_redacted() {
    assert_eq!(
        format!(
            "{:?}",
            DropboxStorage::new(ScriptedTransport::new(Vec::new()))
        ),
        "DropboxStorage(***)"
    );
}

#[test]
fn adapter_source_never_uploads_replica_key() {
    let src = concat!(
        include_str!("dropbox.rs"),
        include_str!("dropbox_auth.rs"),
        include_str!("dropbox_tests.rs")
    );
    let key_name = forbidden_key_name();
    assert!(!src.contains(&key_name));
    assert!(!include_str!("dropbox.rs").contains("KeyEnvelope"));
    let note = include_str!("../../../../docs/moon/dropbox-app-folder.md");
    assert!(note.contains("app folder"));
    assert!(note.contains("[0-9a-f]{64}"));
    assert!(!note.contains(&key_name));
}
