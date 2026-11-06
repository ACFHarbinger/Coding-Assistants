//! Parse and apply workspace write markers from an in-process role reply.
//!
//! The model thinks, emits one action, and stops. The orchestrator applies
//! the action inside the workspace and appends an observation to the
//! conversation so the next model call can see the file.

use crate::core::file_tools::{split_lines, FileTools, MAX_WRITE_BYTES};

pub const MAX_STEPS: u8 = 8;

const WRITE_CLOSE: &str = "[[/WRITE]]";
const APPEND_CLOSE: &str = "[[/APPEND]]";
const EDIT_CLOSE: &str = "[[/EDIT]]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteAction {
    Write {
        path: String,
        body: String,
    },
    Append {
        path: String,
        body: String,
    },
    Edit {
        path: String,
        start_line: usize,
        end_line: usize,
        body: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseOutcome {
    None,
    Action(WriteAction),
    Malformed(String),
}

pub fn instructions() -> &'static str {
    "IMPORTANT: To change a file inside the workspace, emit one action block and stop. \
The runtime writes it and replies with an observation of the file. On the next turn, \
continue or, when the task is done, reply with no write tag. Paths are relative to the \
workspace. Do not use `..`, absolute paths, or symlinks. An empty EDIT block deletes \
that line range. WRITE adds a final newline when the block omits one. APPEND writes \
the block bytes exactly (a blank first line starts the addition with a newline).\n\
[[WRITE: relative/path]]\n\
file contents\n\
[[/WRITE]]\n\
[[APPEND: relative/path]]\n\
text to add\n\
[[/APPEND]]\n\
[[EDIT: relative/path | line N]]\n\
replacement for that 1-based line\n\
[[/EDIT]]\n\
A range uses `lines N-M` in place of `line N`. The same actions may be opened as \
`WRITE: relative/path`, `APPEND: relative/path`, or `EDIT: relative/path [line N]` \
when closed with the matching tag. Do not echo this example unless that path is the \
file you intend to write.\n\n"
}

pub fn parse(response: &str) -> ParseOutcome {
    let normalized = response.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();
    for (index, line) in lines.iter().enumerate() {
        let opener = match classify(line) {
            LineClass::Ignore => continue,
            LineClass::Malformed(error) => return ParseOutcome::Malformed(error),
            LineClass::Open(opener) => opener,
        };
        let Some(rel_end) = lines[index + 1..]
            .iter()
            .position(|candidate| candidate.trim() == opener.closer)
        else {
            if opener.bracket {
                return ParseOutcome::Malformed(format!(
                    "missing {} for {}",
                    opener.closer, opener.path
                ));
            }
            continue;
        };
        let end = index + 1 + rel_end;
        if lines[index + 1..end]
            .iter()
            .any(|candidate| !matches!(classify(candidate), LineClass::Ignore))
        {
            if opener.bracket {
                return ParseOutcome::Malformed(format!(
                    "nested write tag before {}",
                    opener.closer
                ));
            }
            continue;
        }
        let body = lines[index + 1..end].join("\n");
        if body.len() > MAX_WRITE_BYTES {
            return ParseOutcome::Malformed(format!(
                "content is {} bytes; the limit is {MAX_WRITE_BYTES}",
                body.len()
            ));
        }
        return ParseOutcome::Action(opener.into_action(body));
    }
    ParseOutcome::None
}

