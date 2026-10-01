use super::super::google_http::{HttpResponse, ScriptedTransport};
use super::*;
use crate::sync::layout::{device_prefix, replica_prefix};
use crate::sync::types::DeviceId;

const ORIGIN: &str = "https://example.supabase.co";
const BUCKET: &str = "ca-sync";

fn blob(seed: u8) -> BlobId {
    BlobId::parse(&format!("{seed:064x}")).unwrap()
}
fn item(name: &str, etag: &str) -> String {
    format!(r#"[{{"name":"{name}","metadata":{{"eTag":"{etag}","size":3}}}}]"#)
}

#[test]
fn private_bucket_uses_hashed_names_and_no_secrets() {
    let id = blob(0xaa);
    let name = format!("replica/{}", id.as_str());
    let created = blob(0xbb);
    let transport = ScriptedTransport::new(vec![
        HttpResponse {
            status: 200,
            etag: None,
            body: item(&name, "v1").into_bytes(),
        },
        HttpResponse {
            status: 200,
            etag: None,
            body: b"[]".to_vec(),
        },
        HttpResponse {
            status: 201,
            etag: Some("v2".into()),
            body: Vec::new(),
        },
        HttpResponse {
            status: 200,
            etag: None,
            body: item(&format!("replica/{}", created.as_str()), "v2").into_bytes(),
        },
    ]);
    let mut storage = SupabaseStorage::new(transport, ORIGIN, BUCKET).unwrap();
    assert_eq!(storage.list(&replica_prefix()).unwrap()[0].blob_id, id);
    storage
        .put_if_unmodified(&replica_prefix(), &created, &[0xff], None)
        .unwrap();
    let recorded = storage.transport.recorded.lock().unwrap().clone();
    assert!(recorded
        .iter()
        .all(|line| line.contains("example.supabase.co") || line.contains("POST")));
    assert!(recorded.iter().any(|line| {
        line.contains(&format!("/object/{BUCKET}/replica/{}", created.as_str()))
            && line.contains("x-upsert")
            && !line.contains("%2F")
    }));
    assert!(recorded.iter().all(|line| !line.contains("Bearer")
        && !line.contains("access_token")
        && !line.contains(&forbidden_key_name())));
}

#[test]
fn list_keeps_hashed_names_only() {
    let hashed = blob(0xaa);
    let body = format!(
        r#"[{{"name":"replica/{}","metadata":{{"eTag":"1","size":4}}}},{{"name":"journals/claude.md","metadata":{{"eTag":"2","size":9}}}}]"#,
        hashed.as_str()
    );
    let files = parse_object_list(body.as_bytes()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].object.blob_id, hashed);
    assert!(!files[0].name.contains("claude.md"));
}

#[test]
fn storage_http_409_is_precondition_and_secret_free() {
    let transport = ScriptedTransport::new(vec![
        HttpResponse {
            status: 200,
            etag: None,
            body: b"[]".to_vec(),
        },
        HttpResponse {
            status: 409,
            etag: None,
            body: b"{\"message\":\"secret-token\"}".to_vec(),
        },
    ]);
    let mut storage = SupabaseStorage::new(transport, ORIGIN, BUCKET).unwrap();
    let err = storage
        .put_if_unmodified(&replica_prefix(), &blob(0xdd), &[0xff], None)
        .unwrap_err();
    assert_eq!(err, SyncError::Precondition);
    assert!(!format!("{err}").contains("secret-token"));
    assert!(!format!("{err}").contains("Bearer"));
}

#[test]
fn stale_etag_does_not_upload() {
    let id = blob(0xcc);
    let name = format!("replica/{}", id.as_str());
    let transport = ScriptedTransport::new(vec![HttpResponse {
        status: 200,
        etag: None,
        body: item(&name, "old").into_bytes(),
    }]);
    let mut storage = SupabaseStorage::new(transport, ORIGIN, BUCKET).unwrap();
    assert_eq!(
        storage
            .put_if_unmodified(&replica_prefix(), &id, &[1], Some(&ETag::new("stale")))
            .unwrap_err(),
        SyncError::Precondition
    );
    assert_eq!(storage.transport.recorded.lock().unwrap().len(), 1);
}

#[test]
fn failed_put_does_not_prune_device_prefix() {
    let replica = blob(0x11);
    let device = DeviceId::generate();
    let prefix = device_prefix(&device).unwrap();
    let transport = ScriptedTransport::new(vec![
        HttpResponse {
            status: 200,
            etag: None,
            body: b"[]".to_vec(),
        },
        HttpResponse {
            status: 409,
            etag: None,
            body: Vec::new(),
        },
    ]);
    let mut storage = SupabaseStorage::new(transport, ORIGIN, BUCKET).unwrap();
    let result = storage.advance_replica(ReplicaAdvance {
        puts: vec![super::super::client::ReplicaPut {
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
fn rejects_unsafe_bucket_and_origin() {
    assert!(
        SupabaseStorage::new(ScriptedTransport::new(Vec::new()), "http://example", BUCKET).is_err()
    );
    assert!(SupabaseStorage::new(ScriptedTransport::new(Vec::new()), ORIGIN, "a/b").is_err());
    assert_eq!(
        format!(
            "{:?}",
            SupabaseStorage::new(ScriptedTransport::new(Vec::new()), ORIGIN, BUCKET).unwrap()
        ),
        "SupabaseStorage(***)"
    );
}

fn forbidden_key_name() -> String {
    format!("{}{}{}", "cloud-sync", ".", "key")
}

#[test]
fn adapter_source_never_uploads_replica_key() {
    let src = concat!(
        include_str!("supabase.rs"),
        include_str!("supabase_auth.rs"),
        include_str!("supabase_tests.rs")
    );
    let wrap = format!("{}{}", "wrap", "ping");
    let key_name = forbidden_key_name();
    assert!(!src.contains(&key_name));
    assert!(!include_str!("supabase.rs").contains(&wrap));
    assert!(!include_str!("supabase.rs").contains("KeyEnvelope"));
    let sql = include_str!("../../../../docs/moon/supabase-storage.sql");
    assert!(sql.contains("public = false"));
    assert!(sql.contains("[0-9a-f]{64}"));
    assert!(sql.contains("devices/[0-9a-f]{32}"));
    assert!(sql.contains("to authenticated"));
    assert!(!sql.contains("|journals"));
    assert!(!sql.contains("to anon"));
    assert!(!sql.contains(&key_name));
}
