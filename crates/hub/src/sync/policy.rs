//! Classify local `.coding-assistants` relative paths into the locked category table.

use super::types::{Category, CategoryPolicy};
use std::path::Path;

/// Map a path relative to the hub home onto a [`Category`].
pub fn classify(relative: &Path) -> Category {
    let mut comps = relative.components();
    let first = comps.next().and_then(|c| c.as_os_str().to_str());
    match first {
        Some("keys") => match comps.next().and_then(|c| c.as_os_str().to_str()) {
            Some("cloud-sync.key") => Category::CloudSyncKey,
            Some("journals") => Category::JournalKey,
            _ => Category::SecretConfig,
        },
        Some("sync") => Category::SyncLocal,
        Some("hub.db") | Some("hub.db-wal") | Some("hub.db-shm") => Category::HubDatabase,
        Some(name) if name.starts_with("hub.db") => Category::HubDatabase,
        Some("journals") => Category::PrivateJournal,
        Some("markdown") => Category::MarkdownExport,
        Some("wake") => Category::Wake,
        Some("cache" | "caches" | "tmp") => Category::Cache,
        Some("secrets.vault") | Some("settings.toml") => Category::SecretConfig,
        Some("attachments") => Category::SharedDurable,
        _ => Category::SharedDurable,
    }
}

pub fn default_policy(category: Category) -> CategoryPolicy {
    match category {
        Category::HubDatabase | Category::SharedDurable => CategoryPolicy::Snapshot,
        Category::PrivateJournal | Category::Audit | Category::MarkdownExport => {
            CategoryPolicy::Include
        }
        Category::Cache | Category::Wake => CategoryPolicy::Exclude,
        Category::SecretConfig
        | Category::CloudSyncKey
        | Category::JournalKey
        | Category::SyncLocal => CategoryPolicy::LocalOnly,
    }
}

pub fn may_upload(policy: CategoryPolicy) -> bool {
    matches!(
        policy,
        CategoryPolicy::Include | CategoryPolicy::Snapshot | CategoryPolicy::DownloadOnly
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn mandatory_excludes_are_local_only() {
        let cases = [
            ("keys/cloud-sync.key", Category::CloudSyncKey),
            ("keys/journals/claude.key", Category::JournalKey),
            ("sync/lock", Category::SyncLocal),
            ("sync/staging/part", Category::SyncLocal),
            ("secrets.vault", Category::SecretConfig),
        ];
        for (path, category) in cases {
            assert_eq!(classify(Path::new(path)), category, "{path}");
            assert_eq!(default_policy(category), CategoryPolicy::LocalOnly);
            assert!(!may_upload(CategoryPolicy::LocalOnly));
        }
    }

    #[test]
    fn journals_are_not_confused_with_journal_keys() {
        assert_eq!(
            classify(Path::new("journals/claude.md")),
            Category::PrivateJournal
        );
        assert_eq!(
            classify(Path::new("keys/journals/claude.key")),
            Category::JournalKey
        );
    }

    #[test]
    fn hub_db_and_wal_are_snapshot() {
        for path in ["hub.db", "hub.db-wal", "hub.db-shm"] {
            let category = classify(Path::new(path));
            assert_eq!(category, Category::HubDatabase, "{path}");
            assert_eq!(default_policy(category), CategoryPolicy::Snapshot);
            assert!(may_upload(CategoryPolicy::Snapshot));
        }
    }
}
