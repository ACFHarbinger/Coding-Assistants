//! OpenCode (DeepSeek) and Mistral Vibe argv, model listing, and availability.
//!
//! These helpers never read secret files and never concatenate a shell string.
//! Callers pass the returned argv to `Command::args`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Public Vibe aliases from the installed CLI's default model table
/// (`mistral-medium-3.5`, `devstral-small`, `local`). Not API keys.
pub const VIBE_FALLBACK_MODELS: &[&str] = &["mistral-medium-3.5", "devstral-small", "local"];

const VIBE_HELP_FLAGS: &[&str] = &["-p", "--workdir", "--output", "--trust"];

pub fn opencode_model_spec(provider: &str, model: &str) -> String {
    format!("{}/{}", provider.trim(), model.trim())
}

/// `opencode run <prompt> -m provider/model [--dir <abs>]`
pub fn opencode_run_args(
    provider: &str,
    model: &str,
    prompt: &str,
    work_dir: Option<&str>,
) -> Result<Vec<OsString>, String> {
    if prompt.trim().is_empty() {
        return Err("OpenCode run requires a prompt".into());
    }
    if provider.trim().is_empty() || model.trim().is_empty() {
        return Err("OpenCode run requires provider/model".into());
    }
    let mut args = vec![
        OsString::from("run"),
        OsString::from(prompt),
        OsString::from("-m"),
        OsString::from(opencode_model_spec(provider, model)),
    ];
    if let Some(dir) = work_dir.map(str::trim).filter(|value| !value.is_empty()) {
        if !Path::new(dir).is_absolute() {
            return Err("OpenCode --dir must be an absolute path".into());
        }
        args.push(OsString::from("--dir"));
        args.push(OsString::from(dir));
    }
    Ok(args)
}

/// Lines from `opencode models`, preserving provider/model form.
pub fn parse_opencode_models(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
fn deepseek_models_from_opencode(output: &str) -> Vec<String> {
    parse_opencode_models(output)
        .into_iter()
        .filter_map(|line| {
            line.strip_prefix("deepseek/")
                .map(str::to_string)
                .filter(|model| !model.is_empty())
        })
        .collect()
}

pub fn vibe_programmatic_supported(help: &str) -> bool {
    VIBE_HELP_FLAGS
        .iter()
        .all(|flag| help.split_whitespace().any(|token| token == *flag) || help.contains(flag))
}

/// Auth presence only: non-empty `MISTRAL_API_KEY`, or a `~/.vibe/.env` file.
/// Does not read file contents or the keyring. `local` (llama.cpp) needs no key.
pub fn vibe_is_authenticated(model: &str, vibe_home: &Path, mistral_api_key: Option<&str>) -> bool {
    if model.trim() == "local" {
        return true;
    }
    if mistral_api_key
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return true;
    }
    vibe_home.join(".env").is_file()
}

pub fn vibe_home_from_env(vibe_home: Option<&str>, user_home: Option<&str>) -> PathBuf {
    if let Some(dir) = vibe_home.map(str::trim).filter(|value| !value.is_empty()) {
        return PathBuf::from(dir);
    }
    PathBuf::from(user_home.unwrap_or(".")).join(".vibe")
}

/// `vibe -p <prompt> --workdir <abs> --trust --output text --auto-approve`
pub fn vibe_run_args(prompt: &str, work_dir: Option<&str>) -> Result<Vec<OsString>, String> {
    if prompt.trim().is_empty() {
        return Err("Mistral Vibe run requires a prompt".into());
    }
    let mut args = vec![
        OsString::from("-p"),
        OsString::from(prompt),
        OsString::from("--trust"),
        OsString::from("--output"),
        OsString::from("text"),
        OsString::from("--auto-approve"),
    ];
    if let Some(dir) = work_dir.map(str::trim).filter(|value| !value.is_empty()) {
        if !Path::new(dir).is_absolute() {
            return Err("Mistral Vibe --workdir must be an absolute path".into());
        }
        args.push(OsString::from("--workdir"));
        args.push(OsString::from(dir));
    }
    Ok(args)
}

pub fn vibe_unavailable_not_installed(error: impl std::fmt::Display) -> String {
    format!(
        "Mistral (vibe) unavailable: vibe CLI is not installed or not on PATH ({error}). Install Mistral Vibe and retry."
    )
}

pub fn vibe_unavailable_unsupported() -> String {
    "Mistral (vibe) unavailable: this vibe build does not advertise programmatic -p/--workdir/--output/--trust. Upgrade vibe and retry.".into()
}

pub fn vibe_unavailable_unauthenticated() -> String {
    "Mistral (vibe) unavailable: not authenticated. Run `vibe --setup` to configure an API key (no key is stored by Coding Assistants).".into()
}

pub fn deepseek_unavailable_opencode(error: impl std::fmt::Display) -> String {
    format!(
        "DeepSeek (OpenCode) unavailable: opencode CLI is not installed or failed to start ({error}). Install OpenCode, confirm `opencode models` lists deepseek/*, and retry."
    )
}

