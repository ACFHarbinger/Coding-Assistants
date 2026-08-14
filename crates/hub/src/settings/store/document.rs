use super::*;

pub(super) fn default_document() -> DocumentMut {
    let mut document = DocumentMut::new();
    document.insert("schema_version", value(i64::from(CURRENT_SETTINGS_SCHEMA)));
    let mut storage = Table::new();
    storage["backup_retention"] = value(i64::from(DEFAULT_BACKUP_RETENTION));
    document.insert("storage", Item::Table(storage));
    document
}

pub(super) fn write_snapshot_fields(document: &mut DocumentMut, snapshot: &SettingsSnapshot) {
    document["schema_version"] = value(i64::from(snapshot.schema_version));
    if !document.contains_key("storage") {
        document["storage"] = Item::Table(Table::new());
    }
    document["storage"]["backup_retention"] = value(i64::from(snapshot.backup_retention));

    if let Some(ref default_ws) = snapshot.default_workspace {
        if !document.contains_key("general") {
            document["general"] = Item::Table(Table::new());
        }
        document["general"]["default_workspace"] = value(default_ws.as_str());
    } else if document
        .get("general")
        .and_then(Item::as_table)
        .is_some_and(|t| t.contains_key("default_workspace"))
    {
        document["general"]["default_workspace"] = Item::None;
    }

    if let Some(ref default_sess) = snapshot.default_session {
        if !document.contains_key("general") {
            document["general"] = Item::Table(Table::new());
        }
        document["general"]["default_session"] = value(default_sess.as_str());
    } else if document
        .get("general")
        .and_then(Item::as_table)
        .is_some_and(|t| t.contains_key("default_session"))
    {
        document["general"]["default_session"] = Item::None;
    }

    if !document.contains_key("orchestration") {
        document["orchestration"] = Item::Table(Table::new());
    }
    let orch = &snapshot.orchestration;
    document["orchestration"]["confirm_new_enrollment"] = value(orch.confirm_new_enrollment);
    document["orchestration"]["confirm_broadcast"] = value(orch.confirm_broadcast);
    document["orchestration"]["auto_enrollment_allowed"] = value(orch.auto_enrollment_allowed);
    document["orchestration"]["sandbox_strictness"] = value(orch.sandbox_strictness.as_str());
    document["orchestration"]["export_enabled"] = value(orch.export_enabled);
    document["orchestration"]["link_suggestion_mode"] =
        value(orch.link_suggestion_mode.as_str());
    if let Some(days) = orch.retention_days {
        document["orchestration"]["retention_days"] = value(i64::from(days));
    } else if document
        .get("orchestration")
        .and_then(Item::as_table)
        .is_some_and(|t| t.contains_key("retention_days"))
    {
        document["orchestration"]["retention_days"] = Item::None;
    }

    if !document.contains_key("tui") {
        document["tui"] = Item::Table(Table::new());
    }
    let tui = &snapshot.tui;
    document["tui"]["prefix_chord"] = value(tui.prefix_chord.as_str());
    document["tui"]["unicode_fallback"] = value(tui.unicode_fallback);
    document["tui"]["bell_notification"] = value(tui.bell_notification);
    document["tui"]["high_contrast"] = value(tui.high_contrast);
}

/// Rebuild the `[[workspace]]` array-of-tables from `workspaces` on every
/// save, mirroring how `write_snapshot_fields` unconditionally overwrites
/// `[storage]`. Comments inside a rewritten workspace block do not survive a
/// save that touches it; top-level document comments are unaffected.
pub(super) fn write_workspace_fields(
    document: &mut DocumentMut,
    workspaces: &BTreeMap<String, WorkspaceOverride>,
) {
    if workspaces.is_empty() {
        document.remove("workspace");
        return;
    }
    let mut array = ArrayOfTables::new();
    for (path, over) in workspaces {
        let mut table = Table::new();
        table["path"] = value(path.as_str());
        if let Some(retention) = over.backup_retention {
            table["backup_retention"] = value(i64::from(retention));
        }
        if let Some(ref sess) = over.default_session {
            table["default_session"] = value(sess.as_str());
        }
        write_default_profiles(&mut table, &over.default_profiles);
        write_orchestration_override(&mut table, &over.orchestration);
        array.push(table);
    }
    document["workspace"] = Item::ArrayOfTables(array);
}

/// Merge global orchestration policy with an optional workspace override.
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

pub(super) fn normalize_workspace(workspace: &str) -> Result<String, SettingsError> {
    let trimmed = workspace.trim();
    if trimmed.is_empty() {
        return Err(SettingsError::Invalid(
            "workspace path must not be empty".into(),
        ));
    }
    Ok(trimmed.to_string())
}

