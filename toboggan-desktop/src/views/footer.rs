use iced::widget::{container, row};
use iced::{Element, Length, Theme};
use toboggan_client::ConnectionStatus;

use crate::constants::{
    ICON_SIZE_MEDIUM, ICON_SIZE_SMALL, PADDING_CONTAINER, SPACING_MEDIUM, SPACING_SMALL,
};
use crate::icons::{Icon, icon};
use crate::message::Message;
use crate::state::AppState;
use crate::styles;
use crate::widgets::{
    NavButtonPosition, create_icon_button, create_nav_button, create_simple_button,
    create_status_row, create_status_row_with_button,
};

fn connection_status_view(status: &ConnectionStatus) -> Element<'_, Message> {
    match status {
        ConnectionStatus::Closed => create_status_row_with_button(
            icon(Icon::WifiOff, ICON_SIZE_MEDIUM),
            "Disconnected",
            create_simple_button("Connect", Message::Connect).into(),
        )
        .into(),
        ConnectionStatus::Connecting => {
            create_status_row(icon(Icon::Loader, ICON_SIZE_MEDIUM), "Connecting...").into()
        }
        ConnectionStatus::Connected => create_status_row_with_button(
            icon(Icon::Wifi, ICON_SIZE_MEDIUM),
            "Connected",
            create_icon_button(
                icon(Icon::RefreshCw, ICON_SIZE_SMALL),
                "Reconnect",
                Message::Disconnect,
            )
            .style(iced::widget::button::secondary)
            .into(),
        )
        .into(),
        ConnectionStatus::Reconnecting {
            attempt,
            max_attempt,
            ..
        } => {
            let reconnecting_text = format!("Reconnecting... ({attempt}/{max_attempt})");
            iced::widget::row![
                icon(Icon::RefreshCw, ICON_SIZE_MEDIUM),
                iced::widget::text(reconnecting_text).size(12.0)
            ]
            .spacing(SPACING_SMALL)
            .align_y(iced::Alignment::Center)
        }
        .into(),
        ConnectionStatus::Error { message } => {
            let error_text = format!("Error: {message}");
            iced::widget::row![
                icon(Icon::X, ICON_SIZE_MEDIUM),
                iced::widget::text(error_text).size(12.0),
                iced::widget::button(iced::widget::text("Retry").size(11.0))
                    .on_press(Message::Connect)
                    .padding(iced::Padding::new(2.0).right(4.0).left(4.0))
            ]
            .spacing(SPACING_SMALL)
            .align_y(iced::Alignment::Center)
        }
        .into(),
    }
}

fn navigation_controls_view() -> Element<'static, Message> {
    row![
        create_nav_button(
            icon(Icon::SkipBack, ICON_SIZE_MEDIUM),
            "First",
            Message::SendCommand(toboggan_core::Command::First),
            NavButtonPosition::Leading
        ),
        create_nav_button(
            icon(Icon::ChevronLeft, ICON_SIZE_MEDIUM),
            "Previous Step",
            Message::SendCommand(toboggan_core::Command::PreviousStep),
            NavButtonPosition::Leading
        ),
        create_nav_button(
            icon(Icon::ChevronRight, ICON_SIZE_MEDIUM),
            "Next Step",
            Message::SendCommand(toboggan_core::Command::NextStep),
            NavButtonPosition::Trailing
        ),
        create_nav_button(
            icon(Icon::SkipForward, ICON_SIZE_MEDIUM),
            "Last",
            Message::SendCommand(toboggan_core::Command::Last),
            NavButtonPosition::Trailing
        ),
    ]
    .spacing(SPACING_SMALL)
    .align_y(iced::Alignment::Center)
    .into()
}

fn step_indicators_view(state: &AppState) -> Element<'_, Message> {
    use std::cmp::Ordering;

    let Some((current_step, step_count)) = state.step_info() else {
        return iced::widget::text("").into();
    };

    if step_count == 0 {
        return iced::widget::text("").into();
    }

    let primary_color = Theme::Dark.palette().primary;

    let mut indicators = row![].spacing(2.0);
    for step in 0..step_count {
        let circle = match step.cmp(&current_step) {
            Ordering::Less => {
                // Done: filled circle
                iced::widget::text("●").size(10.0)
            }
            Ordering::Equal => {
                // Current: filled circle with primary color
                iced::widget::text("●").size(10.0).color(primary_color)
            }
            Ordering::Greater => {
                // Remaining: empty circle
                iced::widget::text("○").size(10.0)
            }
        };
        indicators = indicators.push(circle);
    }

    indicators.align_y(iced::Alignment::Center).into()
}

fn presentation_controls_view(_state: &AppState) -> Element<'_, Message> {
    let blink_button = create_icon_button(
        icon(Icon::Bell, ICON_SIZE_MEDIUM),
        "Blink",
        Message::SendCommand(toboggan_core::Command::Blink),
    );

    row![blink_button]
        .spacing(SPACING_SMALL)
        .align_y(iced::Alignment::Center)
        .into()
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    let connection_status = connection_status_view(&state.connection_status);
    let navigation_controls = navigation_controls_view();
    let presentation_controls = presentation_controls_view(state);
    let step_indicators = step_indicators_view(state);

    let slide_counter = if let Some((current, total)) = state.slide_index() {
        let counter_text = format!("Slide {current} / {total}");
        iced::widget::text(counter_text).size(12.0)
    } else {
        iced::widget::text("No slides").size(12.0)
    };

    let help_hint = iced::widget::text("Press 'h' for help")
        .size(11.0)
        .color(crate::constants::COLOR_MUTED);

    container(
        row![
            connection_status,
            container(
                row![
                    navigation_controls,
                    container(presentation_controls).padding(
                        iced::Padding::ZERO
                            .left(SPACING_MEDIUM)
                            .right(SPACING_MEDIUM)
                    )
                ]
                .spacing(SPACING_MEDIUM)
                .align_y(iced::Alignment::Center)
            )
            .width(Length::Fill)
            .center_x(Length::Fill),
            slide_counter,
            step_indicators,
            help_hint,
        ]
        .spacing(SPACING_MEDIUM)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .padding(PADDING_CONTAINER)
    .style(styles::footer_container())
    .into()
}
