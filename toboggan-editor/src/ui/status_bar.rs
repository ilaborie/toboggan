//! Status bar component

use gpui::prelude::*;
use gpui::{App, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};
use gpui_component::badge::Badge;
use gpui_component::label::Label;
use gpui_component::theme::ActiveTheme;

use crate::icons::{Icon, IconName, IconSize, Sizable};
use crate::state::EditorState;

/// Data for the status bar
pub struct StatusBarData {
    pub has_talk: bool,
    pub file_name: String,
    pub slide_position: Option<(usize, usize)>,
    pub part_name: Option<String>,
    pub word_count: usize,
    pub is_dirty: bool,
}

/// Status bar component
#[derive(IntoElement)]
pub struct StatusBar {
    data: StatusBarData,
}

impl StatusBar {
    #[must_use]
    pub fn new(state: &EditorState) -> Self {
        let part_name = state
            .current_talk
            .as_ref()
            .and_then(|talk| state.selection.get_part(talk))
            .map(|part| part.title.clone());

        Self {
            data: StatusBarData {
                has_talk: state.has_talk(),
                file_name: state.file_name(),
                slide_position: state.slide_position(),
                part_name,
                word_count: state.word_count(),
                is_dirty: state.dirty,
            },
        }
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = self.data;

        div()
            .id("status-bar")
            .flex()
            .w_full()
            .h(px(24.))
            .px_3()
            .gap_3()
            .items_center()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().secondary)
            .text_size(px(11.))
            // File name with icon
            .child(
                div()
                    .flex()
                    .gap_1()
                    .items_center()
                    .child(Icon::new(IconName::File).with_size(IconSize::XSmall))
                    .child(Label::new(data.file_name).text_size(px(11.))),
            )
            // Modified indicator
            .when(data.is_dirty, |this| {
                this.child(Badge::new().child("Modified"))
            })
            // Separator
            .child(StatusDivider)
            // Slide position
            .when_some(data.slide_position, |this, (current, total)| {
                this.child(
                    div()
                        .flex()
                        .gap_1()
                        .items_center()
                        .child(Icon::new(IconName::GalleryVerticalEnd).with_size(IconSize::XSmall))
                        .child(Label::new(format!("{current} / {total}")).text_size(px(11.))),
                )
            })
            // Current part name
            .when_some(data.part_name, |this, name| {
                this.child(StatusDivider).child(
                    div()
                        .flex()
                        .gap_1()
                        .items_center()
                        .child(Icon::new(IconName::Folder).with_size(IconSize::XSmall))
                        .child(
                            Label::new(name)
                                .text_size(px(11.))
                                .max_w(px(150.))
                                .text_ellipsis(),
                        ),
                )
            })
            // Spacer
            .child(div().flex_1())
            // Word count
            .when(data.has_talk, |this| {
                this.child(
                    div()
                        .flex()
                        .gap_1()
                        .items_center()
                        .child(Icon::new(IconName::ALargeSmall).with_size(IconSize::XSmall))
                        .child(Label::new(format!("{} words", data.word_count)).text_size(px(11.))),
                )
            })
    }
}

/// Visual separator for status bar items
#[derive(IntoElement)]
struct StatusDivider;

impl RenderOnce for StatusDivider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div().h(px(12.)).w(px(1.)).bg(cx.theme().border)
    }
}
