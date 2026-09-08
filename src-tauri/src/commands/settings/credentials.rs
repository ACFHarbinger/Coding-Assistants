//! Write-only credential set / replace / clear and status Tauri commands (#283).
//!
//! # Security invariants (enforced here, not just documented)
//!
//! * **No command returns a stored secret.** Every command returns only
//!   [`hub::secret::SecretStatus`] — presence flag, source tag, and last-updated
//!   timestamp. The actual credential value never crosses the IPC boundary.
//! * **Values are dropped immediately.** The incoming `value` string is passed
//!   straight to `hub::secret::set_secret` and then the Rust binding is dropped;
//!   no copy lands in a log, error string, or `Debug` impl.
//! * **Redacted audit on every set / clear.** Only the field identifier and
//!   the action (`"set"` / `"clear"`) are recorded. The `value` is never
//!   included.
//! * **Catalog-validated field ids.** Every command first checks that the
//!   `field_id` exists in the #285 static catalog and that the caller-supplied
//!   key is accepted by `hub::secret::validate_key`.

use hub::secret::catalog::{FieldSpec, CATALOG};
use hub::secret::{clear_secret, field, secret_status, set_secret, SecretStatus};

/// Return the full static credential/config field catalog. Contains only
/// metadata (display name, env-var name, etc.) — no secret values ever.
#[tauri::command]
pub fn settings_list_credential_fields() -> Vec<FieldSpec> {
    CATALOG.to_vec()
}

/// Record a credential-management event on the settings audit stream.
/// Only the field identifier and action are written — the credential value
/// is never passed here.
fn record_credential_audit(field_id: &str, action: &str) {
    // Best-effort: open the store and write; silently drop on failure rather
    // than letting an audit failure mask a successful credential operation.
    if let Ok(store) = super::store::open_store() {
        let _ = store.record_settings_audit_event(field_id, "global", action);
    }
}

/// Resolve the vault key for a catalog field id.
///
/// Returns `Err` with a user-facing message if the id is unknown or the
/// derived vault key fails validation.
fn vault_key_for(field_id: &str) -> Result<&'static str, String> {
    let spec = field(field_id).ok_or_else(|| format!("unknown credential field: {field_id}"))?;
    if !spec.secret {
        return Err(format!(
            "field is configuration, not a credential: {field_id}"
        ));
    }
    hub::secret::validate_key(spec.vault_key()).map_err(|e| e.to_string())?;
    Ok(spec.vault_key())
}

/// Store or replace the secret for a known catalog field.
///
/// `field_id` must be a valid catalog identifier (e.g. `"provider.deepseek.api_key"`).
/// `value` is accepted write-only and is never returned or logged.
///
/// Returns the non-secret [`SecretStatus`] after the write: source, presence,
/// and last-updated timestamp only.
#[tauri::command]
pub fn settings_set_credential(field_id: String, value: String) -> Result<SecretStatus, String> {
    let vault_key = vault_key_for(&field_id)?;
    let status = set_secret(vault_key, &value).map_err(|e| e.to_string())?;
    record_credential_audit(&field_id, "set");
    Ok(status)
}

/// Remove the stored secret for a known catalog field.
///
/// After clearing, [`hub::secret::resolve`] falls back to the environment
/// variable named in the catalog spec (if any). Clearing an absent key is
/// idempotent and returns `Ok`.
///
/// Returns the non-secret [`SecretStatus`] after the delete.
#[tauri::command]
pub fn settings_clear_credential(field_id: String) -> Result<SecretStatus, String> {
    let vault_key = vault_key_for(&field_id)?;
    let status = clear_secret(vault_key).map_err(|e| e.to_string())?;
    record_credential_audit(&field_id, "clear");
    Ok(status)
}

/// Return the non-secret status of a catalog field's credential: whether it
/// is set, where it came from (keychain / file / env-var / none), and when it
/// was last written (vault entries only).
///
/// This command never errors on an unknown backend or unset key — it returns
/// a `SecretStatus` with `isSet: false` and `source: "none"` in those cases.
/// An unknown `field_id` still returns `Err` so the frontend can distinguish
/// "I don't know that field" from "I know it, but nothing is stored yet".
#[tauri::command]
pub fn settings_get_credential_status(field_id: String) -> Result<SecretStatus, String> {
    let vault_key = vault_key_for(&field_id)?;
    Ok(secret_status(vault_key))
}

