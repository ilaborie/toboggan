//! Panel components for the editor layout

mod editor;
mod inspector;
mod outline;

pub use editor::{EditorPanel, SlideInputs};
pub use inspector::InspectorPanel;
pub use outline::{OutlinePanel, build_tree_items, parse_tree_id};
