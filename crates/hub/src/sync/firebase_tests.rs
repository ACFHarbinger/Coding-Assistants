use super::super::google_http::{HttpResponse, ScriptedTransport};
use super::*;
use crate::sync::client::{ReplicaAdvance, ReplicaPut};
use crate::sync::layout::{device_prefix, replica_prefix};
use crate::sync::types::DeviceId;

const BUCKET: &str = "ca-private.appspot.com";

fn hex_id(seed: u8) -> BlobId {
    BlobId::parse(&format!("{seed:064x}")).unwrap()
}

fn forbidden_key_name() -> String {
    format!("{}{}{}", "cloud-sync", ".", "key")
}

#[test]
fn parse_object_list_keeps_hashed_names_only() {
    let body = br#"{
        "items": [
            {"name": "replica/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "generation": "1", "size": "4"},
            {"name": "journals/claude.md", "generation": "2", "size": "9"}
        ]
    }"#;
    let files = parse_object_list(body).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].object.blob_id.as_str().len(), 64);
    assert!(!files[0].object.blob_id.as_str().contains('/'));
}

#[test]
fn debug_and_errors_omit_tokens() {
    let drive = FirebaseStorage::new(ScriptedTransport::new(Vec::new()), BUCKET).unwrap();
    assert_eq!(format!("{drive:?}"), "FirebaseStorage(***)");
    let err = SyncError::Invalid("firebase storage request failed".into());
    assert!(!format!("{err}").contains("Bearer"));
    assert!(!format!("{err}").contains("ya29"));
    assert!(!format!("{err}").contains("idToken"));
    assert!(!format!("{err}").contains("AIza"));
    assert!(FirebaseStorage::new(ScriptedTransport::new(Vec::new()), "").is_err());
    assert!(FirebaseStorage::new(ScriptedTransport::new(Vec::new()), "..").is_err());
    assert!(FirebaseStorage::new(ScriptedTransport::new(Vec::new()), "a/b").is_err());
}

#[test]
fn list_and_put_use_private_bucket_prefix_and_hashed_names() {
    let blob = hex_id(0xaa);
    let folder = replica_prefix();
    let list_body = format!(
        r#"{{"items":[{{"name":"replica/{}","generation":"1","size":"3"}}]}}"#,
        blob.as_str()
    );
    let created = hex_id(0xbb);
    let create_body = format!(
        r#"{{"name":"replica/{}","generation":"2","size":"3"}}"#,
        created.as_str()
    );
    let transport = ScriptedTransport::new(vec![
        HttpResponse {
            status: 200,
            etag: None,
            body: list_body.into_bytes(),
        },
        HttpResponse {
            status: 404,
            etag: None,
            body: Vec::new(),
        },
        HttpResponse {
            status: 200,
            etag: None,
            body: create_body.into_bytes(),
        },
    ]);
    let mut drive = FirebaseStorage::new(transport, BUCKET).unwrap();
    let listed = drive.list(&folder).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].blob_id, blob);

    let bytes: &[u8] = &[0xff, 0xfe, 0x01];
    drive
        .put_if_unmodified(&folder, &created, bytes, None)
        .unwrap();
    let recorded = drive.transport.recorded.lock().unwrap().clone();
    assert!(recorded
        .iter()
        .any(|line| line.contains("firebasestorage.googleapis.com") && line.contains(BUCKET)));
    assert!(recorded
        .iter()
        .any(|line| line.contains("replica") && line.contains(created.as_str())));
    assert!(recorded
        .iter()
        .any(|line| line.contains(created.as_str()) && !line.contains("enc")));
    assert!(recorded.iter().all(|line| !line.contains("Bearer")));
    assert!(recorded.iter().all(|line| !line.contains("ya29")));
    assert!(recorded.iter().all(|line| !line.contains("AIza")));
    assert!(recorded.iter().all(|line| !line.contains("enc")));
}

#[test]
fn stale_generation_does_not_replace() {
    let blob = hex_id(0xcc);
    let folder = replica_prefix();
    let lookup_body = format!(
        r#"{{"name":"replica/{}","generation":"old","size":"3"}}"#,
        blob.as_str()
    );
    let transport = ScriptedTransport::new(vec![HttpResponse {
        status: 200,
        etag: None,
        body: lookup_body.into_bytes(),
    }]);
    let mut drive = FirebaseStorage::new(transport, BUCKET).unwrap();
    let err = drive
        .put_if_unmodified(&folder, &blob, &[0xff], Some(&ETag::new("nope")))
        .unwrap_err();
    assert_eq!(err, SyncError::Precondition);
}

#[test]
fn storage_http_412_is_precondition() {
    let blob = hex_id(0xdd);
    let folder = replica_prefix();
    let transport = ScriptedTransport::new(vec![HttpResponse {
        status: 412,
        etag: None,
        body: Vec::new(),
    }]);
    let mut drive = FirebaseStorage::new(transport, BUCKET).unwrap();
    let err = drive
        .put_if_unmodified(&folder, &blob, &[0xff], None)
        .unwrap_err();
    assert_eq!(err, SyncError::Precondition);
    assert!(!format!("{err}").contains("Bearer"));
}

#[test]
fn failed_replica_put_leaves_device_prefix_objects() {
    let device = DeviceId::generate();
    let device_folder = device_prefix(&device).unwrap();
    let replica = replica_prefix();
    let replica_blob = hex_id(0x11);
    let device_blob = hex_id(0x22);
    let device_list = format!(
        r#"{{"items":[{{"name":"{}{}","generation":"9","size":"1"}}]}}"#,
        device_folder.as_str(),
        device_blob.as_str()
    );
    let transport = ScriptedTransport::new(vec![
        HttpResponse {
            status: 404,
            etag: None,
            body: Vec::new(),
        },
        HttpResponse {
            status: 412,
            etag: None,
            body: Vec::new(),
        },
        HttpResponse {
            status: 200,
            etag: None,
            body: device_list.into_bytes(),
        },
    ]);
    let mut drive = FirebaseStorage::new(transport, BUCKET).unwrap();
    let err = drive
        .advance_replica(ReplicaAdvance {
            puts: vec![ReplicaPut {
                prefix: replica,
                blob_id: replica_blob,
                bytes: vec![0xff, 0xfe],
                expected_etag: None,
            }],
            prune_prefix: Some(device_folder.clone()),
        })
        .unwrap_err();
    assert_eq!(err, SyncError::Precondition);
    let recorded = drive.transport.recorded.lock().unwrap().clone();
    assert!(recorded.iter().all(|line| !line.starts_with("DELETE")));
    let listed = drive.list(&device_folder).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].blob_id, device_blob);
}

#[test]
fn adapter_source_never_uploads_replica_key() {
    let src = concat!(
        include_str!("firebase.rs"),
        include_str!("firebase_tests.rs")
    );
    let wrap = format!("{}{}", "wrap", "ping");
    assert!(!src.contains(&forbidden_key_name()));
    assert!(!include_str!("firebase.rs").contains(&wrap));
    assert!(!include_str!("firebase.rs").contains("KeyEnvelope"));
    let rules = include_str!("../../../../docs/moon/firebase-storage.rules");
    assert!(rules.contains("request.auth != null"));
    assert!(rules.contains("[0-9a-f]{64}"));
    assert!(!rules.contains(&forbidden_key_name()));
    assert!(!rules.contains("allow read, write: if true"));
}
