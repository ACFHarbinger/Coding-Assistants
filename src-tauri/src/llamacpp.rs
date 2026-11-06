//! Launch llama.cpp's `llama-server` in a terminal-grid PTY.
//!
//! The model file is only the `-m` argument. It is not executed and not read
//! into memory. `pty_spawn` receives an explicit argv; there is no shell.

use std::path::{Component, Path, PathBuf};

use tauri::{AppHandle, State};

pub const LLAMA_SERVER_BIN: &str = "llama-server";
pub const LLAMA_SERVER_HOST: &str = "127.0.0.1";

const MODEL_REQUIRED: &str = "A model file path is required.";
const MODEL_NUL: &str = "Model path must not contain NUL.";
const MODEL_ABSOLUTE: &str = "Model path must be absolute and must not contain '..'.";
const MODEL_SYMLINK: &str = "Model path must be a regular file, not a symlink.";
const MODEL_MISSING: &str = "Model file was not found.";
const MODEL_NOT_FILE: &str = "Model path must be a regular file.";
const PORT_RANGE: &str = "Port must be an integer from 1 to 65535.";
const NO_CWD: &str = "No absolute working directory for llama-server.";
pub const LLAMA_SERVER_MISSING: &str =
    "llama-server was not found on PATH. Install llama.cpp's llama-server and try again.";
const ARGV_REJECTED: &str = "llama.cpp launch only runs llama-server with the validated arguments.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaServerPlan {
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub base_url: String,
}

pub fn validate_port(port: i64) -> Result<u16, String> {
    if (1..=65535).contains(&port) {
        Ok(port as u16)
    } else {
        Err(PORT_RANGE.into())
    }
}

/// `--host 127.0.0.1 --port <port> -m <model>`. No other flags.
pub fn llama_server_args(port: u16, model_path: &str) -> Vec<String> {
    vec![
        "--host".to_string(),
        LLAMA_SERVER_HOST.to_string(),
        "--port".to_string(),
        port.to_string(),
        "-m".to_string(),
        model_path.to_string(),
    ]
}

fn validate_model_path(model_path: &str) -> Result<PathBuf, String> {
    if model_path.is_empty() || model_path.chars().all(char::is_whitespace) {
        return Err(MODEL_REQUIRED.into());
    }
    if model_path.contains('\0') {
        return Err(MODEL_NUL.into());
    }
    let path = Path::new(model_path);
    // `..` is refused before lstat: the kernel would resolve it during lookup.
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(MODEL_ABSOLUTE.into());
    }
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(MODEL_MISSING.into());
        }
        Err(_) => return Err(MODEL_NOT_FILE.into()),
    };
    let kind = meta.file_type();
    if kind.is_symlink() {
        return Err(MODEL_SYMLINK.into());
    }
    if !kind.is_file() {
        return Err(MODEL_NOT_FILE.into());
    }
    Ok(path.to_path_buf())
}

fn launch_cwd(workspace: &str, model: &Path) -> Result<PathBuf, String> {
    let workspace_path = Path::new(workspace);
    if workspace_path.is_absolute()
        && std::fs::metadata(workspace_path).is_ok_and(|meta| meta.is_dir())
    {
        return Ok(workspace_path.to_path_buf());
    }
    let Some(parent) = model.parent().filter(|parent| parent.is_absolute()) else {
        return Err(NO_CWD.into());
    };
    if std::fs::metadata(parent).is_ok_and(|meta| meta.is_dir()) {
        return Ok(parent.to_path_buf());
    }
    Err(NO_CWD.into())
}

pub fn plan_llama_server_launch(
    model_path: &str,
    port: i64,
    workspace: &str,
) -> Result<LlamaServerPlan, String> {
    let port = validate_port(port)?;
    let model = validate_model_path(model_path)?;
    let cwd = launch_cwd(workspace, &model)?;
    Ok(LlamaServerPlan {
        args: llama_server_args(port, model_path),
        cwd,
        base_url: format!("http://{LLAMA_SERVER_HOST}:{port}"),
    })
}

