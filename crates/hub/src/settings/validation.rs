use super::model::{HarnessSettings, ProviderProfile, SecretReference, SettingsError};
use crate::HarnessId;
use std::path::Path;

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

pub fn reject_secret_looking(value: &str, field: &str) -> Result<(), SettingsError> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("sk-")
        || lower.contains("bearer ")
        || lower.contains("api_key=")
        || lower.contains("token=")
        || value.contains('\n')
    {
        return Err(SettingsError::Invalid(format!(
            "{field} contains secret-like content or newline; keep secrets in keychain or env vars"
        )));
    }
    Ok(())
}
