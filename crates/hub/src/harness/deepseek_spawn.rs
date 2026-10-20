//! Official DeepSeek Harness (`dsh`) argv.
//!
//! DeepSeek is not an OpenCode model. The interactive surface is
//! `dsh --profile web` (a local web app). One-shot task delivery is
//! `dsh --profile headless`, which prints the final answer and exits.
//! Headless accepts a task and an optional `--session-id`; it has no
//! `--model` flag — model choice lives in the web app's own settings
//! (`DEEPSEEK_API_KEY`, default `deepseek-flash`).

use crate::HubError;
use std::ffi::OsString;
use std::path::Path;

/// Catalog default. The web app's composition default is
/// `deepseek-official` / `deepseek-flash` (DeepSeek-V4.1-Flash).
pub const DEFAULT_DEEPSEEK_MODEL: &str = "deepseek-flash";

pub const DSH_EXECUTABLE: &str = "dsh";
pub const DSH_WEB_HOST: &str = "127.0.0.1";

pub fn deepseek_executable() -> &'static str {
    DSH_EXECUTABLE
}

/// `dsh --profile web --no-open --host 127.0.0.1 --port <port>`.
///
/// The working directory is the workspace; this argv does not repeat it.
/// `--no-open` keeps the desktop shell as the only browser: startup prints
/// `dsh web: <tokenized url>` and does not hand that URL to the OS browser.
pub fn deepseek_web_spawn_args(port: u16) -> Vec<OsString> {
    vec![
        OsString::from("--profile"),
        OsString::from("web"),
        OsString::from("--no-open"),
        OsString::from("--host"),
        OsString::from(DSH_WEB_HOST),
        OsString::from("--port"),
        OsString::from(port.to_string()),
    ]
}

/// `dsh --profile headless [--session-id <id>] <task>`.
pub fn deepseek_headless_spawn_args(
    workspace: &Path,
    prompt: &str,
    session_id: Option<&str>,
) -> Result<Vec<OsString>, HubError> {
    if prompt.trim().is_empty() {
        return Err(HubError::Invalid(
            "DeepSeek harness requires a prompt".into(),
        ));
    }
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "DeepSeek harness workspace must be an absolute path".into(),
        ));
    }
    let mut args = vec![OsString::from("--profile"), OsString::from("headless")];
    if let Some(id) = session_id.map(str::trim).filter(|id| !id.is_empty()) {
        args.push(OsString::from("--session-id"));
        args.push(OsString::from(id));
    }
    args.push(OsString::from(prompt));
    Ok(args)
}

/// First `dsh web: http(s)://…` line. The token in that URL is the launch
/// credential the embedded page must open; later lines are ignored.
pub fn parse_dsh_web_url(text: &str) -> Option<String> {
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("dsh web:") else {
            continue;
        };
        let url = rest.split_whitespace().next().unwrap_or("").trim();
        if url.starts_with("http://") || url.starts_with("https://") {
            return Some(url.to_string());
        }
    }
    None
}

/// Port advertised in a `dsh web:` URL. `None` when the URL has no port.
pub fn parse_dsh_web_port(url: &str) -> Option<u16> {
    let authority = url.split_once("://")?.1.split(['/', '?', '#']).next()?;
    let hostport = authority.rsplit('@').next()?;
    let port = if let Some(stripped) = hostport.strip_prefix('[') {
        stripped.split_once(']')?.1.strip_prefix(':')?
    } else {
        hostport.split_once(':')?.1
    };
    port.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::super::HarnessId;
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn web_argv_is_loopback_and_does_not_open_a_browser() {
        let args = deepseek_web_spawn_args(3080);
        assert_eq!(
            args.iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            vec![
                "--profile",
                "web",
                "--no-open",
                "--host",
                "127.0.0.1",
                "--port",
                "3080",
            ]
        );
        assert!(!args
            .iter()
            .any(|arg| arg.to_string_lossy().contains("opencode")));
    }

    #[test]
    fn headless_argv_is_explicit_and_rejects_a_relative_workspace() {
        let ws = PathBuf::from("/tmp/coding-assistants-dsh");
        let args = deepseek_headless_spawn_args(&ws, "review the diff", None).unwrap();
        assert_eq!(args[0], "--profile");
        assert_eq!(args[1], "headless");
        assert_eq!(args[2], "review the diff");
        assert!(!args.iter().any(|arg| arg == "--model" || arg == "opencode"));

        let resumed = deepseek_headless_spawn_args(&ws, "continue", Some("session-1")).unwrap();
        assert_eq!(resumed[2], "--session-id");
        assert_eq!(resumed[3], "session-1");
        assert_eq!(resumed[4], "continue");

        assert!(deepseek_headless_spawn_args(Path::new("relative"), "x", None).is_err());
        assert!(deepseek_headless_spawn_args(&ws, "   ", None).is_err());
        assert_eq!(HarnessId::DeepSeek.executable(), "dsh");
        assert_ne!(HarnessId::DeepSeek.executable(), "opencode");
    }

    #[test]
    fn web_url_parser_keeps_the_tokenized_launch_url_only() {
        let log = "\
booting\n\
dsh web: opening the default browser; pass --no-open to disable\n\
dsh web: http://127.0.0.1:3080/?token=abc\n\
later noise\n";
        assert_eq!(
            parse_dsh_web_url(log).as_deref(),
            Some("http://127.0.0.1:3080/?token=abc")
        );
        assert_eq!(parse_dsh_web_url("dsh web: not-a-url"), None);
        assert_eq!(parse_dsh_web_url(""), None);
        assert_eq!(
            parse_dsh_web_port("http://127.0.0.1:3080/?token=abc"),
            Some(3080)
        );
        assert_eq!(parse_dsh_web_port("https://[::1]:8443/"), Some(8443));
        assert_eq!(parse_dsh_web_port("http://127.0.0.1/"), None);
    }
}
