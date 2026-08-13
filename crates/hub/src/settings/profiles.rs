//! Global named provider profiles and per-harness process settings (S4).

use super::model::{
    EffectiveHarnessSettings, FieldStatus, HarnessSettings, ProviderProfile, SecretReference,
    SettingsError,
};
use crate::HarnessId;
use std::collections::BTreeMap;
use std::path::Path;
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table};

pub fn validate_profile_name(name: &str) -> Result<String, SettingsError> {
    let name = name.trim();
    if name.is_empty() || name.len() > 64 {
        return Err(SettingsError::Invalid(
            "profile name must be 1..=64 characters".into(),
        ));
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-')
    {
        return Err(SettingsError::Invalid(
            "profile name may contain only letters, digits, '.', '_' and '-'".into(),
        ));
    }
    reject_secret_looking(name, "profile name")?;
    Ok(name.to_string())
}

pub fn validate_provider(provider: &str) -> Result<String, SettingsError> {
    HarnessId::parse(provider)
        .map(|id| id.as_str().to_string())
        .map_err(|err| SettingsError::Invalid(err.to_string()))
}

pub fn validate_secret(secret: &SecretReference) -> Result<(), SettingsError> {
    match secret {
        SecretReference::Keychain { id } => {
            let id = id.trim();
            if id.is_empty() || id.len() > 128 {
                return Err(SettingsError::Invalid(
                    "keychain id must be 1..=128 characters".into(),
                ));
            }
            reject_secret_looking(id, "keychain id")?;
        }
        SecretReference::EnvVar { name } => {
            let name = name.trim();
            if name.is_empty()
                || !name.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
                || !name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return Err(SettingsError::Invalid(
                    "environment variable name must look like FOO_BAR, not a value".into(),
                ));
            }
        }
        SecretReference::ProviderLogin => {}
    }
    Ok(())
}

pub fn validate_executable(executable: &str) -> Result<String, SettingsError> {
    let executable = executable.trim();
    if executable.is_empty() {
        return Err(SettingsError::Invalid(
            "executable must not be empty".into(),
        ));
    }
    if executable.split_whitespace().count() != 1
        || executable.chars().any(|ch| {
            matches!(
                ch,
                ';' | '|' | '&' | '$' | '`' | '<' | '>' | '\n' | '\r' | '\0'
            )
        })
    {
        return Err(SettingsError::Invalid(
            "executable must be a single program name or path, not a shell command".into(),
        ));
    }
    Ok(executable.to_string())
}

pub fn validate_workdir(workdir: &str) -> Result<String, SettingsError> {
    let workdir = workdir.trim();
    if workdir.is_empty() {
        return Err(SettingsError::Invalid("workdir must not be empty".into()));
    }
    if !Path::new(workdir).is_absolute() {
        return Err(SettingsError::Invalid(
            "workdir must be an absolute path".into(),
        ));
    }
    if workdir.contains('\0') || workdir.contains('\n') {
        return Err(SettingsError::Invalid(
            "workdir contains invalid characters".into(),
        ));
    }
    Ok(workdir.to_string())
}

pub fn validate_profile(profile: &ProviderProfile) -> Result<(), SettingsError> {
    validate_profile_name(&profile.name)?;
    validate_provider(&profile.provider)?;
    if let Some(model) = &profile.model {
        reject_secret_looking(model, "model")?;
    }
    if let Some(base_url) = &profile.base_url {
        reject_secret_looking(base_url, "base_url")?;
        if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
            return Err(SettingsError::Invalid(
                "base_url must be an http(s) URL".into(),
            ));
        }
    }
    validate_secret(&profile.secret)?;
    Ok(())
}

pub fn validate_harness(settings: &HarnessSettings) -> Result<(), SettingsError> {
    validate_provider(&settings.harness)?;
    validate_executable(&settings.executable)?;
    if let Some(workdir) = &settings.workdir {
        validate_workdir(workdir)?;
    }
    Ok(())
}

fn reject_secret_looking(value: &str, field: &str) -> Result<(), SettingsError> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("sk-")
        || lower.contains("bearer ")
        || lower.contains("api_key=")
        || lower.contains("token=")
        || value.contains('\n')
    {
        return Err(SettingsError::Invalid(format!(
            "{field} must not contain a credential value"
        )));
    }
    Ok(())
}

