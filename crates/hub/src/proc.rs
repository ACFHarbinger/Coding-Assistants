//! Cross-platform local process listing for discovery-only inspection.
//!
//! Every caller here reads the process table to *classify* what is already
//! running — it never touches a process's stdin/stdout and never changes
//! ownership. Keep it that way.
//!
//! The list is produced by a platform tool rather than a dependency:
//!
//! * Unix — `ps -eo pid=,args=`.
//! * Windows — Windows PowerShell (`powershell.exe`, in-box on every
//!   supported Windows 10/11). `ps` does not exist there, and `wmic` was
//!   removed from Windows 11 24H2, so neither is a portable option. The
//!   query script is written with `[char]9` instead of quote characters so
//!   it survives the `CreateProcess` argument round-trip without any
//!   escaping.

use std::process::Command;

/// The PowerShell script that emits one `"<pid>\t<command line>"` line per
/// process. `[char]9` is a literal tab; using it (rather than a quoted
/// `"..."` string) keeps the whole script free of quote characters, so the
/// standard library's Windows argument quoting wraps it in exactly one pair
/// of outer quotes and PowerShell parses it verbatim. A protected/kernel
/// process with no readable `CommandLine` yields an empty tail, which the
/// parser drops.
#[cfg(windows)]
const WINDOWS_PROCESS_SCRIPT: &str = "Get-CimInstance Win32_Process | ForEach-Object { [string]$_.ProcessId + [char]9 + [string]$_.CommandLine }";

/// Local processes as `(pid, command_line)` pairs. `command_line` is the
/// full argv joined by spaces on Unix and the raw `Win32_Process.CommandLine`
/// on Windows. Rows whose pid does not parse, or which carry no command
/// text, are skipped.
pub fn list_process_lines() -> Result<Vec<(u32, String)>, String> {
    let table = raw_process_table()?;
    Ok(table.lines().filter_map(parse_process_row).collect())
}

/// Raw stdout of the platform process-listing command.
fn raw_process_table() -> Result<String, String> {
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            WINDOWS_PROCESS_SCRIPT,
        ]);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new("ps");
        command.args(["-eo", "pid=,args="]);
        command
    };

    let output = command
        .output()
        .map_err(|error| format!("failed to inspect local processes: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if detail.is_empty() {
            return Err(
                "failed to inspect local processes: listing command exited non-zero".into(),
            );
        }
        return Err(detail);
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Split one listing row into `(pid, command_line)`.
///
/// This handles both shapes with a single rule — split on the first run of
/// whitespace after the pid:
///
/// * `ps -eo pid=` right-justifies the pid in a padded column
///   (`"  1234 /usr/bin/claude …"`).
/// * The PowerShell script emits `"1234\t/usr/bin/claude …"`.
fn parse_process_row(line: &str) -> Option<(u32, String)> {
    let (pid, rest) = line.trim_start().split_once(char::is_whitespace)?;
    let pid = pid.trim().parse::<u32>().ok()?;
    let command = rest.trim();
    if command.is_empty() {
        return None;
    }
    Some((pid, command.to_string()))
}

#[cfg(test)]
mod tests {
    use super::parse_process_row;

    #[test]
    fn parses_a_space_padded_ps_row() {
        assert_eq!(
            parse_process_row("  1234 /usr/local/bin/claude --continue"),
            Some((1234, "/usr/local/bin/claude --continue".to_string()))
        );
    }

    #[test]
    fn parses_a_tab_delimited_powershell_row() {
        assert_eq!(
            parse_process_row("4321\tC:\\Program Files\\codex\\codex.exe --flag"),
            Some((
                4321,
                "C:\\Program Files\\codex\\codex.exe --flag".to_string()
            ))
        );
    }

    #[test]
    fn drops_rows_with_no_command_text() {
        // A Windows protected process: pid, tab, empty CommandLine.
        assert_eq!(parse_process_row("4\t"), None);
        assert_eq!(parse_process_row("4"), None);
    }

    #[test]
    fn drops_rows_with_a_non_numeric_pid() {
        assert_eq!(parse_process_row("PID   COMMAND"), None);
        assert_eq!(parse_process_row(""), None);
    }
}
