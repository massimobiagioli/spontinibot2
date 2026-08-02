use async_trait::async_trait;

use crate::rag_engine::ports::TrainingNotesPort;
use crate::rag_engine::types::RagError;

/// Reads every `.md` file directly under `dir` (non-recursive) and
/// concatenates them into one block of supplementary system instructions.
///
/// Deliberately uncached: `/train` writes fresh notes into this directory at
/// any time, and the whole point is that the very next chat answer picks
/// them up — no `/admin/persona/reload`-style cache-busting step is needed
/// or wanted here (see ADR 0016).
pub struct TrainingNotesAdapter {
    dir: String,
}

impl TrainingNotesAdapter {
    pub fn new(dir: String) -> Self {
        Self { dir }
    }
}

#[async_trait]
impl TrainingNotesPort for TrainingNotesAdapter {
    async fn training_notes(&self) -> Result<String, RagError> {
        // Plain blocking `std::fs`, matching this codebase's convention for
        // small local-disk reads (see `auth::credential`) — a handful of
        // short `.md` files, no async fs feature needed for it.
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            // No training directory yet (fresh install, or `/train` has
            // never run) is a normal, non-fatal state — not every
            // deployment has curated training notes.
            Err(_) => return Ok(String::new()),
        };

        let mut paths: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .collect();
        paths.sort();

        let notes: Vec<String> = paths
            .into_iter()
            .filter_map(|path| std::fs::read_to_string(&path).ok())
            .map(|content| content.trim().to_string())
            .filter(|trimmed| !trimmed.is_empty())
            .collect();

        Ok(notes.join("\n\n---\n\n"))
    }
}

/// Used as `RagEngine`'s default until `with_training_notes` opts a real
/// adapter in — keeps every existing `RagEngine::new()` call site (tests
/// included) unaffected by this feature.
pub struct NoopTrainingNotes;

#[async_trait]
impl TrainingNotesPort for NoopTrainingNotes {
    async fn training_notes(&self) -> Result<String, RagError> {
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static DIR_COUNTER: AtomicU32 = AtomicU32::new(1);

    fn temp_dir() -> std::path::PathBuf {
        let n = DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("training_notes_test_{n}"));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[tokio::test]
    async fn should_return_empty_string_when_directory_does_not_exist() {
        let adapter = TrainingNotesAdapter::new("/nonexistent/path/xyz".into());
        let notes = adapter.training_notes().await.unwrap();
        assert_eq!(notes, "");
    }

    #[tokio::test]
    async fn should_return_empty_string_when_directory_is_empty() {
        let dir = temp_dir();
        let adapter = TrainingNotesAdapter::new(dir.to_string_lossy().into_owned());
        let notes = adapter.training_notes().await.unwrap();
        assert_eq!(notes, "");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn should_concatenate_md_files_sorted_by_filename() {
        let dir = temp_dir();
        std::fs::write(dir.join("b-second.md"), "Seconda nota.").unwrap();
        std::fs::write(dir.join("a-first.md"), "Prima nota.").unwrap();

        let adapter = TrainingNotesAdapter::new(dir.to_string_lossy().into_owned());
        let notes = adapter.training_notes().await.unwrap();

        let first_pos = notes.find("Prima nota.").unwrap();
        let second_pos = notes.find("Seconda nota.").unwrap();
        assert!(
            first_pos < second_pos,
            "a-first.md must come before b-second.md"
        );
        assert!(notes.contains("\n\n---\n\n"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn should_ignore_non_markdown_files() {
        let dir = temp_dir();
        std::fs::write(dir.join("note.md"), "Nota valida.").unwrap();
        std::fs::write(dir.join("readme.txt"), "Da ignorare.").unwrap();

        let adapter = TrainingNotesAdapter::new(dir.to_string_lossy().into_owned());
        let notes = adapter.training_notes().await.unwrap();

        assert_eq!(notes, "Nota valida.");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn should_trim_each_note_and_skip_blank_files() {
        let dir = temp_dir();
        std::fs::write(dir.join("a.md"), "  Con spazi.  \n").unwrap();
        std::fs::write(dir.join("b.md"), "   \n").unwrap();

        let adapter = TrainingNotesAdapter::new(dir.to_string_lossy().into_owned());
        let notes = adapter.training_notes().await.unwrap();

        assert_eq!(notes, "Con spazi.");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn noop_adapter_should_always_return_empty_string() {
        let adapter = NoopTrainingNotes;
        assert_eq!(adapter.training_notes().await.unwrap(), "");
    }
}
