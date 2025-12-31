//! Outline panel - slide navigator using Tree component

use gpui::prelude::*;
use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};
use gpui_component::label::Label;
use gpui_component::list::ListItem;
use gpui_component::theme::ActiveTheme;
use gpui_component::tree::{TreeEntry, TreeItem, TreeState, tree};

use crate::icons::{Icon, IconName, IconSize, Sizable};

/// Outline panel component - slide navigator
#[derive(IntoElement)]
pub struct OutlinePanel {
    tree_state: Entity<TreeState>,
}

impl OutlinePanel {
    #[must_use]
    pub fn new(tree_state: Entity<TreeState>) -> Self {
        Self { tree_state }
    }
}

impl RenderOnce for OutlinePanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id("outline-panel")
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().secondary)
            .border_r_1()
            .border_color(cx.theme().border)
            // Header
            .child(
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(Label::new("Slides").text_size(px(12.))),
            )
            // Tree content
            .child(
                div()
                    .id("outline-panel-content")
                    .flex_1()
                    .overflow_hidden()
                    .p_2()
                    .child(tree(&self.tree_state, render_tree_item)),
            )
    }
}

/// Render a tree item with appropriate icon
fn render_tree_item(
    ix: usize,
    entry: &TreeEntry,
    selected: bool,
    _window: &mut Window,
    _cx: &mut App,
) -> ListItem {
    let item = entry.item();
    let is_folder = entry.is_folder();
    let is_expanded = entry.is_expanded();
    let depth = entry.depth();

    // Determine icon based on item type
    let icon = if is_folder {
        if is_expanded {
            Icon::new(IconName::FolderOpen).with_size(IconSize::Small)
        } else {
            Icon::new(IconName::Folder).with_size(IconSize::Small)
        }
    } else {
        Icon::new(IconName::File).with_size(IconSize::Small)
    };

    #[allow(clippy::cast_precision_loss)]
    let indent = px(16. * depth as f32);

    ListItem::new(ix)
        .pl(indent)
        .child(
            div()
                .flex()
                .gap_2()
                .items_center()
                .child(icon)
                .child(Label::new(item.label.clone()).text_size(px(12.))),
        )
        .selected(selected)
}

/// Build tree items from talk data
#[must_use]
pub fn build_tree_items(talk: &crate::state::TalkState) -> Vec<TreeItem> {
    let mut items = Vec::new();

    // Add parts with their slides as children
    for (part_idx, part) in talk.parts.iter().enumerate() {
        let part_id = format!("part:{part_idx}");
        let part_title = part.title.clone();
        let slide_items: Vec<TreeItem> = part
            .slides
            .iter()
            .enumerate()
            .map(|(slide_idx, slide)| {
                let slide_id = format!("slide:{part_idx}:{slide_idx}");
                let slide_title = slide.display_title().to_owned();
                TreeItem::new(slide_id, slide_title)
            })
            .collect();
        let part_item = TreeItem::new(part_id, part_title)
            .expanded(true)
            .children(slide_items);
        items.push(part_item);
    }

    // Add loose slides
    for (slide_idx, slide) in talk.loose_slides.iter().enumerate() {
        let slide_id = format!("loose:{slide_idx}");
        let slide_title = slide.display_title().to_owned();
        items.push(TreeItem::new(slide_id, slide_title));
    }

    items
}

/// Parse a tree item ID to get the selection
#[must_use]
pub fn parse_tree_id(id: &str) -> Option<crate::state::Selection> {
    let parts: Vec<&str> = id.split(':').collect();
    match parts.as_slice() {
        ["part", idx] => {
            let index = idx.parse().ok()?;
            Some(crate::state::Selection::Part { index })
        }
        ["slide", part_idx, slide_idx] => {
            let part_index = Some(part_idx.parse().ok()?);
            let slide_index = slide_idx.parse().ok()?;
            Some(crate::state::Selection::Slide {
                part_index,
                slide_index,
            })
        }
        ["loose", slide_idx] => {
            let slide_index = slide_idx.parse().ok()?;
            Some(crate::state::Selection::Slide {
                part_index: None,
                slide_index,
            })
        }
        _ => None,
    }
}