/// Meta Muse provider key used in `ModelConfig.provider`. The CLI's own
/// provider id is `meta` (`muse auth set --provider meta`, `META_API_KEY`
/// priority over `muse login`) — both keys route here; `ModelConfig` is a
/// separate namespace from `HarnessId`, which deliberately rejects bare
/// "meta" as ambiguous between the harness (#273) and this provider (#274).
pub fn is_muse_provider(provider: &str) -> bool {
    matches!(provider.trim(), "muse" | "meta")
}

/// Auth presence only: a non-empty `META_API_KEY`, or a `muse auth.json`
/// file (created by `muse login`) existing under the config dir. Never reads
/// file contents or the keyring — presence gates a clear "not
/// authenticated" error before any spawn; the CLI itself owns the secret.
pub fn muse_is_authenticated(meta_api_key: Option<&str>, muse_config_dir: &Path) -> bool {
    if meta_api_key
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return true;
    }
    muse_config_dir.join("auth.json").is_file()
}

pub fn muse_config_dir_from_env(xdg_config_home: Option<&str>, user_home: Option<&str>) -> PathBuf {
    if let Some(dir) = xdg_config_home
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(dir).join("muse");
    }
    PathBuf::from(user_home.unwrap_or("."))
        .join(".config")
        .join("muse")
}

/// `muse/<id>` catalog entries for `get_available_models`.
///
/// #274 spike outcome: the `muse` CLI publishes no model catalog (no
/// `models` subcommand; `--model` takes an undocumented id and the server
/// otherwise picks its default), so an authenticated install contributes no
/// entries. This probe keeps that decision re-verifiable: it confirms the
/// CLI is present and authenticated, and if a future CLI gains a catalog,
/// its entries land here. Configured models work regardless — the llm.rs
/// completion passes `--model` through verbatim.
pub async fn muse_catalog_entries() -> Vec<String> {
    let version = tokio::process::Command::new("muse")
        .arg("--version")
        .output()
        .await;
    let Ok(version) = version else {
        return Vec::new();
    };
    if !version.status.success() {
        return Vec::new();
    }
    let config_dir = muse_config_dir_from_env(
        std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    );
    let api_key = std::env::var("META_API_KEY").ok();
    if !muse_is_authenticated(api_key.as_deref(), &config_dir) {
        return Vec::new();
    }
    Vec::new()
}

/// `muse exec [--model <id>] [--reasoning-effort <effort>] [--workspace <abs>] <prompt>`
/// (verified live against `muse exec --help`, muse 1.0.3, #274 spike).
/// Model/effort are opaque passthroughs — the CLI publishes no model
/// catalog, so no id is defaulted or validated here. No approval-bypass
/// flags are passed by default.
pub fn muse_run_args(
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
    work_dir: Option<&str>,
) -> Result<Vec<OsString>, String> {
    if prompt.trim().is_empty() {
        return Err("Muse run requires a prompt".into());
    }
    let mut args = vec![OsString::from("exec")];
    if let Some(model) = model.map(str::trim).filter(|value| !value.is_empty()) {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }
    if let Some(effort) = effort.map(str::trim).filter(|value| !value.is_empty()) {
        args.push(OsString::from("--reasoning-effort"));
        args.push(OsString::from(effort));
    }
    if let Some(dir) = work_dir.map(str::trim).filter(|value| !value.is_empty()) {
        if !Path::new(dir).is_absolute() {
            return Err("Muse --workspace must be an absolute path".into());
        }
        args.push(OsString::from("--workspace"));
        args.push(OsString::from(dir));
    }
    args.push(OsString::from(prompt));
    Ok(args)
}

pub fn muse_unavailable_not_installed(error: impl std::fmt::Display) -> String {
    format!(
        "Muse (meta) unavailable: muse CLI is not installed or failed to start ({error}). Install Muse Code and retry."
    )
}

