//! Global named provider profiles and per-harness process settings (S4).

use super::model::{
    EffectiveHarnessSettings, FieldStatus, HarnessSettings, ProviderProfile, SecretReference,
    SettingsError,
};
use std::collections::BTreeMap;
use toml_edit::{value, ArrayOfTables, DocumentMut, Item, Table};

pub use super::validation::*;

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
        if let Some(model) = &settings.default_model {
            table["default_model"] = value(model.as_str());
        }
        if let Some(effort) = &settings.default_effort {
            table["default_effort"] = value(effort.as_str());
        }
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
            default_model: inner
                .get("default_model")
                .and_then(Item::as_str)
                .map(str::to_string),
            default_effort: inner
                .get("default_effort")
                .and_then(Item::as_str)
                .map(str::to_string),
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

pub fn default_models_from_table(table: &Table) -> Result<BTreeMap<String, String>, SettingsError> {
    let mut map = BTreeMap::new();
    let Some(inner) = table.get("default_models").and_then(Item::as_inline_table) else {
        if table.get("default_models").is_some() {
            return Err(SettingsError::Invalid(
                "workspace default_models must be an inline table".into(),
            ));
        }
        return Ok(map);
    };
    for (harness, item) in inner.iter() {
        let model = item.as_str().ok_or_else(|| {
            SettingsError::Invalid(format!("default model for {harness} must be a string"))
        })?;
        let harness = validate_provider(harness)?;
        map.insert(harness, model.to_string());
    }
    Ok(map)
}

pub fn write_default_models(table: &mut Table, defaults: &BTreeMap<String, String>) {
    if defaults.is_empty() {
        table.remove("default_models");
        return;
    }
    let mut inline = toml_edit::InlineTable::new();
    for (harness, model) in defaults {
        inline.insert(harness, value(model.as_str()).into_value().unwrap());
    }
    table["default_models"] = Item::Value(toml_edit::Value::InlineTable(inline));
}

pub fn default_efforts_from_table(
    table: &Table,
) -> Result<BTreeMap<String, String>, SettingsError> {
    let mut map = BTreeMap::new();
    let Some(inner) = table.get("default_efforts").and_then(Item::as_inline_table) else {
        if table.get("default_efforts").is_some() {
            return Err(SettingsError::Invalid(
                "workspace default_efforts must be an inline table".into(),
            ));
        }
        return Ok(map);
    };
    for (harness, item) in inner.iter() {
        let effort = item.as_str().ok_or_else(|| {
            SettingsError::Invalid(format!("default effort for {harness} must be a string"))
        })?;
        let harness = validate_provider(harness)?;
        map.insert(harness, effort.to_string());
    }
    Ok(map)
}

pub fn write_default_efforts(table: &mut Table, defaults: &BTreeMap<String, String>) {
    if defaults.is_empty() {
        table.remove("default_efforts");
        return;
    }
    let mut inline = toml_edit::InlineTable::new();
    for (harness, effort) in defaults {
        inline.insert(harness, value(effort.as_str()).into_value().unwrap());
    }
    table["default_efforts"] = Item::Value(toml_edit::Value::InlineTable(inline));
}

pub fn effective_harnesses(
    harnesses: &BTreeMap<String, HarnessSettings>,
    profiles: &BTreeMap<String, ProviderProfile>,
    workspace_override: Option<&super::model::WorkspaceOverride>,
) -> Vec<EffectiveHarnessSettings> {
    let mut ids: BTreeMap<String, ()> = BTreeMap::new();
    for key in harnesses.keys() {
        ids.insert(key.clone(), ());
    }
    if let Some(over) = workspace_override {
        for key in over.default_profiles.keys() {
            ids.insert(key.clone(), ());
        }
        for key in over.default_models.keys() {
            ids.insert(key.clone(), ());
        }
        for key in over.default_efforts.keys() {
            ids.insert(key.clone(), ());
        }
    }
    ids.into_keys()
        .filter_map(|harness| {
            let settings = harnesses
                .get(&harness)
                .cloned()
                .or_else(|| HarnessSettings::default_for(&harness).ok())?;
            let (default_profile, default_profile_status) = match workspace_override
                .and_then(|over| over.default_profiles.get(&harness))
                .cloned()
            {
                Some(name) => (Some(name), FieldStatus::Override),
                None => (None, FieldStatus::Inherited),
            };
            let default_profile_badge = default_profile
                .as_ref()
                .and_then(|name| profiles.get(name).map(|profile| profile.secret.badge()));

            let (selected_model, selected_model_status) = match workspace_override
                .and_then(|over| over.default_models.get(&harness))
                .cloned()
            {
                Some(model) => (Some(model), FieldStatus::Override),
                None => (settings.default_model.clone(), FieldStatus::Inherited),
            };

            let (selected_effort, selected_effort_status) = match workspace_override
                .and_then(|over| over.default_efforts.get(&harness))
                .cloned()
            {
                Some(effort) => (Some(effort), FieldStatus::Override),
                None => (settings.default_effort.clone(), FieldStatus::Inherited),
            };

            Some(EffectiveHarnessSettings {
                harness: settings.harness,
                executable: settings.executable,
                workdir: settings.workdir,
                capture_polling: settings.capture_polling,
                inject_permission: settings.inject_permission,
                default_profile,
                default_profile_status,
                default_profile_badge,
                selected_model,
                selected_model_status,
                selected_effort,
                selected_effort_status,
            })
        })
        .collect()
}