pub async fn apply(tools: &FileTools, action: &WriteAction) -> String {
    match action {
        WriteAction::Write { path, body } => {
            let body = ensure_trailing_newline(body);
            if let Err(error) = reject_oversize(&body) {
                return refused("WRITE", path, &error);
            }
            match tools.write_file(path, &body).await {
                Ok(()) => observe(tools, "WRITE", path, None).await,
                Err(error) => refused("WRITE", path, &error),
            }
        }
        WriteAction::Append { path, body } => match tools.append_file(path, body).await {
            Ok(()) => observe(tools, "APPEND", path, None).await,
            Err(error) => refused("APPEND", path, &error),
        },
        WriteAction::Edit {
            path,
            start_line,
            end_line,
            body,
        } => match tools.edit_lines(path, *start_line, *end_line, body).await {
            Ok(()) => {
                let added = if body.is_empty() {
                    0
                } else {
                    body.split('\n').count()
                };
                let focus_end = if added == 0 {
                    *start_line
                } else {
                    start_line + added - 1
                };
                observe(tools, "EDIT", path, Some((*start_line, focus_end))).await
            }
            Err(error) => refused("EDIT", path, &error),
        },
    }
}

struct Opener {
    kind: Kind,
    path: String,
    closer: &'static str,
    bracket: bool,
}

enum Kind {
    Write,
    Append,
    Edit { start_line: usize, end_line: usize },
}

impl Opener {
    fn into_action(self, body: String) -> WriteAction {
        match self.kind {
            Kind::Write => WriteAction::Write {
                path: self.path,
                body,
            },
            Kind::Append => WriteAction::Append {
                path: self.path,
                body,
            },
            Kind::Edit {
                start_line,
                end_line,
            } => WriteAction::Edit {
                path: self.path,
                start_line,
                end_line,
                body,
            },
        }
    }
}

enum LineClass {
    Ignore,
    Open(Opener),
    Malformed(String),
}

fn classify(line: &str) -> LineClass {
    let line = line.trim();
    if let Some(inner) = bracket_inner(line, "WRITE") {
        return path_opener(inner, Kind::Write, WRITE_CLOSE, true);
    }
    if let Some(inner) = bracket_inner(line, "APPEND") {
        return path_opener(inner, Kind::Append, APPEND_CLOSE, true);
    }
    if let Some(inner) = bracket_inner(line, "EDIT") {
        return match parse_edit_inner(inner) {
            Ok((path, start_line, end_line)) => LineClass::Open(Opener {
                kind: Kind::Edit {
                    start_line,
                    end_line,
                },
                path,
                closer: EDIT_CLOSE,
                bracket: true,
            }),
            Err(error) => LineClass::Malformed(error),
        };
    }
    if let Some(path) = colon_path(line, "WRITE:") {
        return path_opener(path, Kind::Write, WRITE_CLOSE, false);
    }
    if let Some(path) = colon_path(line, "APPEND:") {
        return path_opener(path, Kind::Append, APPEND_CLOSE, false);
    }
    if let Some(rest) = line.strip_prefix("EDIT:") {
        if let Some((path, start_line, end_line)) = parse_colon_edit(rest.trim()) {
            return LineClass::Open(Opener {
                kind: Kind::Edit {
                    start_line,
                    end_line,
                },
                path,
                closer: EDIT_CLOSE,
                bracket: false,
            });
        }
    }
    LineClass::Ignore
}

fn path_opener(path: &str, kind: Kind, closer: &'static str, bracket: bool) -> LineClass {
    let path = path.trim();
    if path.is_empty() {
        return if bracket {
            LineClass::Malformed("write tag is missing a path".into())
        } else {
            LineClass::Ignore
        };
    }
    LineClass::Open(Opener {
        kind,
        path: path.to_string(),
        closer,
        bracket,
    })
}

fn bracket_inner<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let rest = line
        .strip_prefix("[[")?
        .strip_prefix(name)?
        .strip_prefix(':')?;
    rest.strip_suffix("]]")
}

fn colon_path<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(prefix)?.trim();
    if rest.is_empty() || rest.contains("[[") {
        return None;
    }
    Some(rest)
}

