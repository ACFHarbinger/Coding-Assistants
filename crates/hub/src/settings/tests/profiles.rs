use super::super::*;
use std::fs;
use tempfile::tempdir;

fn sample_profile(name: &str, provider: &str) -> ProviderProfile {
    ProviderProfile {
        name: name.into(),
        provider: provider.into(),
        model: Some("grok-4".into()),
        base_url: Some("https://api.x.ai".into()),
        secret: SecretReference::EnvVar {
            name: "XAI_API_KEY".into(),
        },
    }
}

#[test]
fn profiles_round_trip_without_persisting_secrets() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .upsert_profile(sample_profile("work", "grok"))
        .unwrap();
    store
        .upsert_profile(ProviderProfile {
            name: "login".into(),
            provider: "claude".into(),
            model: None,
            base_url: None,
            secret: SecretReference::ProviderLogin,
        })
        .unwrap();
    store.save().unwrap();
    let raw = fs::read_to_string(store.path()).unwrap();
    assert!(raw.contains("name = \"work\""));
    assert!(raw.contains("secret_source = \"env_var\""));
    assert!(raw.contains("secret_ref = \"XAI_API_KEY\""));
    assert!(!raw.to_ascii_lowercase().contains("sk-"));
    assert!(!raw.contains("Bearer"));
    let reloaded = SettingsStore::open(dir.path());
    let work = reloaded
        .list_profiles()
        .into_iter()
        .find(|p| p.name == "work")
        .unwrap();
    assert_eq!(work.secret_source, SecretSourceKind::EnvVar);
    assert_eq!(work.secret_badge, "Env Var $XAI_API_KEY");
    assert_eq!(
        reloaded.profile("login").unwrap().secret,
        SecretReference::ProviderLogin
    );
}

#[test]
fn profile_rename_updates_workspace_default_selection() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .upsert_profile(sample_profile("work", "grok"))
        .unwrap();
    store
        .set_workspace_default_profile("/abs/repo", "grok", "work")
        .unwrap();
    store.rename_profile("work", "office").unwrap();
    store.save().unwrap();
    let selected = store
        .workspace_override("/abs/repo")
        .unwrap()
        .default_profiles
        .get("grok")
        .cloned();
    assert_eq!(selected.as_deref(), Some("office"));
    assert!(store.profile("work").is_none());
}

#[test]
fn profile_remove_clears_workspace_selection_and_keeps_no_secret_delete() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .upsert_profile(ProviderProfile {
            name: "stored".into(),
            provider: "chat".into(),
            model: None,
            base_url: None,
            secret: SecretReference::Keychain {
                id: "ca.profile.stored".into(),
            },
        })
        .unwrap();
    store
        .set_workspace_default_profile("/abs/repo", "chat", "stored")
        .unwrap();
    let removed = store.remove_profile("stored").unwrap();
    assert_eq!(removed.secret.badge(), "Stored in System Keychain");
    assert!(store
        .workspace_override("/abs/repo")
        .map(|over| over.default_profiles.is_empty())
        .unwrap_or(true));
}

#[test]
fn workspace_cannot_copy_or_select_a_foreign_profile() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .upsert_profile(sample_profile("work", "grok"))
        .unwrap();
    assert!(store
        .set_workspace_default_profile("/abs/repo", "claude", "work")
        .is_err());
    assert!(store
        .upsert_profile(ProviderProfile {
            name: "bad".into(),
            provider: "grok".into(),
            model: Some("sk-secret".into()),
            base_url: None,
            secret: SecretReference::ProviderLogin
        })
        .is_err());
}

