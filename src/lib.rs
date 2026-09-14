pub mod parser;
pub mod stats;

/// A single command pulled out of a shell history file.
///
/// `timestamp` is the unix time the command ran, when the history file's
/// format records one. Plain (non-extended) bash history has no timestamps,
/// so it's `None` there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: Option<u64>,
}
