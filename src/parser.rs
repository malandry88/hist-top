use crate::HistoryEntry;

/// Parses a zsh extended history line: `: 1694721600:0;git status`.
///
/// The two numbers after `:` are start time and elapsed seconds; only the
/// start time is kept. Returns `None` if the line doesn't have that shape,
/// so callers can fall through to other formats.
fn parse_zsh_extended_line(line: &str) -> Option<HistoryEntry> {
    let rest = line.strip_prefix(": ")?;
    let (meta, command) = rest.split_once(';')?;
    let (start, _elapsed) = meta.split_once(':')?;
    let timestamp = start.trim().parse::<u64>().ok()?;
    Some(HistoryEntry {
        command: command.to_string(),
        timestamp: Some(timestamp),
    })
}

/// Parses a bash extended history timestamp comment: `#1694721600`.
///
/// Bash writes these on their own line immediately before the command they
/// belong to, when `HISTTIMEFORMAT` is set.
fn parse_bash_timestamp_line(line: &str) -> Option<u64> {
    let digits = line.strip_prefix('#')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// True if `line` ends with a backslash that isn't itself escaped, i.e. an
/// odd number of trailing backslashes. That's the shape bash and zsh write
/// when a command was continued onto the next physical line.
fn ends_with_continuation_backslash(line: &str) -> bool {
    let trailing = line.chars().rev().take_while(|&c| c == '\\').count();
    trailing % 2 == 1
}

/// Joins physical lines that end with a lone trailing backslash into one
/// logical line, dropping the backslash itself. A command typed as
/// `git commit -m "foo \` / `bar"` is stored across two lines in the
/// history file; without this it would be counted as two unrelated, mostly
/// meaningless entries instead of one.
fn join_continuations(content: &str) -> Vec<String> {
    let mut logical_lines: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim_end_matches('\r');
        // No separator is inserted here: a backslash-newline is removed
        // outright by the shell, so whatever spacing existed before the
        // backslash on the first line is all the spacing that survives.
        let mut current = match pending.take() {
            Some(prefix) => format!("{prefix}{line}"),
            None => line.to_string(),
        };

        if ends_with_continuation_backslash(&current) {
            current.pop();
            pending = Some(current);
        } else {
            logical_lines.push(current);
        }
    }

    // A trailing backslash on the last line of the file has nothing to
    // continue onto; keep whatever was accumulated rather than dropping it.
    if let Some(prefix) = pending {
        logical_lines.push(prefix);
    }

    logical_lines
}

/// Parses the full contents of a shell history file into entries.
///
/// Handles, in order of preference per line:
/// - zsh extended history: `: 1694721600:0;git status`
/// - bash extended history: a `#<unix-seconds>` line followed by the command
/// - plain history: one command per line, no timestamp
///
/// Lines ending in a lone trailing backslash are joined with the next line
/// first, so multi-line commands are reassembled into a single entry.
/// Blank lines are dropped. Aside from that joining and trimming a trailing
/// `\r`, command text is used as-is; no shell parsing or quoting is
/// attempted.
pub fn parse_history(content: &str) -> Vec<HistoryEntry> {
    let mut entries = Vec::new();
    let mut pending_timestamp: Option<u64> = None;

    for line in join_continuations(content) {
        if line.is_empty() {
            continue;
        }

        if let Some(entry) = parse_zsh_extended_line(&line) {
            entries.push(entry);
            pending_timestamp = None;
            continue;
        }

        if let Some(ts) = parse_bash_timestamp_line(&line) {
            pending_timestamp = Some(ts);
            continue;
        }

        entries.push(HistoryEntry {
            command: line,
            timestamp: pending_timestamp.take(),
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_bash_history() {
        let content = "git status\nls -la\n";
        let entries = parse_history(content);
        assert_eq!(
            entries,
            vec![
                HistoryEntry { command: "git status".into(), timestamp: None },
                HistoryEntry { command: "ls -la".into(), timestamp: None },
            ]
        );
    }

    #[test]
    fn parses_bash_extended_history() {
        let content = "#1694721600\ngit status\n#1694721650\nls -la\n";
        let entries = parse_history(content);
        assert_eq!(
            entries,
            vec![
                HistoryEntry { command: "git status".into(), timestamp: Some(1694721600) },
                HistoryEntry { command: "ls -la".into(), timestamp: Some(1694721650) },
            ]
        );
    }

    #[test]
    fn parses_zsh_extended_history() {
        let content = ": 1694721600:0;git status\n: 1694721650:2;cargo test\n";
        let entries = parse_history(content);
        assert_eq!(
            entries,
            vec![
                HistoryEntry { command: "git status".into(), timestamp: Some(1694721600) },
                HistoryEntry { command: "cargo test".into(), timestamp: Some(1694721650) },
            ]
        );
    }

    #[test]
    fn skips_blank_lines() {
        let content = "git status\n\n\nls -la\n";
        let entries = parse_history(content);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn joins_backslash_continued_plain_command() {
        let content = "git commit -m \"foo \\\nbar\"\nls\n";
        let entries = parse_history(content);
        assert_eq!(
            entries,
            vec![
                HistoryEntry { command: "git commit -m \"foo bar\"".into(), timestamp: None },
                HistoryEntry { command: "ls".into(), timestamp: None },
            ]
        );
    }

    #[test]
    fn joins_multiple_continuation_lines() {
        let content = "one \\\ntwo \\\nthree\n";
        let entries = parse_history(content);
        assert_eq!(entries, vec![HistoryEntry { command: "one two three".into(), timestamp: None }]);
    }

    #[test]
    fn joins_backslash_continued_zsh_extended_command() {
        let content = ": 1694721600:0;echo foo \\\nbar\n";
        let entries = parse_history(content);
        assert_eq!(
            entries,
            vec![HistoryEntry { command: "echo foo bar".into(), timestamp: Some(1694721600) }]
        );
    }

    #[test]
    fn trailing_backslash_on_last_line_is_kept_as_is() {
        let content = "git status\nsome command \\";
        let entries = parse_history(content);
        assert_eq!(
            entries,
            vec![
                HistoryEntry { command: "git status".into(), timestamp: None },
                HistoryEntry { command: "some command ".into(), timestamp: None },
            ]
        );
    }

    #[test]
    fn escaped_double_backslash_is_not_a_continuation() {
        let content = "echo foo\\\\\nls\n";
        let entries = parse_history(content);
        assert_eq!(
            entries,
            vec![
                HistoryEntry { command: "echo foo\\\\".into(), timestamp: None },
                HistoryEntry { command: "ls".into(), timestamp: None },
            ]
        );
    }

    #[test]
    fn stray_hash_comment_without_digits_is_not_a_timestamp() {
        let content = "# just a comment\ngit status\n";
        let entries = parse_history(content);
        assert_eq!(
            entries,
            vec![
                HistoryEntry { command: "# just a comment".into(), timestamp: None },
                HistoryEntry { command: "git status".into(), timestamp: None },
            ]
        );
    }
}