pub fn write_profile_fields(
    document: &mut DocumentMut,
    profiles: &BTreeMap<String, ProviderProfile>,
) {
    if profiles.is_empty() {
        document.remove("profile");
        return;
    }
    let mut array = ArrayOfTables::new();
    for profile in profiles.values() {
        let mut table = Table::new();
        table["name"] = value(profile.name.as_str());
        table["provider"] = value(profile.provider.as_str());
        if let Some(model) = &profile.model {
            table["model"] = value(model.as_str());
        }
        if let Some(base_url) = &profile.base_url {
            table["base_url"] = value(base_url.as_str());
        }
        match &profile.secret {
            SecretReference::Keychain { id } => {
                table["secret_source"] = value("keychain");
                table["secret_ref"] = value(id.as_str());
            }
            SecretReference::EnvVar { name } => {
                table["secret_source"] = value("env_var");
                table["secret_ref"] = value(name.as_str());
            }
            SecretReference::ProviderLogin => {
                table["secret_source"] = value("provider_login");
            }
        }
        array.push(table);
    }
    document["profile"] = Item::ArrayOfTables(array);
}

pub fn write_harness_fields(
    document: &mut DocumentMut,
    harnesses: &BTreeMap<String, HarnessSettings>,
) {
    if harnesses.is_empty() {
        document.remove("harness");
        return;
    }
    if !document.contains_key("harness") {
        document["harness"] = Item::Table(Table::new());
    }
    if let Some(existing) = document["harness"].as_table_mut() {
        let stale: Vec<String> = existing.iter().map(|(key, _)| key.to_string()).collect();
        for key in stale {
            existing.remove(&key);
        }
    }
    for settings in harnesses.values() {
        let mut table = Table::new();
        table["executable"] = value(settings.executable.as_str());
        if let Some(workdir) = &settings.workdir {
            table["workdir"] = value(workdir.as_str());
        }
        table["capture_polling"] = value(settings.capture_polling);
        table["inject_permission"] = value(settings.inject_permission);
        document["harness"][settings.harness.as_str()] = Item::Table(table);
    }
}

pub fn profiles_from_document(
    document: &DocumentMut,
) -> Result<BTreeMap<String, ProviderProfile>, SettingsError> {
    let mut map = BTreeMap::new();
    let Some(array) = document.get("profile").and_then(Item::as_array_of_tables) else {
        return Ok(map);
    };
    for table in array.iter() {
        let name = table
            .get("name")
            .and_then(Item::as_str)
            .ok_or_else(|| SettingsError::Invalid("profile entry missing name".into()))?;
        let provider = table
            .get("provider")
            .and_then(Item::as_str)
            .ok_or_else(|| SettingsError::Invalid(format!("profile {name} missing provider")))?;
        let secret = match table
            .get("secret_source")
            .and_then(Item::as_str)
            .unwrap_or("")
        {
            "keychain" => SecretReference::Keychain {
                id: table
                    .get("secret_ref")
                    .and_then(Item::as_str)
                    .ok_or_else(|| {
                        SettingsError::Invalid(format!("profile {name} missing secret_ref"))
                    })?
                    .to_string(),
            },
            "env_var" => SecretReference::EnvVar {
                name: table
                    .get("secret_ref")
                    .and_then(Item::as_str)
                    .ok_or_else(|| {
                        SettingsError::Invalid(format!("profile {name} missing secret_ref"))
                    })?
                    .to_string(),
            },
            "provider_login" => SecretReference::ProviderLogin,
            other => {
                return Err(SettingsError::Invalid(format!(
                    "profile {name} has unknown secret_source {other}"
                )));
            }
        };
        let profile = ProviderProfile {
            name: validate_profile_name(name)?,
            provider: validate_provider(provider)?,
            model: table
                .get("model")
                .and_then(Item::as_str)
                .map(str::to_string),
            base_url: table
                .get("base_url")
                .and_then(Item::as_str)
                .map(str::to_string),
            secret,
        };
        validate_profile(&profile)?;
        if map.insert(profile.name.clone(), profile).is_some() {
            return Err(SettingsError::Invalid(format!(
                "duplicate profile name {name}"
            )));
        }
    }
    Ok(map)
}