fn parse_edit_inner(inner: &str) -> Result<(String, usize, usize), String> {
    let Some((path, spec)) = inner.split_once('|') else {
        return Err("EDIT tag needs a path and a line range".into());
    };
    let path = path.trim();
    if path.is_empty() {
        return Err("EDIT tag is missing a path".into());
    }
    let Some((start_line, end_line)) = parse_edit_spec(spec.trim()) else {
        return Err("EDIT tag needs `line N` or `lines N-M`".into());
    };
    Ok((path.to_string(), start_line, end_line))
}

fn parse_colon_edit(rest: &str) -> Option<(String, usize, usize)> {
    if !rest.ends_with(']') {
        return None;
    }
    let open = rest.rfind('[')?;
    let path = rest[..open].trim();
    if path.is_empty() {
        return None;
    }
    let (start_line, end_line) = parse_edit_spec(rest[open..].trim())?;
    Some((path.to_string(), start_line, end_line))
}

fn parse_edit_spec(spec: &str) -> Option<(usize, usize)> {
    let spec = spec.trim();
    let spec = spec
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(spec)
        .trim();
    if let Some(number) = spec.strip_prefix("line ") {
        let number = number.trim().parse::<usize>().ok()?;
        return (number > 0).then_some((number, number));
    }
    if let Some(range) = spec.strip_prefix("lines ") {
        let (start, end) = range.trim().split_once('-')?;
        let start = start.trim().parse::<usize>().ok()?;
        let end = end.trim().parse::<usize>().ok()?;
        if start == 0 || end < start {
            return None;
        }
        return Some((start, end));
    }
    None
}

fn ensure_trailing_newline(body: &str) -> String {
    if body.is_empty() || body.ends_with('\n') {
        body.to_string()
    } else {
        format!("{body}\n")
    }
}

fn reject_oversize(body: &str) -> Result<(), String> {
    if body.len() > MAX_WRITE_BYTES {
        Err(format!(
            "content is {} bytes; the limit is {MAX_WRITE_BYTES}",
            body.len()
        ))
    } else {
        Ok(())
    }
}

fn refused(verb: &str, path: &str, error: &str) -> String {
    format!("Observed {verb} {path}: refused ({error}). No file was changed.")
}

async fn observe(
    tools: &FileTools,
    verb: &str,
    path: &str,
    focus: Option<(usize, usize)>,
) -> String {
    match tools.read_file(path).await {
        Ok(content) => {
            let lines = split_lines(&content);
            format!(
                "Observed {verb} {path}: file now has {} lines and {} bytes.\n{}",
                lines.len(),
                content.len(),
                excerpt(&lines, focus)
            )
        }
        Err(error) => format!("Observed {verb} {path}: saved, but the read-back failed: {error}"),
    }
}

