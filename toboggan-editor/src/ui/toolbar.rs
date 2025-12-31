//! Toolbar component

use gpui::prelude::*;
use gpui::{App, FocusHandle, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};
use gpui_component::Disableable;
use gpui_component::button::{Button, ButtonGroup};
use gpui_component::divider::Divider;
use gpui_component::theme::ActiveTheme;

use crate::actions::{ExportTalk, NewPart, NewSlide, NewTalk, OpenTalk, Redo, SaveTalk, Undo};

/// Toolbar data for rendering
#[allow(clippy::struct_excessive_bools)]
pub struct ToolbarData {
    pub has_talk: bool,
    pub is_dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}

/// Toolbar component
#[derive(IntoElement)]
pub struct Toolbar {
    data: ToolbarData,
    focus_handle: FocusHandle,
}

impl Toolbar {
    #[must_use]
    pub fn new(state: &crate::state::EditorState, focus_handle: FocusHandle) -> Self {
        Self {
            data: ToolbarData {
                has_talk: state.has_talk(),
                is_dirty: state.dirty,
                can_undo: state.can_undo(),
                can_redo: state.can_redo(),
            },
            focus_handle,
        }
    }
}

impl RenderOnce for Toolbar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ToolbarData {
            has_talk,
            is_dirty: _, // Reserved for future "unsaved" indicator
            can_undo,
            can_redo,
        } = self.data;

        div()
            .id("toolbar")
            .flex()
            .w_full()
            .h(px(40.))
            .px_3()
            .gap_2()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().secondary)
            .child(file_ops_group(has_talk, self.focus_handle.clone()))
            .child(Divider::vertical().h(px(20.)))
            .child(edit_ops_group(
                can_undo,
                can_redo,
                self.focus_handle.clone(),
            ))
            .child(Divider::vertical().h(px(20.)))
            .child(slide_ops_group(has_talk, self.focus_handle.clone()))
            .child(div().flex_1())
            .child(export_button(has_talk, self.focus_handle))
    }
}

/// File operations button group (New, Open, Save)
fn file_ops_group(has_talk: bool, focus_handle: FocusHandle) -> ButtonGroup {
    let fh_new = focus_handle.clone();
    let fh_open = focus_handle.clone();
    let fh_save = focus_handle;

    ButtonGroup::new("file-ops")
        .child(
            Button::new("new")
                .label("New")
                .tooltip("New Talk")
                .on_click(move |_, window, cx| {
                    fh_new.dispatch_action(&NewTalk, window, cx);
                }),
        )
        .child(
            Button::new("open")
                .label("Open")
                .tooltip("Open Talk")
                .on_click(move |_, window, cx| {
                    fh_open.dispatch_action(&OpenTalk, window, cx);
                }),
        )
        .child(
            Button::new("save")
                .label("Save")
                .tooltip("Save")
                .disabled(!has_talk)
                .on_click(move |_, window, cx| {
                    fh_save.dispatch_action(&SaveTalk, window, cx);
                }),
        )
}

/// Edit operations button group (Undo, Redo)
fn edit_ops_group(can_undo: bool, can_redo: bool, focus_handle: FocusHandle) -> ButtonGroup {
    let fh_undo = focus_handle.clone();
    let fh_redo = focus_handle;

    ButtonGroup::new("edit-ops")
        .child(
            Button::new("undo")
                .label("Undo")
                .tooltip("Undo")
                .disabled(!can_undo)
                .on_click(move |_, window, cx| {
                    fh_undo.dispatch_action(&Undo, window, cx);
                }),
        )
        .child(
            Button::new("redo")
                .label("Redo")
                .tooltip("Redo")
                .disabled(!can_redo)
                .on_click(move |_, window, cx| {
                    fh_redo.dispatch_action(&Redo, window, cx);
                }),
        )
}

/// Slide/Part operations button group (New Part, New Slide)
fn slide_ops_group(has_talk: bool, focus_handle: FocusHandle) -> ButtonGroup {
    let fh_new_part = focus_handle.clone();
    let fh_new_slide = focus_handle;

    ButtonGroup::new("slide-ops")
        .child(
            Button::new("new-part")
                .label("Part")
                .tooltip("New Part")
                .disabled(!has_talk)
                .on_click(move |_, window, cx| {
                    fh_new_part.dispatch_action(&NewPart, window, cx);
                }),
        )
        .child(
            Button::new("new-slide")
                .label("Slide")
                .tooltip("New Slide")
                .disabled(!has_talk)
                .on_click(move |_, window, cx| {
                    fh_new_slide.dispatch_action(&NewSlide, window, cx);
                }),
        )
}

/// Export button
fn export_button(has_talk: bool, focus_handle: FocusHandle) -> Button {
    Button::new("export")
        .label("Export")
        .disabled(!has_talk)
        .on_click(move |_, window, cx| {
            focus_handle.dispatch_action(&ExportTalk, window, cx);
        })
}
