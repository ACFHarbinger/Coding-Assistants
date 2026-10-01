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

fn item(name: &str, etag: &str) -> String {
    format!(r#"{{"name":"{name}","eTag":"\"{etag}\"","size":3}}"#)
}

fn list_body(name: &str, etag: &str) -> String {
    format!(r#"{{"value":[{}]}}"#, item(name, etag))
}

#[test]
fn list_keeps_hashed_names_only() {
    let hashed = blob(0xaa);
    let body = format!(
        r#"{{"value":[{{"name":"{}","eTag":"1","size":4}},{{"name":"claude.md","eTag":"2","size":9}}]}}"#,
        hashed.as_str()
    );
    let files = parse_object_list(body.as_bytes(), "replica/").unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].object.blob_id, hashed);
    assert!(files[0].name.starts_with("replica/"));
}

#[test]
fn private_app_folder_uses_hashed_names_and_no_secrets() {
    let id = blob(0xaa);
    let created = blob(0xbb);
    let transport = ScriptedTransport::new(vec![
        HttpResponse {
            status: 200,
            etag: None,
            body: list_body(id.as_str(), "v1").into_bytes(),
        },
        HttpResponse {
            status: 404,
            etag: None,
            body: Vec::new(),
        },
        HttpResponse {
            status: 201,
            etag: None,
            body: item(created.as_str(), "v2").into_bytes(),
        },
    ]);
    let mut storage = OneDriveStorage::new(transport);
    assert_eq!(storage.list(&replica_prefix()).unwrap()[0].blob_id, id);
    storage
        .put_if_unmodified(&replica_prefix(), &created, &[0xff], None)
        .unwrap();
    let recorded = storage.transport.recorded.lock().unwrap().clone();
    assert!(recorded.iter().any(|line| {
        line.contains("graph.microsoft.com")
            && line.contains("special/approot")
            && line.contains("replica")
            && line.contains(created.as_str())
            && line.contains("PUT")
            && !line.contains("%2F")
    }));
    assert!(recorded.iter().all(|line| !line.contains("Bearer")
        && !line.contains("access_token")
        && !line.contains(&forbidden_key_name())));
}

#[test]
fn stale_etag_does_not_upload() {
    let id = blob(0xcc);
    let transport = ScriptedTransport::new(vec![HttpResponse {
        status: 200,
        etag: None,
        body: item(id.as_str(), "old").into_bytes(),
    }]);
    let mut storage = OneDriveStorage::new(transport);
    assert_eq!(
        storage
            .put_if_unmodified(&replica_prefix(), &id, &[1], Some(&ETag::new("stale")))
            .unwrap_err(),
        SyncError::Precondition
    );
    assert_eq!(storage.transport.recorded.lock().unwrap().len(), 1);
}

#[test]
fn storage_http_412_is_precondition_and_secret_free() {
    let transport = ScriptedTransport::new(vec![
        HttpResponse {
            status: 404,
            etag: None,
            body: Vec::new(),
        },
        HttpResponse {
            status: 412,
            etag: None,
            body: b"{\"error\":\"secret-token\"}".to_vec(),
        },
    ]);
    let mut storage = OneDriveStorage::new(transport);
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
            status: 404,
            etag: None,
            body: Vec::new(),
        },
        HttpResponse {
            status: 409,
            etag: None,
            body: Vec::new(),
        },
    ]);
    let mut storage = OneDriveStorage::new(transport);
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
        .all(|line| !line.starts_with("DELETE")));
}

#[test]
fn debug_is_redacted() {
    assert_eq!(
        format!(
            "{:?}",
            OneDriveStorage::new(ScriptedTransport::new(Vec::new()))
        ),
        "OneDriveStorage(***)"
    );
}

#[test]
fn adapter_source_never_uploads_replica_key() {
    let src = concat!(
        include_str!("onedrive.rs"),
        include_str!("onedrive_auth.rs"),
        include_str!("onedrive_tests.rs")
    );
    let key_name = forbidden_key_name();
    assert!(!src.contains(&key_name));
    assert!(!include_str!("onedrive.rs").contains("KeyEnvelope"));
    let note = include_str!("../../../../docs/moon/onedrive-app-folder.md");
    assert!(note.contains("approot"));
    assert!(note.contains("[0-9a-f]{64}"));
    assert!(!note.contains(&key_name));
}
