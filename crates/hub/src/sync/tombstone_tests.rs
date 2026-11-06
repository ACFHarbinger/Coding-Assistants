use super::*;
use crate::sync::review::{apply_choice, ConflictChoice};
use tempfile::tempdir;

fn stage_delete(home: &Path, slug: &str, path: &str, local: &[u8]) {
    let dir = conflicts_root(home).join(slug);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("reason.txt"), "delete-confirm").unwrap();
    fs::write(dir.join("path.txt"), path).unwrap();
    fs::write(dir.join("local"), local).unwrap();
    fs::write(dir.join("created_at.txt"), Utc::now().to_rfc3339()).unwrap();
}

fn write_live(home: &Path, rel: &str, bytes: &[u8]) {
    let path = home.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
}

fn write_stone(home: &Path, slug: &str, created: DateTime<Utc>) {
    let stone = Tombstone {
        slug: slug.into(),
        path: slug.replace("__", "/"),
        created_at: created.to_rfc3339(),
        expires_at: (created + Duration::days(DEFAULT_RETENTION_DAYS)).to_rfc3339(),
        content_hash: "aa".into(),
        policy: POLICY_CONFIRM_ONLY.into(),
    };
    atomic_write(
        &tombstones_root(home).join(format!("{slug}.json")),
        &serde_json::to_vec(&stone).unwrap(),
    )
    .unwrap();
}

#[test]
fn confirm_only_delete_writes_tombstone_and_removes_live() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    stage_delete(dir.path(), "markdown__note.md", "markdown/note.md", b"L");
    write_live(dir.path(), "markdown/note.md", b"L");
    let stone = confirm_delete(&store, "markdown__note.md").unwrap();
    assert_eq!(stone.policy, POLICY_CONFIRM_ONLY);
    assert!(!dir.path().join("markdown/note.md").exists());
    assert!(tombstones_root(dir.path())
        .join("markdown__note.md.json")
        .is_file());
    assert_eq!(
        fs::read(conflicts_root(dir.path()).join("markdown__note.md/local")).unwrap(),
        b"L"
    );
    let row = store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .find(|e| e.operation == "sync-tombstone")
        .unwrap();
    assert!(row.process_json.contains("markdown__note.md"));
    assert!(!row.process_json.contains("\"L\""));
    assert!(!row.process_json.contains("cloud-sync.key"));
}

#[test]
fn apply_remote_missing_is_confirm_click_other_choices_keep_live() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    stage_delete(dir.path(), "markdown__keep.md", "markdown/keep.md", b"K");
    write_live(dir.path(), "markdown/keep.md", b"K");
    apply_choice(&store, "markdown__keep.md", ConflictChoice::Local).unwrap();
    assert_eq!(fs::read(dir.path().join("markdown/keep.md")).unwrap(), b"K");
    apply_choice(&store, "markdown__keep.md", ConflictChoice::KeepBoth).unwrap();
    assert!(dir.path().join("markdown/keep.md").is_file());
    apply_choice(&store, "markdown__keep.md", ConflictChoice::Manual).unwrap();
    assert!(dir.path().join("markdown/keep.md").is_file());
    apply_choice(&store, "markdown__gone.md", ConflictChoice::Remote).unwrap_err();
    stage_delete(dir.path(), "markdown__gone.md", "markdown/gone.md", b"G");
    write_live(dir.path(), "markdown/gone.md", b"G");
    apply_choice(&store, "markdown__gone.md", ConflictChoice::Remote).unwrap();
    assert!(!dir.path().join("markdown/gone.md").exists());
    assert!(tombstones_root(dir.path())
        .join("markdown__gone.md.json")
        .is_file());
}

#[test]
fn hub_db_confirm_delete_refuses() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let live = fs::read(dir.path().join("hub.db")).unwrap();
    stage_delete(dir.path(), "hub.db", "hub.db", b"snap");
    let err = confirm_delete(&store, "hub.db").unwrap_err();
    assert!(err.to_string().contains("hub.db"));
    assert_eq!(fs::read(dir.path().join("hub.db")).unwrap(), live);
    assert!(list_tombstones(dir.path()).unwrap().is_empty());
}

#[test]
fn list_empty_when_no_tombstone_file() {
    let dir = tempdir().unwrap();
    assert!(list_tombstones(dir.path()).unwrap().is_empty());
    fs::create_dir_all(tombstones_root(dir.path())).unwrap();
    assert!(list_tombstones(dir.path()).unwrap().is_empty());
}

#[test]
fn expired_listed_but_listing_does_not_delete() {
    let dir = tempdir().unwrap();
    let created = Utc::now() - Duration::days(31);
    write_stone(dir.path(), "markdown__old.md", created);
    let cdir = conflicts_root(dir.path()).join("markdown__old.md");
    fs::create_dir_all(&cdir).unwrap();
    fs::write(cdir.join("created_at.txt"), created.to_rfc3339()).unwrap();
    fs::write(cdir.join("local"), b"old").unwrap();
    let listed = expired_cleanup_candidates(dir.path(), Utc::now()).unwrap();
    assert!(listed.iter().any(|c| c.kind == "tombstone"));
    assert!(listed.iter().any(|c| c.kind == "conflict"));
    assert!(tombstones_root(dir.path())
        .join("markdown__old.md.json")
        .is_file());
    assert!(cdir.join("local").is_file());
}

#[test]
fn purge_expired_removes_only_owner_selected_expired() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let old = Utc::now() - Duration::days(40);
    write_stone(dir.path(), "markdown__old.md", old);
    let cdir = conflicts_root(dir.path()).join("markdown__old.md");
    fs::create_dir_all(&cdir).unwrap();
    fs::write(cdir.join("created_at.txt"), old.to_rfc3339()).unwrap();
    write_stone(dir.path(), "markdown__fresh.md", Utc::now());
    let err = purge_expired(&store, &["markdown__fresh.md".into()], false).unwrap_err();
    assert!(err.to_string().contains("non-expired"));
    assert!(tombstones_root(dir.path())
        .join("markdown__fresh.md.json")
        .is_file());
    let report = purge_expired(&store, &[], true).unwrap();
    assert!(report.purged.iter().any(|s| s.contains("markdown__old.md")));
    assert!(!tombstones_root(dir.path())
        .join("markdown__old.md.json")
        .exists());
    assert!(!cdir.exists());
    assert!(tombstones_root(dir.path())
        .join("markdown__fresh.md.json")
        .is_file());
}

#[test]
fn malformed_slug_fail_closed() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    stage_delete(dir.path(), "markdown__note.md", "markdown/note.md", b"L");
    write_live(dir.path(), "markdown/note.md", b"L");
    let err = confirm_delete(&store, "..").unwrap_err();
    assert!(err.to_string().contains("invalid conflict slug"));
    assert_eq!(fs::read(dir.path().join("markdown/note.md")).unwrap(), b"L");
    let err = purge_expired(&store, &["..".into()], false).unwrap_err();
    assert!(err.to_string().contains("invalid conflict slug"));
}
