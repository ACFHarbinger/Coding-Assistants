use super::super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn embedding_provider_defaults_to_local_and_round_trips() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    assert_eq!(
        store.snapshot().embedding_provider,
        EmbeddingProvider::Local
    );

    store.set_embedding_provider(EmbeddingProvider::Openai);
    store.save().unwrap();

    let reloaded = SettingsStore::open(dir.path());
    assert_eq!(
        reloaded.snapshot().embedding_provider,
        EmbeddingProvider::Openai
    );
    assert!(fs::read_to_string(reloaded.path())
        .unwrap()
        .contains("embedding_provider = \"openai\""));
}

#[test]
fn unknown_embedding_provider_is_invalid() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("settings.toml"),
        "schema_version = 1\n[storage]\nbackup_retention = 3\n[memory]\nembedding_provider = \"remote\"\n",
    )
    .unwrap();

    let store = SettingsStore::open(dir.path());
    assert!(matches!(store.load().status, LoadStatus::Invalid { .. }));
}
