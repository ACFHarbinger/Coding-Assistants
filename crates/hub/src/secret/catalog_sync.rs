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

pub const ONEDRIVE_FIELDS: &[FieldSpec] = &[
    FieldSpec {
        id: "tool.sync.onedrive_refresh_token",
        display_name: "OneDrive Refresh Token",
        owner_kind: OwnerKind::Tool,
        owner_key: "sync",
        env_var: Some("ONEDRIVE_REFRESH_TOKEN"),
        secret: true,
        scope: Scope::Global,
        docs_url: Some("https://learn.microsoft.com/graph/onedrive-concept-overview"),
        notes: Some("App-folder refresh token. Vault wins. Never logged."),
    },
    FieldSpec {
        id: "tool.sync.onedrive_client_id",
        display_name: "OneDrive Client ID",
        owner_kind: OwnerKind::Tool,
        owner_key: "sync",
        env_var: Some("ONEDRIVE_CLIENT_ID"),
        secret: false,
        scope: Scope::Global,
        docs_url: Some("https://learn.microsoft.com/entra/identity-platform/"),
        notes: Some("Public client id for the token refresh form. Not a secret."),
    },
];

pub const DROPBOX_FIELDS: &[FieldSpec] = &[FieldSpec {
    id: "tool.sync.dropbox_access_token",
    display_name: "Dropbox Access Token",
    owner_kind: OwnerKind::Tool,
    owner_key: "sync",
    env_var: Some("DROPBOX_ACCESS_TOKEN"),
    secret: true,
    scope: Scope::Global,
    docs_url: Some("https://www.dropbox.com/developers/documentation/http/documentation"),
    notes: Some("App-folder bearer token. Vault wins. Never logged."),
}];