pub fn harnesses_from_document(
    document: &DocumentMut,
) -> Result<BTreeMap<String, HarnessSettings>, SettingsError> {
    let mut map = BTreeMap::new();
    let Some(table) = document.get("harness").and_then(Item::as_table) else {
        return Ok(map);
    };
    for (harness, item) in table.iter() {
        let Some(inner) = item.as_table() else {
            return Err(SettingsError::Invalid(format!(
                "harness.{harness} must be a table"
            )));
        };
        let executable = inner
            .get("executable")
            .and_then(Item::as_str)
            .ok_or_else(|| {
                SettingsError::Invalid(format!("harness.{harness} missing executable"))
            })?;
        let settings = HarnessSettings {
            harness: validate_provider(harness)?,
            executable: validate_executable(executable)?,
            workdir: match inner.get("workdir").and_then(Item::as_str) {
                Some(dir) => Some(validate_workdir(dir)?),
                None => None,
            },
            capture_polling: inner
                .get("capture_polling")
                .and_then(Item::as_bool)
                .unwrap_or(true),
            inject_permission: inner
                .get("inject_permission")
                .and_then(Item::as_bool)
                .unwrap_or(true),
        };
        validate_harness(&settings)?;
        map.insert(settings.harness.clone(), settings);
    }
    Ok(map)
}

pub fn default_profiles_from_table(
    table: &Table,
) -> Result<BTreeMap<String, String>, SettingsError> {
    let mut map = BTreeMap::new();
    let Some(inner) = table
        .get("default_profiles")
        .and_then(Item::as_inline_table)
    else {
        if table.get("default_profiles").is_some() {
            return Err(SettingsError::Invalid(
                "workspace default_profiles must be an inline table".into(),
            ));
        }
        return Ok(map);
    };
    for (harness, item) in inner.iter() {
        let profile = item.as_str().ok_or_else(|| {
            SettingsError::Invalid(format!("default profile for {harness} must be a string"))
        })?;
        let harness = validate_provider(harness)?;
        let profile = validate_profile_name(profile)?;
        map.insert(harness, profile);
    }
    Ok(map)
}

pub fn write_default_profiles(table: &mut Table, defaults: &BTreeMap<String, String>) {
    if defaults.is_empty() {
        table.remove("default_profiles");
        return;
    }
    let mut inline = toml_edit::InlineTable::new();
    for (harness, profile) in defaults {
        inline.insert(harness, value(profile.as_str()).into_value().unwrap());
    }
    table["default_profiles"] = Item::Value(toml_edit::Value::InlineTable(inline));
}

pub fn effective_harnesses(
    harnesses: &BTreeMap<String, HarnessSettings>,
    profiles: &BTreeMap<String, ProviderProfile>,
    workspace_defaults: Option<&BTreeMap<String, String>>,
) -> Vec<EffectiveHarnessSettings> {
    let mut ids: BTreeMap<String, ()> = BTreeMap::new();
    for key in harnesses.keys() {
        ids.insert(key.clone(), ());
    }
    if let Some(defaults) = workspace_defaults {
        for key in defaults.keys() {
            ids.insert(key.clone(), ());
        }
    }
    ids.into_keys()
        .filter_map(|harness| {
            let settings = harnesses
                .get(&harness)
                .cloned()
                .or_else(|| HarnessSettings::default_for(&harness).ok())?;
            let (default_profile, default_profile_status) = match workspace_defaults
                .and_then(|map| map.get(&harness))
                .cloned()
            {
                Some(name) => (Some(name), FieldStatus::Override),
                None => (None, FieldStatus::Inherited),
            };
            let default_profile_badge = default_profile
                .as_ref()
                .and_then(|name| profiles.get(name).map(|profile| profile.secret.badge()));
            Some(EffectiveHarnessSettings {
                harness: settings.harness,
                executable: settings.executable,
                workdir: settings.workdir,
                capture_polling: settings.capture_polling,
                inject_permission: settings.inject_permission,
                default_profile,
                default_profile_status,
                default_profile_badge,
            })
        })
        .collect()
}
