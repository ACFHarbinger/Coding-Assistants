use super::*;
use toml_edit::{value, Item, Table};

pub(super) fn tui_settings_from_table(table: &Table) -> Result<TuiSettings, SettingsError> {
    let defaults = TuiSettings::default();
    let prefix_chord = match table.get("prefix_chord").and_then(Item::as_str) {
        Some(s) => s.to_string(),
        None => defaults.prefix_chord,
    };
    let unicode_fallback = bool_key_or(table, "unicode_fallback", defaults.unicode_fallback)?;
    let bell_notification = bool_key_or(table, "bell_notification", defaults.bell_notification)?;
    let high_contrast = bool_key_or(table, "high_contrast", defaults.high_contrast)?;
    Ok(TuiSettings {
        prefix_chord,
        unicode_fallback,
        bell_notification,
        high_contrast,
    })
}

pub(super) fn orchestration_policy_from_table(
    table: &Table,
) -> Result<OrchestrationPolicy, SettingsError> {
    let defaults = OrchestrationPolicy::default();
    let confirm_new_enrollment = bool_key_or(
        table,
        "confirm_new_enrollment",
        defaults.confirm_new_enrollment,
    )?;
    let confirm_broadcast = bool_key_or(table, "confirm_broadcast", defaults.confirm_broadcast)?;
    let auto_enrollment_allowed = bool_key_or(
        table,
        "auto_enrollment_allowed",
        defaults.auto_enrollment_allowed,
    )?;
    let export_enabled = bool_key_or(table, "export_enabled", defaults.export_enabled)?;
    let sandbox_strictness = match table.get("sandbox_strictness").and_then(Item::as_str) {
        Some(raw) => SandboxStrictness::parse(raw).ok_or_else(|| {
            SettingsError::Invalid(format!(
                "orchestration.sandbox_strictness {raw:?} is unknown"
            ))
        })?,
        None => defaults.sandbox_strictness,
    };
    let retention_days = match table.get("retention_days") {
        Some(item) => Some(u32_from_i64(
            item.as_integer().ok_or_else(|| {
                SettingsError::Invalid("orchestration.retention_days must be an integer".into())
            })?,
            "orchestration.retention_days",
        )?),
        None => None,
    };
    let link_suggestion_mode = match table.get("link_suggestion_mode").and_then(Item::as_str) {
        Some(raw) => LinkSuggestionMode::parse(raw).ok_or_else(|| {
            SettingsError::Invalid(format!(
                "orchestration.link_suggestion_mode {raw:?} is unknown"
            ))
        })?,
        None => defaults.link_suggestion_mode,
    };
    Ok(OrchestrationPolicy {
        confirm_new_enrollment,
        confirm_broadcast,
        auto_enrollment_allowed,
        sandbox_strictness,
        retention_days,
        export_enabled,
        link_suggestion_mode,
    })
}

pub(super) fn effective_orchestration(
    global: &OrchestrationPolicy,
    over: Option<&OrchestrationOverride>,
) -> EffectiveOrchestrationPolicy {
    fn merge<T: Copy>(global: T, over: Option<T>) -> (T, FieldStatus) {
        match over {
            Some(value) => (value, FieldStatus::Override),
            None => (global, FieldStatus::Inherited),
        }
    }
    let (confirm_new_enrollment, confirm_new_enrollment_status) = merge(
        global.confirm_new_enrollment,
        over.and_then(|o| o.confirm_new_enrollment),
    );
    let (confirm_broadcast, confirm_broadcast_status) = merge(
        global.confirm_broadcast,
        over.and_then(|o| o.confirm_broadcast),
    );
    let (auto_enrollment_allowed, auto_enrollment_allowed_status) = merge(
        global.auto_enrollment_allowed,
        over.and_then(|o| o.auto_enrollment_allowed),
    );
    let (sandbox_strictness, sandbox_strictness_status) = merge(
        global.sandbox_strictness,
        over.and_then(|o| o.sandbox_strictness),
    );
    let (retention_days, retention_days_status) = match over.and_then(|o| o.retention_days) {
        Some(value) => (Some(value), FieldStatus::Override),
        None => (global.retention_days, FieldStatus::Inherited),
    };
    let (export_enabled, export_enabled_status) =
        merge(global.export_enabled, over.and_then(|o| o.export_enabled));
    let (link_suggestion_mode, link_suggestion_mode_status) = merge(
        global.link_suggestion_mode,
        over.and_then(|o| o.link_suggestion_mode),
    );
    EffectiveOrchestrationPolicy {
        confirm_new_enrollment,
        confirm_new_enrollment_status,
        confirm_broadcast,
        confirm_broadcast_status,
        auto_enrollment_allowed,
        auto_enrollment_allowed_status,
        sandbox_strictness,
        sandbox_strictness_status,
        retention_days,
        retention_days_status,
        export_enabled,
        export_enabled_status,
        link_suggestion_mode,
        link_suggestion_mode_status,
    }
}

