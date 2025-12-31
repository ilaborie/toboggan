//! Icon definitions using gpui-component icons
//!
//! This module re-exports gpui-component's Icon system and provides
//! convenience mappings for editor-specific icon needs.

// Re-export Icon and IconName from gpui-component
/// Icon size helper to use with gpui-component's Size system
pub use gpui_component::Size as IconSize;
pub use gpui_component::{Icon, IconName, Sizable};
