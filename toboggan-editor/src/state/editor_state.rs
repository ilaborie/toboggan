//! Main editor state

use std::path::PathBuf;

use serde::Deserialize;

use super::{History, Selection, SlideState, TalkState};

/// Central editor tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum EditorTab {
    #[default]
    Content,
    Notes,
    Style,
}

/// Maximum undo history size
const MAX_UNDO_HISTORY: usize = 50;

/// Main editor state
#[derive(Debug)]
pub struct EditorState {
    /// Current talk being edited
    pub current_talk: Option<TalkState>,

    /// Currently selected item
    pub selection: Selection,

    /// Whether there are unsaved changes
    pub dirty: bool,

    /// Path to current file (None for new/unsaved)
    pub file_path: Option<PathBuf>,

    /// Undo/redo history (stores `TalkState` directly, no wrapper needed)
    pub history: History<TalkState>,

    /// Currently active editor tab
    pub active_tab: EditorTab,

    /// Whether the left panel (slide navigator) is visible
    pub show_left_panel: bool,

    /// Whether the right panel (properties) is visible
    pub show_right_panel: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            current_talk: None,
            selection: Selection::None,
            dirty: false,
            file_path: None,
            history: History::new(MAX_UNDO_HISTORY),
            active_tab: EditorTab::Content,
            show_left_panel: true,
            show_right_panel: true,
        }
    }
}

impl EditorState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new empty talk
    pub fn new_talk(&mut self, title: String, date: toboggan_core::Date) {
        self.current_talk = Some(TalkState::new(title, date));
        self.selection = Selection::Cover;
        self.dirty = true;
        self.file_path = None;
        self.history.clear();
    }

    /// Load talk from TOML string
    pub fn load_talk(&mut self, content: &str, path: PathBuf) -> anyhow::Result<()> {
        let talk: toboggan_core::Talk = toml::from_str(content)
            .map_err(|err| anyhow::anyhow!("Failed to parse TOML: {err}"))?;

        self.current_talk = Some(TalkState::from_core(talk));
        self.selection = Selection::Cover;
        self.dirty = false;
        self.file_path = Some(path);
        self.history.clear();

        Ok(())
    }

    /// Mark as saved
    pub fn mark_saved(&mut self, path: PathBuf) {
        self.dirty = false;
        self.file_path = Some(path);
    }

    /// Record current state for undo
    pub fn record_state(&mut self) {
        if let Some(talk) = &self.current_talk {
            self.history.push(talk.clone());
            self.dirty = true;
        }
    }

    /// Undo last change
    pub fn undo(&mut self) {
        // First save current state for redo
        if let Some(talk) = &self.current_talk {
            self.history.save_for_redo(talk.clone());
        }

        if let Some(previous) = self.history.undo() {
            self.current_talk = Some(previous);
            self.dirty = true;
        }
    }

    /// Redo last undone change
    pub fn redo(&mut self) {
        if let Some(next) = self.history.redo() {
            self.current_talk = Some(next);
            self.dirty = true;
        }
    }

    /// Check if undo is available
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Check if redo is available
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Get currently selected slide (if any)
    #[must_use]
    pub fn current_slide(&self) -> Option<&SlideState> {
        self.current_talk
            .as_ref()
            .and_then(|talk| self.selection.get_slide(talk))
    }

    /// Get currently selected slide mutably
    pub fn current_slide_mut(&mut self) -> Option<&mut SlideState> {
        let talk = self.current_talk.as_mut()?;
        self.selection.get_slide_mut(talk)
    }

    /// Check if there's a talk loaded
    #[must_use]
    pub fn has_talk(&self) -> bool {
        self.current_talk.is_some()
    }

    /// Get the file name for display
    #[must_use]
    pub fn file_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map_or_else(
                || "Untitled".into(),
                |name| name.to_string_lossy().into_owned(),
            )
    }

    /// Get the current slide position (1-indexed current, total)
    #[must_use]
    pub fn slide_position(&self) -> Option<(usize, usize)> {
        let talk = self.current_talk.as_ref()?;
        let total = talk.total_slides();
        let current = self.selection.display_number(talk)?;
        Some((current, total))
    }

    /// Calculate word count for the entire talk
    #[must_use]
    pub fn word_count(&self) -> usize {
        self.current_talk.as_ref().map_or(0, TalkState::word_count)
    }
}
