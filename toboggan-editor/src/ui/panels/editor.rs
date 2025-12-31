//! Editor panel - main editor area

use gpui::prelude::*;
use gpui::{
    App, Entity, FocusHandle, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px,
};
use gpui_component::Selectable;
use gpui_component::input::{Input, InputState};
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::theme::ActiveTheme;

use crate::actions::SelectTab;
use crate::icons::{Icon, IconName, IconSize, Sizable};
use crate::state::{EditorState, EditorTab};

/// Data for the center panel
pub struct EditorPanelData {
    pub active_tab: EditorTab,
    pub selected_slide: Option<SelectedSlideData>,
}

pub struct SelectedSlideData {
    pub classes: Vec<String>,
}

/// Slide input entities for the center panel
pub struct SlideInputs {
    pub title: Entity<InputState>,
    pub body: Entity<InputState>,
    pub notes: Entity<InputState>,
    pub style: Entity<InputState>,
}

/// Center panel component
#[derive(IntoElement)]
pub struct EditorPanel {
    data: EditorPanelData,
    focus_handle: FocusHandle,
    slide_inputs: SlideInputs,
}

impl EditorPanel {
    #[must_use]
    pub fn new(state: &EditorState, focus_handle: FocusHandle, slide_inputs: SlideInputs) -> Self {
        let selected_slide = state
            .current_talk
            .as_ref()
            .and_then(|talk| state.selection.get_slide(talk))
            .map(|slide| SelectedSlideData {
                classes: slide.classes.clone(),
            });

        Self {
            data: EditorPanelData {
                active_tab: state.active_tab,
                selected_slide,
            },
            focus_handle,
            slide_inputs,
        }
    }
}

impl RenderOnce for EditorPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = self.data;
        let focus_handle = self.focus_handle;

        div()
            .id("editor-panel")
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .child(if data.selected_slide.is_some() {
                // Tab bar
                div()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        TabBar::new("editor-tabs")
                            .on_click(move |index, window, cx| {
                                let tab = match index {
                                    0 => EditorTab::Content,
                                    1 => EditorTab::Notes,
                                    _ => EditorTab::Style,
                                };
                                focus_handle.dispatch_action(&SelectTab(tab), window, cx);
                            })
                            .child(
                                Tab::new()
                                    .label("Content")
                                    .selected(data.active_tab == EditorTab::Content),
                            )
                            .child(
                                Tab::new()
                                    .label("Notes")
                                    .selected(data.active_tab == EditorTab::Notes),
                            )
                            .child(
                                Tab::new()
                                    .label("Style")
                                    .selected(data.active_tab == EditorTab::Style),
                            ),
                    )
                    .into_any_element()
            } else {
                div().into_any_element()
            })
            .child(
                div()
                    .flex_1()
                    .child(if let Some(slide) = data.selected_slide {
                        match data.active_tab {
                            EditorTab::Content => ContentEditor::new(
                                self.slide_inputs.title.clone(),
                                self.slide_inputs.body.clone(),
                            )
                            .into_any_element(),
                            EditorTab::Notes => {
                                NotesEditor::new(self.slide_inputs.notes.clone()).into_any_element()
                            }
                            EditorTab::Style => {
                                StyleEditor::new(slide, self.slide_inputs.style.clone())
                                    .into_any_element()
                            }
                        }
                    } else {
                        EmptyState.into_any_element()
                    }),
            )
    }
}

/// Empty state when no slide is selected
#[derive(IntoElement)]
struct EmptyState;

impl RenderOnce for EmptyState {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(Icon::new(IconName::GalleryVerticalEnd).with_size(IconSize::Large))
                    .child(Label::new("Select a slide to edit").text_size(px(16.))),
            )
    }
}

/// Content editor tab
#[derive(IntoElement)]
struct ContentEditor {
    title_input: Entity<InputState>,
    body_input: Entity<InputState>,
}

impl ContentEditor {
    fn new(title_input: Entity<InputState>, body_input: Entity<InputState>) -> Self {
        Self {
            title_input,
            body_input,
        }
    }
}

impl RenderOnce for ContentEditor {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id("content-editor")
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .overflow_y_scrollbar()
            // Title field
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Label::new("Title").text_size(px(12.)))
                    .child(Input::new(&self.title_input)),
            )
            // Body field
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .flex_1()
                    .child(Label::new("Content").text_size(px(12.)))
                    .child(Input::new(&self.body_input).h(px(200.))),
            )
    }
}

/// Notes editor tab
#[derive(IntoElement)]
struct NotesEditor {
    notes_input: Entity<InputState>,
}

impl NotesEditor {
    fn new(notes_input: Entity<InputState>) -> Self {
        Self { notes_input }
    }
}

impl RenderOnce for NotesEditor {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id("notes-editor")
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .child(Label::new("Speaker Notes").text_size(px(12.)))
            .child(Input::new(&self.notes_input).h(px(300.)))
    }
}

/// Style editor tab
#[derive(IntoElement)]
struct StyleEditor {
    slide: SelectedSlideData,
    style_input: Entity<InputState>,
}

impl StyleEditor {
    fn new(slide: SelectedSlideData, style_input: Entity<InputState>) -> Self {
        Self { slide, style_input }
    }
}

impl RenderOnce for StyleEditor {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id("style-editor")
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .overflow_y_scrollbar()
            // CSS Classes (display only for now)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Label::new("CSS Classes").text_size(px(12.)))
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded(px(4.))
                            .border_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().secondary)
                            .child(Label::new(if self.slide.classes.is_empty() {
                                "(no classes)".to_string()
                            } else {
                                self.slide.classes.join(", ")
                            })),
                    ),
            )
            // Inline Style - CSS code editor
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .flex_1()
                    .child(Label::new("Inline Style (CSS)").text_size(px(12.)))
                    .child(Input::new(&self.style_input).h(px(200.))),
            )
    }
}