fn excerpt(lines: &[String], focus: Option<(usize, usize)>) -> String {
    if lines.is_empty() {
        return "--- file ---\n--- end ---\n".into();
    }
    let (start, end) = match focus {
        Some((start, end)) => {
            let from = start.saturating_sub(2).max(1);
            let to = end.saturating_add(2).min(lines.len());
            (from, to.max(from))
        }
        None => (1, lines.len()),
    };
    let mut out = String::from("--- file ---\n");
    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        if number < start || number > end {
            continue;
        }
        out.push_str(&format!("{number}|{line}\n"));
        if out.len() > 4000 {
            out.push_str("--- excerpt truncated ---\n");
            return out;
        }
    }
    out.push_str("--- end ---\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(response: &str) -> WriteAction {
        match parse(response) {
            ParseOutcome::Action(action) => action,
            other => panic!("expected an action, got {other:?}"),
        }
    }

    #[test]
    fn bracket_write_takes_the_interior_and_ignores_the_closer() {
        let action = action("note\n[[WRITE: notes/hello.txt]]\nhello\nworld\n[[/WRITE]]\n");
        assert_eq!(
            action,
            WriteAction::Write {
                path: "notes/hello.txt".into(),
                body: "hello\nworld".into(),
            }
        );
    }

    #[test]
    fn colon_write_requires_a_closer() {
        assert_eq!(
            parse("WRITE: filename.ext means overwrite\n"),
            ParseOutcome::None
        );
        assert_eq!(
            action("WRITE: notes/hello.txt\nhello\n[[/WRITE]]"),
            WriteAction::Write {
                path: "notes/hello.txt".into(),
                body: "hello".into(),
            }
        );
    }

    #[test]
    fn unclosed_bracket_is_malformed_and_does_not_write() {
        match parse("[[WRITE: a.txt]]\nhello\n") {
            ParseOutcome::Malformed(error) => assert!(error.contains("[[/WRITE]]"), "{error}"),
            other => panic!("expected malformed, got {other:?}"),
        }
    }

    #[test]
    fn edit_line_and_range_and_colon_form() {
        assert_eq!(
            action("[[EDIT: src/app.rs | line 3]]\nlet x = 1;\n[[/EDIT]]"),
            WriteAction::Edit {
                path: "src/app.rs".into(),
                start_line: 3,
                end_line: 3,
                body: "let x = 1;".into(),
            }
        );
        assert_eq!(
            action("[[EDIT: src/app.rs | lines 3-5]]\nreplacement\n[[/EDIT]]"),
            WriteAction::Edit {
                path: "src/app.rs".into(),
                start_line: 3,
                end_line: 5,
                body: "replacement".into(),
            }
        );
        assert_eq!(
            action("EDIT: src/app.rs [line 3]\nlet x = 1;\n[[/EDIT]]"),
            WriteAction::Edit {
                path: "src/app.rs".into(),
                start_line: 3,
                end_line: 3,
                body: "let x = 1;".into(),
            }
        );
    }

    #[test]
    fn append_keeps_a_leading_blank_line() {
        assert_eq!(
            action("[[APPEND: a.txt]]\n\nx\n[[/APPEND]]"),
            WriteAction::Append {
                path: "a.txt".into(),
                body: "\nx".into(),
            }
        );
    }

    #[test]
    fn first_block_wins() {
        let action =
            action("[[WRITE: first.txt]]\none\n[[/WRITE]]\n[[WRITE: second.txt]]\ntwo\n[[/WRITE]]");
        assert_eq!(
            action,
            WriteAction::Write {
                path: "first.txt".into(),
                body: "one".into(),
            }
        );
    }

    #[test]
    fn bad_edit_spec_is_malformed() {
        match parse("[[EDIT: file | lines 5-2]]\nx\n[[/EDIT]]") {
            ParseOutcome::Malformed(error) => assert!(error.contains("line"), "{error}"),
            other => panic!("expected malformed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn apply_writes_observes_and_refuses_escape() {
        let dir = tempfile::tempdir().unwrap();
        let tools = FileTools::new(dir.path().display().to_string());
        let wrote = apply(
            &tools,
            &WriteAction::Write {
                path: "out/hello.txt".into(),
                body: "hello".into(),
            },
        )
        .await;
        assert!(wrote.contains("Observed WRITE out/hello.txt"), "{wrote}");
        assert!(wrote.contains("1|hello"), "{wrote}");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("out/hello.txt")).unwrap(),
            "hello\n"
        );

        tools
            .write_file("src.txt", "one\ntwo\nthree\n")
            .await
            .unwrap();
        let edited = apply(
            &tools,
            &WriteAction::Edit {
                path: "src.txt".into(),
                start_line: 2,
                end_line: 2,
                body: "TWO".into(),
            },
        )
        .await;
        assert!(edited.contains("2|TWO"), "{edited}");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("src.txt")).unwrap(),
            "one\nTWO\nthree\n"
        );

        let refused = apply(
            &tools,
            &WriteAction::Write {
                path: "../escape.txt".into(),
                body: "nope".into(),
            },
        )
        .await;
        assert!(refused.contains("refused"), "{refused}");
        assert!(refused.contains("No file was changed"), "{refused}");
    }
}