pub fn muse_unavailable_unauthenticated() -> String {
    "Muse (meta) unavailable: not authenticated. Run `muse login` or set META_API_KEY (Coding Assistants never stores the key).".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn opencode_argv_is_explicit_and_rejects_relative_dir() {
        let args = opencode_run_args(
            "deepseek",
            "deepseek-chat",
            "review this",
            Some("/tmp/workspace"),
        )
        .unwrap();
        assert_eq!(args[0], "run");
        assert_eq!(args[1], "review this");
        assert_eq!(args[2], "-m");
        assert_eq!(args[3], "deepseek/deepseek-chat");
        assert_eq!(args[4], "--dir");
        assert_eq!(args[5], "/tmp/workspace");
        assert!(opencode_run_args("deepseek", "deepseek-chat", "x", Some("rel")).is_err());
        assert!(opencode_run_args("deepseek", "deepseek-chat", "  ", None).is_err());
    }

    #[test]
    fn deepseek_models_come_from_opencode_listing_not_hardcoded_secrets() {
        let listing = "\
openai/gpt-4.1
deepseek/deepseek-chat
deepseek/deepseek-reasoner
opencode/deepseek-v4-flash-free
deepseek/deepseek-v4-flash
";
        assert_eq!(
            deepseek_models_from_opencode(listing),
            vec!["deepseek-chat", "deepseek-reasoner", "deepseek-v4-flash",]
        );
    }

    #[test]
    fn vibe_help_requires_programmatic_flags() {
        let help = "\
usage: vibe [-h] [-p [TEXT]] [--output {text,json,streaming}]
            [--auto-approve] [--workdir DIR] [--trust]
";
        assert!(vibe_programmatic_supported(help));
        assert!(!vibe_programmatic_supported("usage: vibe [--setup]"));
    }

    #[test]
    fn vibe_argv_is_explicit_and_rejects_relative_workdir() {
        let args = vibe_run_args("summarize", Some("/tmp/workspace")).unwrap();
        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "summarize");
        assert!(args.iter().any(|arg| arg == "--trust"));
        assert!(args.iter().any(|arg| arg == "--auto-approve"));
        let output_at = args.iter().position(|arg| arg == "--output").unwrap();
        assert_eq!(args[output_at + 1], "text");
        let workdir_at = args.iter().position(|arg| arg == "--workdir").unwrap();
        assert_eq!(args[workdir_at + 1], "/tmp/workspace");
        assert!(vibe_run_args("x", Some("relative")).is_err());
        assert!(vibe_run_args("   ", None).is_err());
    }

    #[test]
    fn vibe_auth_uses_presence_only_and_skips_local() {
        let dir = tempdir().unwrap();
        assert!(vibe_is_authenticated("local", dir.path(), None));
        assert!(!vibe_is_authenticated(
            "mistral-medium-3.5",
            dir.path(),
            None
        ));
        assert!(vibe_is_authenticated(
            "mistral-medium-3.5",
            dir.path(),
            Some("not-a-secret-flag")
        ));
        fs::write(dir.path().join(".env"), "MISTRAL_API_KEY=do-not-read\n").unwrap();
        assert!(vibe_is_authenticated(
            "mistral-medium-3.5",
            dir.path(),
            None
        ));
        assert_eq!(
            vibe_home_from_env(Some("/custom/vibe"), Some("/home/user")),
            PathBuf::from("/custom/vibe")
        );
        assert_eq!(
            vibe_home_from_env(None, Some("/home/user")),
            PathBuf::from("/home/user/.vibe")
        );
    }

    #[test]
    fn vibe_argv_keeps_shell_metacharacters_as_one_argument() {
        let dangerous = "; rm -rf / && echo pwned $(whoami)";
        let args = vibe_run_args(dangerous, Some("/tmp/ws")).unwrap();
        assert_eq!(args.iter().filter(|arg| *arg == dangerous).count(), 1);
    }

    #[test]
    fn muse_provider_keys_cover_app_and_cli_ids() {
        assert!(is_muse_provider("muse"));
        assert!(is_muse_provider("meta"));
        assert!(is_muse_provider(" muse "));
        assert!(!is_muse_provider("opencode"));
        assert!(!is_muse_provider(""));
    }

    #[test]
    fn muse_auth_is_presence_only_and_never_reads_secrets() {
        let dir = tempdir().unwrap();
        // No key, no login file: unauthenticated.
        assert!(!muse_is_authenticated(None, dir.path()));
        assert!(!muse_is_authenticated(Some("  "), dir.path()));
        // Env key presence authenticates without touching the fs.
        assert!(muse_is_authenticated(
            Some("presence-flag-not-a-secret"),
            dir.path()
        ));
        // A login file's EXISTENCE authenticates; contents are never read.
        fs::write(dir.path().join("auth.json"), "{}").unwrap();
        assert!(muse_is_authenticated(None, dir.path()));
        assert_eq!(
            muse_config_dir_from_env(Some("/custom/config"), Some("/home/user")),
            PathBuf::from("/custom/config/muse")
        );
        assert_eq!(
            muse_config_dir_from_env(None, Some("/home/user")),
            PathBuf::from("/home/user/.config/muse")
        );
    }

    #[test]
    fn muse_argv_is_explicit_and_rejects_relative_workdir() {
        let args = muse_run_args("summarize", None, None, Some("/tmp/workspace")).unwrap();
        assert_eq!(args[0], "exec");
        assert_eq!(args[args.len() - 3], "--workspace");
        assert_eq!(args[args.len() - 2], "/tmp/workspace");
        assert_eq!(args[args.len() - 1], "summarize");

        let custom = muse_run_args("summarize", Some("test-model"), Some("low"), None).unwrap();
        assert_eq!(custom[0], "exec");
        assert_eq!(custom[1], "--model");
        assert_eq!(custom[2], "test-model");
        assert_eq!(custom[3], "--reasoning-effort");
        assert_eq!(custom[4], "low");
        assert_eq!(custom[5], "summarize");

        assert!(muse_run_args("x", None, None, Some("relative")).is_err());
        assert!(muse_run_args("   ", None, None, None).is_err());
        // No approval-bypass flags ride along by default.
        for args in [&args, &custom] {
            assert!(!args
                .iter()
                .any(|arg| arg == "--yolo" || arg == "--disable-approval"));
        }
    }
}