// ─── Linked external accounts (#286 / H7) ───────────────────────────────────

/// Known linkable account providers surfaced in the Settings UI (#284 / #286).
const KNOWN_LINKABLE_PROVIDERS: &[(&str, &str)] = &[
    ("openai", "oauth_device"),
    ("anthropic", "vendor_cli_login"),
    ("google", "vendor_cli_login"),
    ("deepseek", "oauth_device"),
    ("meta", "oauth_device"),
    ("xai", "oauth_device"),
    ("mistral", "oauth_device"),
    ("perplexity", "vendor_cli_login"),
];

fn known_linkable_provider(provider: &str) -> bool {
    KNOWN_LINKABLE_PROVIDERS
        .iter()
        .any(|(known, _)| *known == provider)
}

/// List all linked external provider accounts (#286), merged with unlinked
/// status for standard providers (ChatGPT, Claude, Google, etc.).
/// Scoped to owner = "local" (provisional single-user key).
#[tauri::command]
pub fn hub_list_linked_accounts() -> Result<Vec<hub::LinkedAccountStatus>, String> {
    let store = super::store::open_store()?;
    let records = store
        .list_linked_accounts("local")
        .map_err(|e| e.to_string())?;

    let mut statuses = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (provider, default_kind) in KNOWN_LINKABLE_PROVIDERS {
        seen.insert(*provider);
        if let Some(rec) = records.iter().find(|r| r.provider == *provider) {
            statuses.push(hub::LinkedAccountStatus {
                provider: rec.provider.clone(),
                external_label: rec.external_label.clone(),
                connection_kind: rec.connection_kind.clone(),
                is_linked: true,
                linked_at: Some(rec.linked_at),
                source: if rec.token_ref.is_some() {
                    "vault".into()
                } else {
                    "vendor_cli".into()
                },
            });
        } else {
            statuses.push(hub::LinkedAccountStatus {
                provider: (*provider).into(),
                external_label: None,
                connection_kind: (*default_kind).into(),
                is_linked: false,
                linked_at: None,
                source: "none".into(),
            });
        }
    }

    for rec in records {
        if !seen.contains(rec.provider.as_str()) {
            statuses.push(hub::LinkedAccountStatus {
                provider: rec.provider.clone(),
                external_label: rec.external_label.clone(),
                connection_kind: rec.connection_kind.clone(),
                is_linked: true,
                linked_at: Some(rec.linked_at),
                source: if rec.token_ref.is_some() {
                    "vault".into()
                } else {
                    "vendor_cli".into()
                },
            });
        }
    }

    Ok(statuses)
}

/// Record an external provider account that uses the vendor's native CLI
/// login (#286). Provisional single-user storage under owner = "local".
#[tauri::command]
pub fn hub_link_account_cli(
    provider: String,
    external_label: Option<String>,
) -> Result<hub::LinkedAccountStatus, String> {
    let provider = provider.trim().to_lowercase();
    if !known_linkable_provider(&provider) {
        return Err("unsupported account provider".into());
    }
    let label = external_label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if label.is_some_and(|value| value.len() > 256 || value.chars().any(char::is_control)) {
        return Err("invalid external account label".into());
    }

    let store = super::store::open_store()?;
    store
        .link_account("local", &provider, label, "vendor_cli_login", None)
        .map_err(|e| e.to_string())?;

    record_credential_audit(&format!("account.{provider}"), "link_cli");

    Ok(hub::LinkedAccountStatus {
        provider,
        external_label: label.map(String::from),
        connection_kind: "vendor_cli_login".into(),
        is_linked: true,
        linked_at: Some(chrono::Utc::now().timestamp()),
        source: "vendor_cli".into(),
    })
}

