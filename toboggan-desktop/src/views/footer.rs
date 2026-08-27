use iced::widget::{Column, container, pick_list, progress_bar, row, text};
use iced::{Element, Length};
use toboggan_client::ConnectionStatus;
use toboggan_core::ClientRole;
use toboggan_core::pacing::{format_duration, wall_clock};

use crate::constants::{
    FONT_SIZE_LARGE, FONT_SIZE_MEDIUM, FONT_SIZE_SMALL, ICON_SIZE_MEDIUM, ICON_SIZE_SMALL,
    PADDING_CONTAINER, PADDING_SMALL, SPACING_MEDIUM, SPACING_SMALL,
};
use crate::icons::{Icon, icon};
use crate::message::Message;
use crate::state::{AppState, ThemeChoice};
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
        // Labelled for what it does. It said "Reconnect" while sending
        // `Disconnect`, and was only ever right because `handle_disconnect`
        // reconnected 100 ms later behind its back — which also meant a client
        // that should stand down could not.
        ConnectionStatus::Connected => create_status_row_with_button(
            icon(Icon::Wifi, ICON_SIZE_MEDIUM),
            "Connected",
            create_icon_button(
                icon(Icon::WifiOff, ICON_SIZE_SMALL),
                "Disconnect",
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
            row![
                icon(Icon::RefreshCw, ICON_SIZE_MEDIUM),
                text(reconnecting_text).size(12.0)
            ]
            .spacing(SPACING_SMALL)
            .align_y(iced::Alignment::Center)
        }
        .into(),
        ConnectionStatus::Error { message } => {
            let error_text = format!("Error: {message}");
            row![
                icon(Icon::X, ICON_SIZE_MEDIUM),
                text(error_text).size(12.0),
                iced::widget::button(text("Retry").size(11.0))
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
        return text("").into();
    };

    if step_count == 0 {
        return text("").into();
    }

    // Styled with the theme-aware `text::*` functions rather than a colour read
    // off `Theme::Dark`: the closure is handed the live theme, so the dots
    // follow it instead of pinning this one readout to the dark palette.
    let mut indicators = row![].spacing(2.0);
    for step in 0..step_count {
        let circle = match step.cmp(&current_step) {
            // Done.
            Ordering::Less => text("●").size(10.0).style(text::secondary),
            // Where the deck is now.
            Ordering::Equal => text("●").size(10.0).style(text::primary),
            // Still to come.
            Ordering::Greater => text("○").size(10.0).style(text::secondary),
        };
        indicators = indicators.push(circle);
    }

    indicators.align_y(iced::Alignment::Center).into()
}

/// The clock, the talk's own timer and its controls.
///
/// A presenter tool without a sense of time was the largest thing this client
/// lacked: the tick that drives this has been firing once a second since it was
/// written, into an arm that discarded it.
fn timer_view(state: &AppState) -> Element<'_, Message> {
    let running = state.elapsed.is_running();
    let toggle = create_icon_button(
        icon(
            if running { Icon::Pause } else { Icon::Play },
            ICON_SIZE_SMALL,
        ),
        if running { "Pause" } else { "Resume" },
        Message::ToggleTimer,
    )
    .style(iced::widget::button::secondary);

    let reset = create_icon_button(
        icon(Icon::RotateCcw, ICON_SIZE_SMALL),
        "Reset",
        Message::ResetTimer,
    )
    .style(iced::widget::button::secondary);

    row![
        icon(Icon::Clock, ICON_SIZE_SMALL),
        text(wall_clock()).size(FONT_SIZE_SMALL),
        icon(Icon::Timer, ICON_SIZE_SMALL),
        text(format_duration(state.elapsed_secs())).size(FONT_SIZE_LARGE),
        toggle,
        reset,
    ]
    .spacing(SPACING_SMALL)
    .align_y(iced::Alignment::Center)
    .into()
}

/// How far ahead of or behind the deck's plan the talk is running.
///
/// Nothing at all when the deck declares no `duration` front matter: a readout
/// that appears anyway would be measuring against a schedule the speaker never
/// set, which is worse than no readout.
fn pacing_view(state: &AppState) -> Element<'_, Message> {
    let Some(drift) = state.drift_secs() else {
        return text("").into();
    };
    let magnitude = format_duration(drift.unsigned_abs());
    // `text::success` and `text::danger` are distinct function items, so they
    // cannot share a binding — the whole element branches instead.
    if drift < 0 {
        text(format!("\u{2212}{magnitude}"))
            .size(FONT_SIZE_MEDIUM)
            .style(text::success)
            .into()
    } else {
        text(format!("+{magnitude}"))
            .size(FONT_SIZE_MEDIUM)
            .style(text::danger)
            .into()
    }
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

pub(super) fn view(state: &AppState) -> Element<'_, Message> {
    let connection_status = connection_status_view(&state.connection_status);
    let navigation_controls = navigation_controls_view();
    let presentation_controls = presentation_controls_view(state);
    let step_indicators = step_indicators_view(state);

    let slide_counter = if let Some((current, total)) = state.slide_index() {
        let counter_text = format!("Slide {current} / {total}");
        text(counter_text).size(12.0)
    } else {
        text("No slides").size(12.0)
    };

    let theme_picker = pick_list(
        ThemeChoice::ALL,
        Some(state.theme_choice),
        Message::ThemeChosen,
    )
    .text_size(FONT_SIZE_SMALL)
    .padding(PADDING_SMALL);

    let help_hint = text("Press 'h' for help")
        .size(11.0)
        .color(crate::constants::COLOR_MUTED);

    // Said plainly, because the alternative is discovering it by pressing an
    // arrow key and having the server refuse.
    let role_hint = match state.role {
        Some(ClientRole::Audience) => text("Watching — this client cannot present")
            .size(11.0)
            .color(crate::constants::COLOR_MUTED),
        _ => text("").size(11.0),
    };

    // A bar above the strip rather than a number inside it: "Slide 5 / 42" says
    // where you are, and this says how much is left, which is the thing a
    // speaker deciding whether to expand on a point is actually asking.
    let progress = {
        #[allow(clippy::cast_precision_loss)]
        let fraction = state.slide_index().map_or(0.0, |(current, total)| {
            if total == 0 {
                0.0
            } else {
                current.min(total) as f32 / total as f32
            }
        });
        // `girth` is the thin axis; `length` is the one it runs along.
        progress_bar(0.0..=1.0, fraction).girth(4.0)
    };

    let strip = container(
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
            timer_view(state),
            pacing_view(state),
            theme_picker,
            help_hint,
            role_hint,
        ]
        .spacing(SPACING_MEDIUM)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .padding(PADDING_CONTAINER)
    .style(styles::footer_container());

    Column::with_children(vec![progress.into(), strip.into()])
        .width(Length::Fill)
        .into()
}