#[allow(clippy::type_complexity)]
pub(super) fn parse_document(
    raw: &str,
) -> Result<
    (
        DocumentMut,
        SettingsSnapshot,
        BTreeMap<String, WorkspaceOverride>,
        BTreeMap<String, ProviderProfile>,
        BTreeMap<String, HarnessSettings>,
    ),
    SettingsError,
> {
    let document = raw
        .parse::<DocumentMut>()
        .map_err(|err| SettingsError::Invalid(err.to_string()))?;
    let snapshot = snapshot_from_document(&document)?;
    snapshot.validate()?;
    let workspaces = workspaces_from_document(&document)?;
    for (path, over) in &workspaces {
        over.validate()
            .map_err(|err| SettingsError::Invalid(format!("workspace {path}: {err}")))?;
    }
    let profiles = profiles_from_document(&document)?;
    let harnesses = harnesses_from_document(&document)?;
    Ok((document, snapshot, workspaces, profiles, harnesses))
}

pub(super) fn workspaces_from_document(
    document: &DocumentMut,
) -> Result<BTreeMap<String, WorkspaceOverride>, SettingsError> {
    let mut map = BTreeMap::new();
    let Some(array) = document.get("workspace").and_then(Item::as_array_of_tables) else {
        return Ok(map);
    };
    for table in array.iter() {
        let path = table
            .get("path")
            .and_then(Item::as_str)
            .ok_or_else(|| SettingsError::Invalid("workspace entry missing path".into()))?
            .to_string();
        if path.trim().is_empty() {
            return Err(SettingsError::Invalid(
                "workspace path must not be empty".into(),
            ));
        }
        let backup_retention = match table.get("backup_retention") {
            Some(item) => Some(u32_from_i64(
                item.as_integer().ok_or_else(|| {
                    SettingsError::Invalid(format!(
                        "workspace {path} backup_retention must be an integer"
                    ))
                })?,
                "workspace.backup_retention",
            )?),
            None => None,
        };
        let default_session = table
            .get("default_session")
            .and_then(Item::as_str)
            .map(str::to_string);
        let default_profiles = default_profiles_from_table(table)?;
        let orchestration = orchestration_override_from_table(table)
            .map_err(|err| SettingsError::Invalid(format!("workspace {path}: {err}")))?;
        if map
            .insert(
                path.clone(),
                WorkspaceOverride {
                    backup_retention,
                    default_session,
                    default_profiles,
                    orchestration,
                },
            )
            .is_some()
        {
            return Err(SettingsError::Invalid(format!(
                "duplicate workspace override for {path}"
            )));
        }
    }
    Ok(map)
}

pub(super) fn snapshot_from_document(
    document: &DocumentMut,
) -> Result<SettingsSnapshot, SettingsError> {
    let schema_version = integer_key(document.as_table(), "schema_version")?;
    let storage = document
        .get("storage")
        .and_then(Item::as_table)
        .ok_or_else(|| SettingsError::Invalid("missing [storage] table".into()))?;
    let backup_retention = integer_key(storage, "backup_retention")?;

    let general = document.get("general").and_then(Item::as_table);
    let default_workspace = general
        .and_then(|g| g.get("default_workspace"))
        .and_then(Item::as_str)
        .map(str::to_string);
    let default_session = general
        .and_then(|g| g.get("default_session"))
        .and_then(Item::as_str)
        .map(str::to_string);

    let orchestration = document
        .get("orchestration")
        .and_then(Item::as_table)
        .map(orchestration_policy_from_table)
        .transpose()?
        .unwrap_or_default();

    let tui = document
        .get("tui")
        .and_then(Item::as_table)
        .map(tui_settings_from_table)
        .transpose()?
        .unwrap_or_default();

    Ok(SettingsSnapshot {
        schema_version: u32_from_i64(schema_version, "schema_version")?,
        backup_retention: u32_from_i64(backup_retention, "storage.backup_retention")?,
        default_workspace,
        default_session,
        orchestration,
        tui,
    })
}

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

pub(super) fn replace_file(tmp: &Path, dest: &Path) -> Result<(), SettingsError> {
    match fs::rename(tmp, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            let _ = fs::remove_file(dest);
            fs::rename(tmp, dest).map_err(SettingsError::from)
        }
    }
}

pub(super) fn fsync_dir(path: &Path) -> std::io::Result<()> {
    let file = File::open(path)?;
    file.sync_all()
}
