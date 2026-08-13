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
}
