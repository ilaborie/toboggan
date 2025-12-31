//! Inspector panel - metadata and properties

use gpui::prelude::*;
use gpui::{
    App, Entity, FocusHandle, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px,
};
use gpui_component::Sizable;
use gpui_component::calendar::{Calendar, CalendarState};
use gpui_component::input::{Input, InputState};
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::switch::Switch;
use gpui_component::theme::ActiveTheme;

use crate::actions::ToggleSlideSkip;
use crate::state::EditorState;

/// Data for the inspector panel
pub struct InspectorPanelData {
    pub has_slide: bool,
    pub slide_skip: bool,
    pub slide_display_title: String,
}

/// Inspector panel component
#[derive(IntoElement)]
pub struct InspectorPanel {
    data: InspectorPanelData,
    focus_handle: FocusHandle,
    title_input: Entity<InputState>,
    calendar_state: Entity<CalendarState>,
}

impl InspectorPanel {
    #[must_use]
    pub fn new(
        state: &EditorState,
        focus_handle: FocusHandle,
        title_input: Entity<InputState>,
        calendar_state: Entity<CalendarState>,
    ) -> Self {
        let (has_slide, slide_skip, slide_display_title) = state
            .current_talk
            .as_ref()
            .and_then(|talk| state.selection.get_slide(talk))
            .map_or((false, false, String::new()), |slide| {
                (true, slide.skip, slide.display_title().to_owned())
            });

        Self {
            data: InspectorPanelData {
                has_slide,
                slide_skip,
                slide_display_title,
            },
            focus_handle,
            title_input,
            calendar_state,
        }
    }
}

impl RenderOnce for InspectorPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = self.data;
        let focus_handle = self.focus_handle;

        div()
            .id("inspector-panel")
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().secondary)
            .border_l_1()
            .border_color(cx.theme().border)
            // Header
            .child(
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(Label::new("Properties").text_size(px(12.))),
            )
            // Content
            .child(
                div()
                    .id("inspector-panel-content")
                    .flex_1()
                    .overflow_y_scrollbar()
                    .p_2()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            // Talk Metadata Section
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        Label::new("Talk Metadata")
                                            .text_size(px(12.))
                                            .font_weight(gpui::FontWeight::SEMIBOLD),
                                    )
                                    .child(TalkMetadataSection {
                                        title_input: self.title_input,
                                        calendar_state: self.calendar_state,
                                    }),
                            )
                            // Slide Properties Section (if slide selected)
                            .when(data.has_slide, |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .child(
                                            Label::new("Slide Properties")
                                                .text_size(px(12.))
                                                .font_weight(gpui::FontWeight::SEMIBOLD),
                                        )
                                        .child(SlidePropertiesSection {
                                            skip: data.slide_skip,
                                            display_title: data.slide_display_title,
                                            focus_handle,
                                        }),
                                )
                            }),
                    ),
            )
    }
}

/// Talk metadata section
#[derive(IntoElement)]
struct TalkMetadataSection {
    title_input: Entity<InputState>,
    calendar_state: Entity<CalendarState>,
}

impl RenderOnce for TalkMetadataSection {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            // Title - editable input
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Label::new("Title").text_size(px(10.)))
                    .child(Input::new(&self.title_input)),
            )
            // Date - Calendar picker
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Label::new("Date").text_size(px(10.)))
                    .child(
                        Calendar::new(&self.calendar_state).with_size(gpui_component::Size::Small),
                    ),
            )
    }
}

/// Slide properties section
#[derive(IntoElement)]
struct SlidePropertiesSection {
    skip: bool,
    display_title: String,
    focus_handle: FocusHandle,
}

impl RenderOnce for SlidePropertiesSection {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.focus_handle;

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Skip toggle
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new("Skip Slide").text_size(px(12.)))
                    .child(Switch::new("skip-toggle").checked(self.skip).on_click(
                        move |_, window, cx| {
                            focus_handle.dispatch_action(&ToggleSlideSkip, window, cx);
                        },
                    )),
            )
            // Title (display only)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Label::new("Title").text_size(px(10.)))
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded(px(4.))
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(Label::new(self.display_title).text_size(px(12.))),
                    ),
            )
    }
}
