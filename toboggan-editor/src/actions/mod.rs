//! Editor actions
//!
//! This module defines action types for GPUI's action system.
#![allow(clippy::unsafe_derive_deserialize)]

use gpui::{Action, actions};
use serde::Deserialize;

use crate::state::EditorTab;

// Simple actions (no parameters)
actions!(
    editor,
    [
        NewTalk,
        OpenTalk,
        SaveTalk,
        ExportTalk,
        Quit,
        // Edit operations
        Undo,
        Redo,
        // Slide/Part operations
        NewPart,
        NewSlide,
        ToggleSlideSkip,
        SelectCover,
        // View operations
        ToggleLeftPanel,
        ToggleRightPanel,
    ]
);

/// Action to select a tab in the center panel
#[derive(Debug, Clone, PartialEq, Deserialize, Action)]
#[action(namespace = editor, no_json)]
pub struct SelectTab(pub EditorTab);

/// Action to select a part in the left panel
#[derive(Debug, Clone, PartialEq, Deserialize, Action)]
#[action(namespace = editor, no_json)]
pub struct SelectPart(pub usize);

/// Action to select a slide in the left panel
#[derive(Debug, Clone, PartialEq, Deserialize, Action)]
#[action(namespace = editor, no_json)]
pub struct SelectSlide {
    pub part_index: Option<usize>,
    pub slide_index: usize,
}

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    #[default]
    Toml,
    Html,
    Folder,
}
