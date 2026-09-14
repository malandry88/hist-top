use crate::HistoryEntry;
use std::collections::HashMap;

/// Collapses leading/trailing whitespace and internal runs of whitespace
/// down to single spaces, so `git   status` and `git status` count as the
/// same command.
pub fn normalize_command(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Counts how many times each normalized command appears.
///
/// Returns pairs sorted by descending count, breaking ties alphabetically
/// so the output order is stable across runs.
pub fn command_frequency(entries: &[HistoryEntry]) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for entry in entries {
        let key = normalize_command(&entry.command);
        if key.is_empty() {
            continue;
        }
        *counts.entry(key).or_insert(0) += 1;
    }

    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
}

/// Keeps only entries whose normalized command has at least `min_words`
/// space-separated tokens. Useful for filtering noise like bare `ls` or
/// `cd` out of a frequency report.
pub fn filter_by_min_words(entries: &[HistoryEntry], min_words: usize) -> Vec<HistoryEntry> {
    entries
        .iter()
        .filter(|e| normalize_command(&e.command).split(' ').count() >= min_words)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(command: &str) -> HistoryEntry {
        HistoryEntry { command: command.to_string(), timestamp: None }
    }

    #[test]
    fn normalizes_internal_whitespace() {
        assert_eq!(normalize_command("git   status "), "git status");
        assert_eq!(normalize_command("  ls"), "ls");
    }

    #[test]
    fn ranks_by_descending_frequency() {
        let entries = vec![
            entry("git status"),
            entry("ls"),
            entry("git status"),
            entry("git status"),
            entry("ls"),
        ];
        let ranked = command_frequency(&entries);
        assert_eq!(ranked, vec![("git status".to_string(), 3), ("ls".to_string(), 2)]);
    }

    #[test]
    fn ties_break_alphabetically() {
        let entries = vec![entry("zsh"), entry("bash")];
        let ranked = command_frequency(&entries);
        assert_eq!(ranked, vec![("bash".to_string(), 1), ("zsh".to_string(), 1)]);
    }

    #[test]
    fn filters_by_minimum_word_count() {
        let entries = vec![entry("ls"), entry("git status"), entry("cd")];
        let filtered = filter_by_min_words(&entries, 2);
        assert_eq!(filtered, vec![entry("git status")]);
    }
}
