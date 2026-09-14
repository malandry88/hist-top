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

/// Parses the full contents of a shell history file into entries.
///
/// Handles, in order of preference per line:
/// - zsh extended history: `: 1694721600:0;git status`
/// - bash extended history: a `#<unix-seconds>` line followed by the command
/// - plain history: one command per line, no timestamp
///
/// Blank lines are dropped. Command text is used as-is aside from trimming
/// a trailing `\r`; no shell parsing, quoting, or line-continuation
/// handling is attempted.
pub fn parse_history(content: &str) -> Vec<HistoryEntry> {
    let mut entries = Vec::new();
    let mut pending_timestamp: Option<u64> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }

        if let Some(entry) = parse_zsh_extended_line(line) {
            entries.push(entry);
            pending_timestamp = None;
            continue;
        }

        if let Some(ts) = parse_bash_timestamp_line(line) {
            pending_timestamp = Some(ts);
            continue;
        }

        entries.push(HistoryEntry {
            command: line.to_string(),
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
