//! The event journal: a plain text file next to the database, one line per event.
//!
//! Best-effort like the beeper: a journal that cannot be written must never stop the
//! bar, so every error is swallowed after being reported once.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Local;

use crate::domain::ports::Journal;
use crate::infrastructure::redb_store;

/// Appends timestamped lines to `path`.
pub struct FileJournal {
    path: PathBuf,
    reported: AtomicBool,
}

impl FileJournal {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            reported: AtomicBool::new(false),
        }
    }

    /// `journal.log` in the same directory as the database.
    pub fn default_path() -> PathBuf {
        redb_store::data_dir().join("journal.log")
    }

    fn report(&self, error: &std::io::Error) {
        if !self.reported.swap(true, Ordering::Relaxed) {
            eprintln!(
                "wiptracker: the journal {} cannot be written: {error}",
                self.path.display()
            );
        }
    }
}

impl Journal for FileJournal {
    fn record(&self, event: &str) {
        let line = format!("{}  {event}\n", Local::now().format("%Y-%m-%d %H:%M:%S"));
        let written = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .and_then(|mut file| file.write_all(line.as_bytes()));
        if let Err(error) = written {
            self.report(&error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each event becomes one dated line, appended after the ones before it.
    #[test]
    fn events_are_appended_as_dated_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("journal.log");
        let journal = FileJournal::new(path.clone());

        journal.record("first");
        journal.record("second");

        let text = std::fs::read_to_string(&path).expect("journal exists");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("  first"), "{:?}", lines[0]);
        assert!(lines[1].ends_with("  second"), "{:?}", lines[1]);
        assert_eq!(lines[0].len(), "2026-09-30 11:14:40  first".len());
    }

    /// A journal in a directory that does not exist swallows the error and carries on.
    #[test]
    fn an_unwritable_journal_never_panics() {
        let journal = FileJournal::new(PathBuf::from("/nonexistent/dir/journal.log"));
        journal.record("lost");
        journal.record("also lost");
    }
}