/// Disconnect / unlink an external provider account (#286).
/// Deletes the vault entry if present and removes the row from hub.db.
#[tauri::command]
pub fn hub_unlink_account(provider: String) -> Result<bool, String> {
    let provider = provider.trim().to_lowercase();
    if !known_linkable_provider(&provider) {
        return Err("unsupported account provider".into());
    }
    let store = super::store::open_store()?;

    if let Ok(Some(rec)) = store.get_linked_account("local", &provider) {
        if let Some(token_ref) = rec.token_ref {
            let _ = hub::secret::clear_secret(&token_ref);
        }
    }

    let unlinked = store
        .unlink_account("local", &provider)
        .map_err(|e| e.to_string())?;
    record_credential_audit(&format!("account.{provider}"), "unlink");
    Ok(unlinked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub::secret::{SecretSource, CATALOG};

    /// Every secret catalog field id that routes through the vault must be
    /// accepted by `vault_key_for`; configuration fields belong to their
    /// dedicated settings path and must never be stored as credentials.
    #[test]
    fn vault_key_resolves_for_secret_catalog_fields_only() {
        for spec in CATALOG.iter().filter(|spec| spec.secret) {
            let result = vault_key_for(spec.id);
            assert!(
                result.is_ok(),
                "vault_key_for failed for catalog field {}: {:?}",
                spec.id,
                result
            );
        }
    }

    #[test]
    fn vault_key_for_configuration_field_returns_error() {
        let err = vault_key_for("provider.deepseek.base_url").unwrap_err();
        assert!(err.contains("configuration, not a credential"));
    }

    /// A field id that is not in the catalog must produce a clear error message.
    #[test]
    fn vault_key_for_unknown_id_returns_error() {
        let err = vault_key_for("not.a.real.field").unwrap_err();
        assert!(
            err.contains("unknown credential field"),
            "unexpected error message: {err}"
        );
    }

    /// `settings_get_credential_status` with an unknown field id returns `Err`.
    #[test]
    fn get_status_unknown_field_returns_err() {
        let result = settings_get_credential_status("no.such.field".to_string());
        assert!(result.is_err());
    }

    /// A known but unset field returns `SecretStatus { is_set: false, source: None }`.
    /// We use a key that is extremely unlikely to be set in the test environment.
    #[test]
    fn get_status_known_unset_field_returns_not_set() {
        // DeepSeek key is in the catalog; assume it is not set in the test env.
        // If it *is* set (e.g. CI with a real key), the test still passes
        // because the command must not error — it may just report `is_set: true`.
        let result = settings_get_credential_status("provider.deepseek.api_key".to_string());
        assert!(
            result.is_ok(),
            "status command must not error for a known field: {:?}",
            result
        );
        let status = result.unwrap();
        assert_eq!(status.key, "DEEPSEEK_API_KEY"); // vault key = env_var for this field
                                                    // source is either None (unset) or EnvVar/Keychain (if set in the env)
        assert!(matches!(
            status.source,
            SecretSource::None | SecretSource::EnvVar | SecretSource::Keychain | SecretSource::File
        ));
    }

    /// Attempting to set a credential for an unknown field must fail before
    /// touching the backend.
    #[test]
    fn set_credential_unknown_field_returns_err() {
        let result = settings_set_credential("not.a.real.field".to_string(), "s3cr3t".to_string());
        assert!(result.is_err());
    }

    /// Clearing an unknown field must also fail early.
    #[test]
    fn clear_credential_unknown_field_returns_err() {
        let result = settings_clear_credential("not.a.real.field".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn linked_accounts_lists_known_providers() {
        let _guard = crate::commands::commands::tests::CA_HOME_ENV_LOCK
            .lock()
            .unwrap();
        let list = hub_list_linked_accounts().unwrap();
        assert!(list.iter().any(|a| a.provider == "openai"));
        assert!(list.iter().any(|a| a.provider == "anthropic"));
        assert!(list.iter().any(|a| a.provider == "google"));
    }

    #[test]
    fn link_and_unlink_account_cli_round_trip() {
        let _guard = crate::commands::commands::tests::CA_HOME_ENV_LOCK
            .lock()
            .unwrap();
        let dir = std::env::temp_dir().join(format!(
            "hub-tauri-linked-account-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::env::set_var("CA_HOME", &dir);

        let linked =
            hub_link_account_cli("anthropic".to_string(), Some("test-user".to_string())).unwrap();
        assert_eq!(linked.provider, "anthropic");
        assert_eq!(linked.external_label.as_deref(), Some("test-user"));
        assert!(linked.is_linked);

        let list = hub_list_linked_accounts().unwrap();
        let found = list.iter().find(|a| a.provider == "anthropic").unwrap();
        assert!(found.is_linked);
        assert_eq!(found.external_label.as_deref(), Some("test-user"));

        let unlinked = hub_unlink_account("anthropic".to_string()).unwrap();
        assert!(unlinked);

        let list = hub_list_linked_accounts().unwrap();
        let found = list.iter().find(|a| a.provider == "anthropic").unwrap();
        assert!(!found.is_linked);

        std::env::remove_var("CA_HOME");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
