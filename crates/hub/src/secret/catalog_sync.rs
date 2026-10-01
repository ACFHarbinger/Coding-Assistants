//! Cloud-sync catalog rows, split from `catalog.rs` to honor the 500-LoC cap.

use super::{FieldSpec, OwnerKind, Scope};

pub const SUPABASE_FIELDS: &[FieldSpec] = &[
    FieldSpec {
        id: "tool.sync.supabase_access_token",
        display_name: "Supabase Access Token",
        owner_kind: OwnerKind::Tool,
        owner_key: "sync",
        env_var: Some("SUPABASE_ACCESS_TOKEN"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://supabase.com/docs/guides/auth"),
        notes: Some("Authenticated-owner Storage bearer token. Vault wins. Never logged."),
    },
    FieldSpec {
        id: "tool.sync.supabase_url",
        display_name: "Supabase Project URL",
        owner_kind: OwnerKind::Tool,
        owner_key: "sync",
        env_var: Some("SUPABASE_URL"),
        secret: false,
        scope: Scope::Global,
        docs_url: Some("https://supabase.com/docs/guides/api"),
        notes: Some("HTTPS project origin only; query, fragment, and credentials are rejected."),
    },
    FieldSpec {
        id: "tool.sync.supabase_storage_bucket",
        display_name: "Supabase Storage Bucket",
        owner_kind: OwnerKind::Tool,
        owner_key: "sync",
        env_var: Some("SUPABASE_STORAGE_BUCKET"),
        secret: false,
        scope: Scope::Global,
        docs_url: Some("https://supabase.com/docs/guides/storage"),
        notes: Some(
            "Private Storage bucket name only; RLS must limit it to the authenticated owner.",
        ),
    },
];