pub(super) fn write_orchestration_override(table: &mut Table, over: &OrchestrationOverride) {
    if over.is_empty() {
        table.remove("orchestration");
        return;
    }
    let mut inline = toml_edit::InlineTable::new();
    if let Some(v) = over.confirm_new_enrollment {
        inline.insert("confirm_new_enrollment", value(v).into_value().unwrap());
    }
    if let Some(v) = over.confirm_broadcast {
        inline.insert("confirm_broadcast", value(v).into_value().unwrap());
    }
    if let Some(v) = over.auto_enrollment_allowed {
        inline.insert("auto_enrollment_allowed", value(v).into_value().unwrap());
    }
    if let Some(v) = over.sandbox_strictness {
        inline.insert(
            "sandbox_strictness",
            value(v.as_str()).into_value().unwrap(),
        );
    }
    if let Some(v) = over.retention_days {
        inline.insert("retention_days", value(i64::from(v)).into_value().unwrap());
    }
    if let Some(v) = over.export_enabled {
        inline.insert("export_enabled", value(v).into_value().unwrap());
    }
    if let Some(v) = over.link_suggestion_mode {
        inline.insert(
            "link_suggestion_mode",
            value(v.as_str()).into_value().unwrap(),
        );
    }
    table["orchestration"] = Item::Value(toml_edit::Value::InlineTable(inline));
}

pub(super) fn orchestration_override_from_table(
    table: &Table,
) -> Result<OrchestrationOverride, SettingsError> {
    let Some(inner) = table.get("orchestration").and_then(Item::as_inline_table) else {
        if table.get("orchestration").is_some() {
            return Err(SettingsError::Invalid(
                "orchestration override must be an inline table".into(),
            ));
        }
        return Ok(OrchestrationOverride::default());
    };
    let confirm_new_enrollment = inner
        .get("confirm_new_enrollment")
        .and_then(|v| v.as_bool());
    let confirm_broadcast = inner.get("confirm_broadcast").and_then(|v| v.as_bool());
    let auto_enrollment_allowed = inner
        .get("auto_enrollment_allowed")
        .and_then(|v| v.as_bool());
    let sandbox_strictness = match inner.get("sandbox_strictness").and_then(|v| v.as_str()) {
        Some(raw) => Some(SandboxStrictness::parse(raw).ok_or_else(|| {
            SettingsError::Invalid(format!(
                "orchestration.sandbox_strictness {raw:?} is unknown"
            ))
        })?),
        None => None,
    };
    let retention_days = match inner.get("retention_days") {
        Some(v) => Some(u32_from_i64(
            v.as_integer().ok_or_else(|| {
                SettingsError::Invalid("orchestration.retention_days must be an integer".into())
            })?,
            "orchestration.retention_days",
        )?),
        None => None,
    };
    let export_enabled = inner.get("export_enabled").and_then(|v| v.as_bool());
    let link_suggestion_mode = match inner.get("link_suggestion_mode").and_then(|v| v.as_str()) {
        Some(raw) => Some(LinkSuggestionMode::parse(raw).ok_or_else(|| {
            SettingsError::Invalid(format!(
                "orchestration.link_suggestion_mode {raw:?} is unknown"
            ))
        })?),
        None => None,
    };
    Ok(OrchestrationOverride {
        confirm_new_enrollment,
        confirm_broadcast,
        auto_enrollment_allowed,
        sandbox_strictness,
        retention_days,
        export_enabled,
        link_suggestion_mode,
    })
}

pub(super) fn bool_key_or(table: &Table, key: &str, fallback: bool) -> Result<bool, SettingsError> {
    match table.get(key) {
        Some(item) => item.as_bool().ok_or_else(|| {
            SettingsError::Invalid(format!("orchestration.{key} must be a boolean"))
        }),
        None => Ok(fallback),
    }
}

pub(super) fn integer_key(table: &Table, key: &str) -> Result<i64, SettingsError> {
    table
        .get(key)
        .and_then(Item::as_integer)
        .ok_or_else(|| SettingsError::Invalid(format!("missing or non-integer {key}")))
}

pub(super) fn u32_from_i64(value: i64, key: &str) -> Result<u32, SettingsError> {
    u32::try_from(value)
        .map_err(|_| SettingsError::Invalid(format!("{key} {value} is out of range")))
}
