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
    document["orchestration"]["link_suggestion_mode"] = value(orch.link_suggestion_mode.as_str());
    document["orchestration"]["memory_recall_enabled"] = value(orch.memory_recall_enabled);
    document["orchestration"]["memory_recall_limit"] = value(i64::from(orch.memory_recall_limit));
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
        write_default_models(&mut table, &over.default_models);
        write_default_efforts(&mut table, &over.default_efforts);
        write_orchestration_override(&mut table, &over.orchestration);
        array.push(table);
    }
    document["workspace"] = Item::ArrayOfTables(array);
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
        let default_models = default_models_from_table(table)?;
        let default_efforts = default_efforts_from_table(table)?;
        let orchestration = orchestration_override_from_table(table)
            .map_err(|err| SettingsError::Invalid(format!("workspace {path}: {err}")))?;
        if map
            .insert(
                path.clone(),
                WorkspaceOverride {
                    backup_retention,
                    default_session,
                    default_profiles,
                    default_models,
                    default_efforts,
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