#[test]
fn harness_settings_validate_executable_and_absolute_workdir() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    assert!(store
        .set_harness_settings(HarnessSettings {
            harness: "grok".into(),
            executable: "grok --evil".into(),
            workdir: None,
            capture_polling: true,
            inject_permission: true,
            default_model: None,
            default_effort: None,
        })
        .is_err());
    assert!(store
        .set_harness_settings(HarnessSettings {
            harness: "grok".into(),
            executable: "grok".into(),
            workdir: Some("relative".into()),
            capture_polling: true,
            inject_permission: false,
            default_model: None,
            default_effort: None,
        })
        .is_err());
    store
        .set_harness_settings(HarnessSettings {
            harness: "grok".into(),
            executable: "/usr/bin/grok".into(),
            workdir: Some("/abs/ws".into()),
            capture_polling: false,
            inject_permission: false,
            default_model: Some("grok-4.6".into()),
            default_effort: Some("high".into()),
        })
        .unwrap();
    store.save().unwrap();
    let harness = SettingsStore::open(dir.path())
        .harness_settings("grok")
        .unwrap();
    assert_eq!(harness.executable, "/usr/bin/grok");
    assert_eq!(harness.workdir.as_deref(), Some("/abs/ws"));
    assert!(!harness.capture_polling);
    assert!(!harness.inject_permission);
    assert_eq!(harness.default_model.as_deref(), Some("grok-4.6"));
    assert_eq!(harness.default_effort.as_deref(), Some("high"));
}

#[test]
fn workspace_default_profile_is_a_name_reference_not_a_copy() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());
    store
        .upsert_profile(sample_profile("work", "grok"))
        .unwrap();
    store
        .set_workspace_default_profile("/abs/repo", "grok", "work")
        .unwrap();
    store.save().unwrap();
    let raw = fs::read_to_string(store.path()).unwrap();
    assert!(
        raw.contains("work = \"work\"") || raw.contains("grok = \"work\""),
        "{raw}"
    );
    assert_eq!(raw.matches("https://api.x.ai").count(), 1);
    store
        .upsert_profile(ProviderProfile {
            name: "work".into(),
            provider: "grok".into(),
            model: Some("grok-4.1".into()),
            base_url: Some("https://api.x.ai".into()),
            secret: SecretReference::EnvVar {
                name: "XAI_API_KEY".into(),
            },
        })
        .unwrap();
    let effective = store.effective_harness(Some("/abs/repo"), "grok").unwrap();
    assert_eq!(effective.default_profile.as_deref(), Some("work"));
    assert_eq!(effective.default_profile_status, FieldStatus::Override);
    assert_eq!(
        effective.default_profile_badge.as_deref(),
        Some("Env Var $XAI_API_KEY")
    );
    assert_eq!(
        store.profile("work").unwrap().model.as_deref(),
        Some("grok-4.1")
    );
}

#[test]
fn workspace_default_model_and_effort_overrides_behave_properly() {
    let dir = tempdir().unwrap();
    let mut store = SettingsStore::open(dir.path());

    let eff_global = store.effective_harness(None, "opencode").unwrap();
    assert_eq!(
        eff_global.selected_model.as_deref(),
        Some("opencode-go/glm-5.3")
    );
    assert_eq!(eff_global.selected_model_status, FieldStatus::Inherited);

    store
        .set_workspace_default_model("/abs/myproject", "opencode", "deepseek/deepseek-v4-flash")
        .unwrap();
    store
        .set_workspace_default_effort("/abs/myproject", "opencode", "high")
        .unwrap();
    store.save().unwrap();

    let eff_ws = store
        .effective_harness(Some("/abs/myproject"), "opencode")
        .unwrap();
    assert_eq!(
        eff_ws.selected_model.as_deref(),
        Some("deepseek/deepseek-v4-flash")
    );
    assert_eq!(eff_ws.selected_model_status, FieldStatus::Override);
    assert_eq!(eff_ws.selected_effort.as_deref(), Some("high"));
    assert_eq!(eff_ws.selected_effort_status, FieldStatus::Override);

    store
        .reset_workspace_default_model("/abs/myproject", "opencode")
        .unwrap();
    store
        .reset_workspace_default_effort("/abs/myproject", "opencode")
        .unwrap();
    store.save().unwrap();

    let eff_reset = store
        .effective_harness(Some("/abs/myproject"), "opencode")
        .unwrap();
    assert_eq!(
        eff_reset.selected_model.as_deref(),
        Some("opencode-go/glm-5.3")
    );
    assert_eq!(eff_reset.selected_model_status, FieldStatus::Inherited);
}
