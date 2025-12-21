use iced::widget::{container, row};
use iced::{Element, Length};
use toboggan_client::ConnectionStatus;

use crate::constants::{
    ICON_SIZE_MEDIUM, ICON_SIZE_SMALL, PADDING_CONTAINER, SPACING_MEDIUM, SPACING_SMALL,
};
use crate::icons::{
    icon_bell, icon_chevron_left, icon_chevron_right, icon_loader, icon_pause, icon_play,
    icon_refresh_cw, icon_skip_back, icon_skip_forward, icon_wifi, icon_wifi_off, icon_x,
};
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
            icon_wifi_off(ICON_SIZE_MEDIUM),
            "Disconnected",
            create_simple_button("Connect", Message::Connect).into(),
        )
        .into(),
        ConnectionStatus::Connecting => {
            create_status_row(icon_loader(ICON_SIZE_MEDIUM), "Connecting...").into()
        }
        ConnectionStatus::Connected => create_status_row_with_button(
            icon_wifi(ICON_SIZE_MEDIUM),
            "Connected",
            create_icon_button(
                icon_refresh_cw(ICON_SIZE_SMALL),
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
                icon_refresh_cw(ICON_SIZE_MEDIUM),
                iced::widget::text(reconnecting_text).size(12.0)
            ]
            .spacing(SPACING_SMALL)
            .align_y(iced::Alignment::Center)
        }
        .into(),
        ConnectionStatus::Error { message } => {
            let error_text = format!("Error: {message}");
            iced::widget::row![
                icon_x(ICON_SIZE_MEDIUM),
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
            icon_skip_back(ICON_SIZE_MEDIUM),
            "First",
            Message::SendCommand(toboggan_core::Command::First),
            NavButtonPosition::Leading
        ),
        create_nav_button(
            icon_chevron_left(ICON_SIZE_MEDIUM),
            "Previous",
            Message::SendCommand(toboggan_core::Command::Previous),
            NavButtonPosition::Leading
        ),
        create_nav_button(
            icon_chevron_right(ICON_SIZE_MEDIUM),
            "Next",
            Message::SendCommand(toboggan_core::Command::Next),
            NavButtonPosition::Trailing
        ),
        create_nav_button(
            icon_skip_forward(ICON_SIZE_MEDIUM),
            "Last",
            Message::SendCommand(toboggan_core::Command::Last),
            NavButtonPosition::Trailing
        ),
    ]
    .spacing(SPACING_SMALL)
    .align_y(iced::Alignment::Center)
    .into()
}

fn presentation_controls_view(state: &AppState) -> Element<'_, Message> {
    let pause_resume_button = match &state.presentation_state {
        Some(toboggan_core::State::Running { .. }) => {
            // Show pause button when presentation is running
            create_icon_button(
                icon_pause(ICON_SIZE_MEDIUM),
                "Pause",
                Message::SendCommand(toboggan_core::Command::Pause),
            )
        }
        Some(toboggan_core::State::Paused { .. }) => {
            // Show resume (play) button when presentation is paused
            create_icon_button(
                icon_play(ICON_SIZE_MEDIUM),
                "Resume",
                Message::SendCommand(toboggan_core::Command::Resume),
            )
        }
        _ => {
            // Default to pause button for Init/Done states
            create_icon_button(
                icon_pause(ICON_SIZE_MEDIUM),
                "Pause",
                Message::SendCommand(toboggan_core::Command::Pause),
            )
        }
    };

    let blink_button = create_icon_button(
        icon_bell(ICON_SIZE_MEDIUM),
        "Blink",
        Message::SendCommand(toboggan_core::Command::Blink),
    );

    row![pause_resume_button, blink_button]
        .spacing(SPACING_SMALL)
        .align_y(iced::Alignment::Center)
        .into()
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    let connection_status = connection_status_view(&state.connection_status);
    let navigation_controls = navigation_controls_view();
    let presentation_controls = presentation_controls_view(state);

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
