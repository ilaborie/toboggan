use crate::config::display::{
    COLOR_BLACK, COLOR_CYAN, COLOR_ERROR_BACKGROUND, COLOR_GREEN, COLOR_ORANGE, COLOR_RED,
    COLOR_WHITE, COLOR_YELLOW, CURRENT_SLIDE_LINE_START, CURRENT_SLIDE_MAX_LINES, LINE_HEIGHT,
    MAX_CHARS_PER_LINE, NEXT_SLIDE_LINE_START, NEXT_SLIDE_MAX_LINES, TITLE_LINE_START,
    TITLE_MAX_LINES,
};
use crate::state::{AppState, StateMode, TalkData};
use embedded_graphics::{
    mono_font::{
        ascii::{FONT_8X13, FONT_8X13_BOLD},
        MonoTextStyle,
    },
    pixelcolor::Rgb565,
    prelude::*,
    text::{Alignment, Text},
};
use log::info;

pub struct DisplayManager<D>
where
    D: DrawTarget<Color = Rgb565>,
{
    pub display: D,
    current_state_hash: u64,
}

impl<D> DisplayManager<D>
where
    D: DrawTarget<Color = Rgb565>,
{
    /// Create a new display manager with the given display
    ///
    /// # Errors
    /// Returns error if display initialization fails
    pub fn new(mut display: D) -> anyhow::Result<Self> {
        display
            .clear(COLOR_BLACK)
            .map_err(|_| anyhow::anyhow!("Failed to clear display"))?;

        Ok(Self {
            display,
            current_state_hash: 0,
        })
    }

    /// Update the display based on the application state and talk data
    ///
    /// # Errors
    /// Returns error if display rendering fails
    pub fn update_display(
        &mut self,
        state: &AppState,
        talk_data: Option<&TalkData>,
    ) -> anyhow::Result<()> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Calculate hash of current state and talk data to detect changes
        let mut hasher = DefaultHasher::new();
        state.hash(&mut hasher);
        if let Some(data) = talk_data {
            data.hash(&mut hasher);
        }
        let new_state_hash = hasher.finish();

        if self.current_state_hash != new_state_hash {
            info!("Updating display for state: {state:?}");
            self.current_state_hash = new_state_hash;
            self.render_state(state, talk_data)?;
        }

        Ok(())
    }

    /// Render the complete state-based display layout
    fn render_state(
        &mut self,
        state: &AppState,
        talk_data: Option<&TalkData>,
    ) -> anyhow::Result<()> {
        // Clear display
        let background_color = match state {
            AppState::Error { .. } => COLOR_ERROR_BACKGROUND,
            _ => COLOR_BLACK,
        };

        self.display
            .clear(background_color)
            .map_err(|_| anyhow::anyhow!("Failed to clear display"))?;

        match state {
            AppState::Play { current, mode } => {
                if let Some(data) = talk_data {
                    self.render_play_state(data, *current, *mode)?;
                } else {
                    self.render_simple_state("No talk data", COLOR_RED, TITLE_LINE_START + 3)?;
                }
            }
            AppState::Initialized => {
                if let Some(data) = talk_data {
                    self.render_simple_state("Ready", COLOR_CYAN, TITLE_LINE_START)?;
                    self.render_title(&data.title, COLOR_CYAN)?;
                } else {
                    self.render_simple_state("Loading...", COLOR_YELLOW, TITLE_LINE_START + 3)?;
                }
            }
            AppState::Error { message } => {
                self.render_simple_state("ERROR", COLOR_RED, TITLE_LINE_START)?;
                let error_lines = Self::wrap_text(message, MAX_CHARS_PER_LINE);
                self.draw_text_at_lines(
                    &error_lines,
                    CURRENT_SLIDE_LINE_START,
                    COLOR_RED,
                    &FONT_8X13,
                )?;
            }
            _ => {
                let (status_text, color) = match state {
                    AppState::Booting => ("Booting...", COLOR_ORANGE),
                    AppState::Connecting { ssid } => {
                        self.render_simple_state("Connecting", COLOR_YELLOW, TITLE_LINE_START)?;
                        let ssid_lines = Self::wrap_text(ssid, MAX_CHARS_PER_LINE);
                        self.draw_text_at_lines(
                            &ssid_lines,
                            CURRENT_SLIDE_LINE_START,
                            COLOR_YELLOW,
                            &FONT_8X13,
                        )?;
                        return Ok(());
                    }
                    AppState::Connected { ssid } => {
                        self.render_simple_state("Connected", COLOR_GREEN, TITLE_LINE_START)?;
                        let ssid_lines = Self::wrap_text(ssid, MAX_CHARS_PER_LINE);
                        self.draw_text_at_lines(
                            &ssid_lines,
                            CURRENT_SLIDE_LINE_START,
                            COLOR_WHITE,
                            &FONT_8X13,
                        )?;
                        return Ok(());
                    }
                    AppState::Loading => ("Loading talk...", COLOR_WHITE),
                    _ => unreachable!(),
                };
                self.render_simple_state(status_text, color, TITLE_LINE_START + 3)?;
            }
        }

        Ok(())
    }

    /// Render the play state with title, current slide, and next slide
    fn render_play_state(
        &mut self,
        talk_data: &TalkData,
        current: usize,
        mode: StateMode,
    ) -> anyhow::Result<()> {
        // Determine title color based on play mode
        let title_color = match mode {
            StateMode::Paused => COLOR_YELLOW,
            StateMode::Running => COLOR_GREEN,
            StateMode::Done => COLOR_CYAN,
        };

        // Render talk title (lines 2-3)
        self.render_title(&talk_data.title, title_color)?;

        // Render current slide (lines 7-11, bold)
        if let Some(current_slide) = talk_data.get_slide(current) {
            let current_lines = Self::wrap_text(current_slide, MAX_CHARS_PER_LINE);
            let limited_lines: Vec<_> = current_lines
                .into_iter()
                .take(CURRENT_SLIDE_MAX_LINES)
                .collect();
            self.draw_text_at_lines(
                &limited_lines,
                CURRENT_SLIDE_LINE_START,
                COLOR_WHITE,
                &FONT_8X13_BOLD,
            )?;
        }

        // Render next slide (lines 13-14) if available
        if let Some(next_slide) = talk_data.get_next_slide(current) {
            let next_lines = Self::wrap_text(next_slide, MAX_CHARS_PER_LINE);
            let limited_lines: Vec<_> = next_lines.into_iter().take(NEXT_SLIDE_MAX_LINES).collect();
            self.draw_text_at_lines(
                &limited_lines,
                NEXT_SLIDE_LINE_START,
                COLOR_CYAN,
                &FONT_8X13,
            )?;
        }

        Ok(())
    }

    /// Render a simple centered status message
    fn render_simple_state(&mut self, text: &str, color: Rgb565, line: i32) -> anyhow::Result<()> {
        let display_size = self.display.bounding_box().size;
        let center_x = i32::try_from(display_size.width).unwrap_or(0) / 2;
        let y_pos = line * LINE_HEIGHT;

        let text_style = MonoTextStyle::new(&FONT_8X13, color);
        Text::with_alignment(
            text,
            Point::new(center_x, y_pos),
            text_style,
            Alignment::Center,
        )
        .draw(&mut self.display)
        .map_err(|_| anyhow::anyhow!("Failed to draw status text"))?;

        Ok(())
    }

    /// Render the talk title with proper wrapping
    fn render_title(&mut self, title: &str, color: Rgb565) -> anyhow::Result<()> {
        let title_lines = Self::wrap_text(title, MAX_CHARS_PER_LINE);
        let limited_lines: Vec<_> = title_lines.into_iter().take(TITLE_MAX_LINES).collect();
        self.draw_text_at_lines(&limited_lines, TITLE_LINE_START, color, &FONT_8X13)
    }

    /// Draw text lines at specific line positions
    fn draw_text_at_lines(
        &mut self,
        lines: &[String],
        start_line: i32,
        color: Rgb565,
        font: &embedded_graphics::mono_font::MonoFont,
    ) -> anyhow::Result<()> {
        let display_size = self.display.bounding_box().size;
        let center_x = i32::try_from(display_size.width).unwrap_or(0) / 2;
        let text_style = MonoTextStyle::new(font, color);

        for (index, line) in lines.iter().enumerate() {
            let y_pos = (start_line + i32::try_from(index).unwrap_or(0)) * LINE_HEIGHT;
            Text::with_alignment(
                line,
                Point::new(center_x, y_pos),
                text_style,
                Alignment::Center,
            )
            .draw(&mut self.display)
            .map_err(|_| anyhow::anyhow!("Failed to draw text line '{line}'"))?;
        }

        Ok(())
    }

    /// Wrap text to fit within specified character width
    fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
        if text.len() <= max_chars {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= max_chars {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }
}