fn launchable_binary(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 == 0 {
            return false;
        }
    }
    true
}

/// First absolute `PATH` entry whose file name is exactly `llama-server`.
/// `llama-cli`, `llama-run`, and `llama` are not servers and are ignored.
pub fn resolve_llama_server_on_path(path_var: &str) -> Result<PathBuf, String> {
    for dir in std::env::split_paths(path_var) {
        if !dir.is_absolute() {
            continue;
        }
        let candidate = dir.join(LLAMA_SERVER_BIN);
        if launchable_binary(&candidate) {
            return Ok(candidate);
        }
    }
    Err(LLAMA_SERVER_MISSING.into())
}

pub fn resolve_llama_server() -> Result<PathBuf, String> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    resolve_llama_server_on_path(&path_var)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCppLaunchOutcome {
    pub session_id: String,
    pub base_url: String,
}

/// Spawn `llama-server` for one grid pane. `program` must be the literal
/// name `llama-server`; the process image is the absolute `PATH` match.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn launch_llamacpp_server(
    app: AppHandle,
    state: State<'_, crate::pty::PtySessions>,
    session_id: String,
    model_path: String,
    port: i64,
    workspace: String,
    program: String,
    args: Vec<String>,
    rows: u16,
    cols: u16,
) -> Result<LlamaCppLaunchOutcome, String> {
    if session_id.is_empty() || session_id.contains('\0') {
        return Err("terminal session id is invalid".into());
    }
    let plan = plan_llama_server_launch(&model_path, port, &workspace)?;
    if program != LLAMA_SERVER_BIN || args != plan.args {
        return Err(ARGV_REJECTED.into());
    }
    let resolved = resolve_llama_server()?;
    if resolved.file_name().and_then(|name| name.to_str()) != Some(LLAMA_SERVER_BIN) {
        return Err(LLAMA_SERVER_MISSING.into());
    }
    let program_path = resolved
        .to_str()
        .ok_or("llama-server path is not valid UTF-8.")?
        .to_string();
    let cwd = plan
        .cwd
        .to_str()
        .ok_or("working directory is not valid UTF-8.")?
        .to_string();
    let rows = if rows == 0 { 24 } else { rows };
    let cols = if cols == 0 { 80 } else { cols };
    crate::pty::pty_spawn(
        app,
        state,
        session_id.clone(),
        program_path,
        plan.args,
        cwd,
        rows,
        cols,
    )?;
    Ok(LlamaCppLaunchOutcome {
        session_id,
        base_url: plan.base_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regular_model(dir: &Path, name: &str) -> PathBuf {
        let model = dir.join(name);
        std::fs::File::create(&model).unwrap();
        model
    }

    #[test]
    fn plan_accepts_regular_file_and_port_8080() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let model = regular_model(dir.path(), "model.gguf");
        let model_str = model.to_str().unwrap();
        let plan =
            plan_llama_server_launch(model_str, 8080, other.path().to_str().unwrap()).unwrap();
        assert_eq!(
            plan.args,
            vec![
                "--host".to_string(),
                "127.0.0.1".to_string(),
                "--port".to_string(),
                "8080".to_string(),
                "-m".to_string(),
                model_str.to_string(),
            ]
        );
        assert_eq!(plan.base_url, "http://127.0.0.1:8080");
        assert_eq!(plan.cwd, other.path());
        assert!(plan.cwd.is_dir());
        assert_ne!(plan.cwd, model);
        assert_eq!(std::fs::metadata(&model).unwrap().len(), 0);
    }

    #[test]
    fn cwd_uses_model_parent_when_workspace_is_not_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let model = regular_model(dir.path(), "model.gguf");
        let model_str = model.to_str().unwrap();
        let relative = plan_llama_server_launch(model_str, 8080, "relative").unwrap();
        assert_eq!(relative.cwd, dir.path());
        let missing = dir.path().join("no-such-workspace");
        let fallback =
            plan_llama_server_launch(model_str, 8080, missing.to_str().unwrap()).unwrap();
        assert_eq!(fallback.cwd, dir.path());
        let file_workspace = plan_llama_server_launch(model_str, 8080, model_str).unwrap();
        assert_eq!(file_workspace.cwd, dir.path());
    }

    #[test]
    fn plan_rejects_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_str().unwrap();
        for raw in ["model.gguf", "rel/model.gguf", "./model.gguf"] {
            let err = plan_llama_server_launch(raw, 8080, workspace).unwrap_err();
            assert!(err.contains("absolute"), "{raw}: {err}");
        }
    }

    #[test]
    fn plan_rejects_parent_dir_components() {
        let dir = tempfile::tempdir().unwrap();
        let model = regular_model(dir.path(), "model.gguf");
        let name = dir.path().file_name().unwrap();
        let dotted = dir.path().join("..").join(name).join("model.gguf");
        assert!(
            dotted.exists(),
            "fixture should exist once '..' is resolved"
        );
        assert!(dotted
            .components()
            .any(|component| matches!(component, Component::ParentDir)));
        let err =
            plan_llama_server_launch(dotted.to_str().unwrap(), 8080, dir.path().to_str().unwrap())
                .unwrap_err();
        assert!(err.contains(".."), "{err}");
        assert_eq!(std::fs::metadata(&model).unwrap().len(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn plan_rejects_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let model = regular_model(dir.path(), "model.gguf");
        let link = dir.path().join("link.gguf");
        std::os::unix::fs::symlink(&model, &link).unwrap();
        let err =
            plan_llama_server_launch(link.to_str().unwrap(), 8080, dir.path().to_str().unwrap())
                .unwrap_err();
        assert!(err.to_lowercase().contains("symlink"), "{err}");
    }

    #[test]
    fn plan_rejects_missing_file_and_directory() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_str().unwrap();
        let missing = dir.path().join("missing.gguf");
        let missing_err =
            plan_llama_server_launch(missing.to_str().unwrap(), 8080, workspace).unwrap_err();
        assert!(
            missing_err.to_lowercase().contains("not found"),
            "{missing_err}"
        );
        let dir_err = plan_llama_server_launch(workspace, 8080, workspace).unwrap_err();
        assert!(dir_err.to_lowercase().contains("regular file"), "{dir_err}");
    }

    #[test]
    fn plan_rejects_port_0_and_70000() {
        let dir = tempfile::tempdir().unwrap();
        let model = regular_model(dir.path(), "model.gguf");
        let model_str = model.to_str().unwrap();
        let workspace = dir.path().to_str().unwrap();
        for port in [0_i64, 70_000, -1, 65536] {
            let err = plan_llama_server_launch(model_str, port, workspace).unwrap_err();
            assert!(err.contains("65535"), "port {port}: {err}");
        }
        assert!(plan_llama_server_launch(model_str, 1, workspace).is_ok());
        assert!(plan_llama_server_launch(model_str, 65535, workspace).is_ok());
    }

    #[test]
    fn plan_rejects_nul() {
        let err = plan_llama_server_launch("/tmp/model\0.gguf", 8080, "/tmp").unwrap_err();
        assert!(err.to_lowercase().contains("nul"), "{err}");
    }

    fn write_exec(path: &Path) {
        std::fs::write(path, b"#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).unwrap();
        }
    }

    #[test]
    fn resolve_llama_server_ignores_other_binaries() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["llama-cli", "llama-run", "llama"] {
            write_exec(&dir.path().join(name));
        }
        let server = dir.path().join(LLAMA_SERVER_BIN);
        std::fs::write(&server, b"not-executable\n").unwrap();
        let path = dir.path().to_str().unwrap();
        assert_eq!(
            resolve_llama_server_on_path("relative-dir").unwrap_err(),
            LLAMA_SERVER_MISSING
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&server, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert_eq!(
                resolve_llama_server_on_path(path).unwrap_err(),
                LLAMA_SERVER_MISSING
            );
        }
        write_exec(&server);
        assert_eq!(resolve_llama_server_on_path(path).unwrap(), server);
    }
}
