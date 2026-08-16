#![allow(clippy::min_ident_chars)]

use gloo::console::error;
use toboggan_core::Theme;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

/// Current SGR state applied to subsequently printed characters
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
struct CellStyle {
    fg: Rgb,
    bg: Rgb,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    reverse: bool,
    strikethrough: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            fg: Rgb::WHITE,
            bg: Rgb::BLACK,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            reverse: false,
            strikethrough: false,
        }
    }
}

/// A cell in the virtual terminal grid
#[derive(Debug, Clone, Copy)]
struct Cell {
    ch: char,
    style: CellStyle,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: CellStyle::default(),
        }
    }
}

impl Cell {
    /// A blank space cell painted in the given default foreground/background.
    fn blank(fg: Rgb, bg: Rgb) -> Self {
        Self {
            style: CellStyle {
                fg,
                bg,
                ..CellStyle::default()
            },
            ..Self::default()
        }
    }
}

/// Builds a `rows × cols` grid of blank cells in the given default colors.
fn blank_grid(cols: u16, rows: u16, fg: Rgb, bg: Rgb) -> Vec<Vec<Cell>> {
    vec![vec![Cell::blank(fg, bg); cols as usize]; rows as usize]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rgb {
    red: u8,
    green: u8,
    blue: u8,
}

impl Rgb {
    const BLACK: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
    };
    const WHITE: Self = Self {
        red: 255,
        green: 255,
        blue: 255,
    };

    fn to_css(self) -> String {
        format!("rgb({},{},{})", self.red, self.green, self.blue)
    }
}

#[derive(Debug, Clone)]
struct SavedScreen {
    grid: Vec<Vec<Cell>>,
    cursor_row: u16,
    cursor_col: u16,
}

/// Dark theme ANSI color palette (Catppuccin Mocha-inspired)
const DARK_COLORS: [Rgb; 16] = [
    Rgb {
        red: 69,
        green: 71,
        blue: 90,
    }, // 0 black (surface1)
    Rgb {
        red: 243,
        green: 139,
        blue: 168,
    }, // 1 red
    Rgb {
        red: 166,
        green: 227,
        blue: 161,
    }, // 2 green
    Rgb {
        red: 249,
        green: 226,
        blue: 175,
    }, // 3 yellow
    Rgb {
        red: 137,
        green: 180,
        blue: 250,
    }, // 4 blue
    Rgb {
        red: 203,
        green: 166,
        blue: 247,
    }, // 5 magenta
    Rgb {
        red: 148,
        green: 226,
        blue: 213,
    }, // 6 cyan
    Rgb {
        red: 186,
        green: 194,
        blue: 222,
    }, // 7 white (subtext1)
    Rgb {
        red: 88,
        green: 91,
        blue: 112,
    }, // 8 bright black (surface2)
    Rgb {
        red: 243,
        green: 139,
        blue: 168,
    }, // 9 bright red
    Rgb {
        red: 166,
        green: 227,
        blue: 161,
    }, // 10 bright green
    Rgb {
        red: 249,
        green: 226,
        blue: 175,
    }, // 11 bright yellow
    Rgb {
        red: 137,
        green: 180,
        blue: 250,
    }, // 12 bright blue
    Rgb {
        red: 203,
        green: 166,
        blue: 247,
    }, // 13 bright magenta
    Rgb {
        red: 148,
        green: 226,
        blue: 213,
    }, // 14 bright cyan
    Rgb {
        red: 205,
        green: 214,
        blue: 244,
    }, // 15 bright white (text)
];

/// Light theme ANSI color palette (Catppuccin Latte-inspired)
const LIGHT_COLORS: [Rgb; 16] = [
    Rgb {
        red: 172,
        green: 176,
        blue: 190,
    }, // 0 black (surface2, #acb0be)
    Rgb {
        red: 210,
        green: 15,
        blue: 57,
    }, // 1 red
    Rgb {
        red: 64,
        green: 160,
        blue: 43,
    }, // 2 green
    Rgb {
        red: 223,
        green: 142,
        blue: 29,
    }, // 3 yellow
    Rgb {
        red: 30,
        green: 102,
        blue: 245,
    }, // 4 blue
    Rgb {
        red: 136,
        green: 57,
        blue: 239,
    }, // 5 magenta
    Rgb {
        red: 23,
        green: 146,
        blue: 153,
    }, // 6 cyan
    Rgb {
        red: 92,
        green: 95,
        blue: 119,
    }, // 7 white (subtext1)
    Rgb {
        red: 188,
        green: 192,
        blue: 204,
    }, // 8 bright black (surface1, #bcc0cc)
    Rgb {
        red: 210,
        green: 15,
        blue: 57,
    }, // 9 bright red
    Rgb {
        red: 64,
        green: 160,
        blue: 43,
    }, // 10 bright green
    Rgb {
        red: 223,
        green: 142,
        blue: 29,
    }, // 11 bright yellow
    Rgb {
        red: 30,
        green: 102,
        blue: 245,
    }, // 12 bright blue
    Rgb {
        red: 136,
        green: 57,
        blue: 239,
    }, // 13 bright magenta
    Rgb {
        red: 23,
        green: 146,
        blue: 153,
    }, // 14 bright cyan
    Rgb {
        red: 76,
        green: 79,
        blue: 105,
    }, // 15 bright white (text)
];

/// How many alternate-screen snapshots to keep. Nesting deeper than this is not
/// something real programs do; the oldest is dropped rather than growing forever.
const MAX_SAVED_SCREENS: usize = 8;

pub(super) struct VirtualTerminal {
    screen: TermScreen,
    parser: vte::Parser,
}

struct TermScreen {
    cols: u16,
    rows: u16,
    cursor_row: u16,
    cursor_col: u16,
    grid: Vec<Vec<Cell>>,
    /// Saved grids for alternate screen buffer save/restore (DECSET 47/1047/1049).
    ///
    /// Bounded by [`MAX_SAVED_SCREENS`]: a program that only ever enters the
    /// alternate screen (or a stream of `\e[?1049h` with no matching `l`) would
    /// otherwise push a full grid clone every time and grow without limit.
    saved_screens: Vec<SavedScreen>,
    /// Saved cursor position for DEC/ANSI save/restore
    saved_cursor: Option<(u16, u16)>,
    /// Scroll region: (top, bottom) inclusive, 0-indexed
    scroll_top: u16,
    scroll_bottom: u16,
    /// Whether to auto-wrap at end of line
    autowrap: bool,
    /// Deferred wrap: cursor hit the right edge, next print will wrap
    wrap_pending: bool,
    /// Window title set by OSC 0 or 2 escape sequences (OSC 1 sets the icon name,
    /// which this terminal does not display)
    title: Option<String>,
    /// Inside an OSC 8 hyperlink — suppress underline rendering
    in_hyperlink: bool,
    attrs: CellStyle,
    default_fg: Rgb,
    default_bg: Rgb,
    colors: [Rgb; 16],
}

impl VirtualTerminal {
    pub(super) fn new(cols: u16, rows: u16, theme: Theme) -> Self {
        Self {
            screen: TermScreen::new(cols, rows, theme),
            parser: vte::Parser::new(),
        }
    }

    pub(super) fn process(&mut self, data: &[u8]) {
        self.parser.advance(&mut self.screen, data);
    }

    pub(super) fn render_to_canvas(&self, canvas: &HtmlCanvasElement, font_size: f64) {
        self.screen.render_to_canvas(canvas, font_size);
    }

    pub(super) fn title(&self) -> Option<&str> {
        self.screen.title.as_deref()
    }

    pub(super) fn resize(&mut self, cols: u16, rows: u16) {
        self.screen.resize(cols, rows);
    }
}

/// Monospace font stack used for the terminal canvas. Shared so the render path
/// and the row/col sizing path (`compute_terminal_size`) measure the same font.
pub(super) const FONT_FAMILY: &str = "'JetBrainsMono Nerd Font Mono', 'MonaspiceNe Nerd Font', 'JetBrainsMono Nerd Font', 'Inconsolata', monospace";

/// Cell-width ratio (× font size) used when the font can't be measured yet.
pub(super) const FALLBACK_WIDTH_RATIO: f64 = 0.6;
/// Cell-height ratio (× font size) used when the font can't be measured yet.
pub(super) const FALLBACK_HEIGHT_RATIO: f64 = 1.3;

/// Stroke width (CSS px) for underline / strikethrough decorations.
const DECORATION_LINE_WIDTH: f64 = 1.0;
/// Distance (CSS px) up from the cell bottom at which the underline is drawn.
const UNDERLINE_INSET: f64 = 2.0;
/// Translucent fill used for the block cursor.
const CURSOR_FILL: &str = "rgba(200,200,200,0.5)";

/// Acquires the 2D rendering context for `canvas`, or `None` if unavailable.
fn canvas_2d(canvas: &HtmlCanvasElement) -> Option<web_sys::CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()
        .flatten()?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .ok()
}

/// CSI numeric parameter at `idx`, defaulting to `default` and clamped to at least
/// 1 — the `params.get(idx).unwrap_or(default).max(1)` idiom used across CSI dispatch.
fn param(params: &[u16], idx: usize, default: u16) -> u16 {
    params.get(idx).copied().unwrap_or(default).max(1)
}

/// 1-based CSI coordinate at `idx` (defaulting to 1) converted to a 0-based grid
/// index: `param(params, idx, 1) - 1`.
fn coord(params: &[u16], idx: usize) -> u16 {
    param(params, idx, 1) - 1
}

/// Cell layout metrics derived from a browser font measurement.
pub(super) struct CellMetrics {
    /// Advance width of one monospace cell, ceiled to whole pixels.
    pub(super) char_width: f64,
    /// Full line-box height of the cell (font ascent + descent), ceiled.
    pub(super) char_height: f64,
    /// Distance from the top of the cell down to the text baseline.
    pub(super) ascent: f64,
}

/// Measure monospace cell metrics for `font` using the canvas `TextMetrics`.
///
/// `char_width` is the advance of "M" (ceiled to avoid sub-pixel gaps between
/// cells). The vertical metrics come from the font bounding box so a full-height
/// glyph (e.g. a powerline separator, which fills the ascent→descent box) lands
/// exactly on one cell and lines up with the cell background — anchoring by a
/// fixed offset instead misregisters those glyphs against the background rect.
///
/// Falls back to width `font_size * 0.6` / height `font_size * 1.3` / ascent
/// `font_size` when the platform reports no vertical metrics (e.g. the web font
/// has not loaded yet); the next render (on WS output or resize) self-corrects.
/// Returns `None` only if the 2D context is unavailable.
pub(super) fn measure_cell_metrics(
    canvas: &HtmlCanvasElement,
    font: &str,
    font_size: f64,
) -> Option<CellMetrics> {
    let ctx = canvas_2d(canvas)?;
    ctx.set_font(font);
    let metrics = ctx.measure_text("M").ok();

    // Guard the measured width the same way the height below is guarded. A font
    // that has not loaded yet measures 0, and `avail_width / 0.0` is `inf`, which
    // `as u16` saturates to 65535 — a 65535-column grid is ~25 MB of cells and a
    // `resize` message the server has to honour.
    let char_width = metrics
        .as_ref()
        .map(web_sys::TextMetrics::width)
        .filter(|width| *width > 0.0)
        .unwrap_or(font_size * FALLBACK_WIDTH_RATIO)
        .ceil();

    let (line_height, ascent) = metrics
        .as_ref()
        .map(|m| (m.font_bounding_box_ascent(), m.font_bounding_box_descent()))
        .filter(|(ascent, descent)| ascent + descent > 0.0)
        .map_or(
            (font_size * FALLBACK_HEIGHT_RATIO, font_size),
            |(ascent, descent)| (ascent + descent, ascent),
        );

    Some(CellMetrics {
        char_width,
        char_height: line_height.ceil(),
        ascent,
    })
}

/// Measure cell metrics for `font_size` without an existing canvas, by creating a
/// detached one. Used by the sizing path so it derives rows/cols from the same
/// metrics the renderer uses. Returns `None` if a canvas or 2D context is
/// unavailable.
pub(super) fn cell_metrics_for(font_size: f64) -> Option<CellMetrics> {
    let canvas = gloo::utils::document()
        .create_element("canvas")
        .ok()?
        .dyn_into::<HtmlCanvasElement>()
        .ok()?;
    let font = format!("{font_size}px {FONT_FAMILY}");
    measure_cell_metrics(&canvas, &font, font_size)
}

impl TermScreen {
    #[allow(clippy::similar_names)]
    fn new(cols: u16, rows: u16, theme: Theme) -> Self {
        let is_light = theme == Theme::Light;
        let (default_fg, default_bg, colors) = if is_light {
            (
                Rgb {
                    red: 76,
                    green: 79,
                    blue: 105,
                }, // Catppuccin Latte text
                Rgb {
                    red: 239,
                    green: 241,
                    blue: 245,
                }, // Catppuccin Latte base
                LIGHT_COLORS,
            )
        } else {
            (
                Rgb {
                    red: 205,
                    green: 214,
                    blue: 244,
                }, // Catppuccin Mocha text
                Rgb {
                    red: 30,
                    green: 30,
                    blue: 46,
                }, // Catppuccin Mocha base
                DARK_COLORS,
            )
        };

        let attrs = CellStyle {
            fg: default_fg,
            bg: default_bg,
            ..CellStyle::default()
        };

        let grid = blank_grid(cols, rows, default_fg, default_bg);

        Self {
            cols,
            rows,
            cursor_row: 0,
            cursor_col: 0,
            grid,
            saved_screens: Vec::new(),
            saved_cursor: None,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            autowrap: true,
            wrap_pending: false,
            title: None,
            in_hyperlink: false,
            attrs,
            default_fg,
            default_bg,
            colors,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;

        let mut new_grid = blank_grid(cols, rows, self.default_fg, self.default_bg);

        // Copy existing content
        for (row_idx, row) in new_grid.iter_mut().enumerate() {
            let Some(old_row) = self.grid.get(row_idx) else {
                break;
            };
            for (col_idx, cell) in row.iter_mut().enumerate() {
                let Some(old_cell) = old_row.get(col_idx) else {
                    break;
                };
                *cell = *old_cell;
            }
        }

        self.grid = new_grid;
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn render_to_canvas(&self, canvas: &HtmlCanvasElement, font_size: f64) {
        let font_family = FONT_FAMILY;
        let font = format!("{font_size}px {font_family}");

        // Measure the cell box (width + font-metric line height + baseline) so
        // full-height glyphs align with the cell background.
        let Some(metrics) = measure_cell_metrics(canvas, &font, font_size) else {
            error!("Failed to measure terminal cell metrics");
            return;
        };
        let CellMetrics {
            char_width,
            char_height,
            ascent,
        } = metrics;

        // Logical (CSS px) grid size.
        let css_width = f64::from(self.cols) * char_width;
        let css_height = f64::from(self.rows) * char_height;

        // Size the backing store at device resolution so glyphs stay crisp on
        // HiDPI screens, and pin the CSS size so the browser does not rescale
        // (and blur) the canvas to fit its box.
        let dpr = web_sys::window()
            .map(|win| win.device_pixel_ratio())
            .filter(|ratio| *ratio > 0.0)
            .unwrap_or(1.0);
        canvas.set_width((css_width * dpr) as u32);
        canvas.set_height((css_height * dpr) as u32);
        let style = canvas.style();
        let _ = style.set_property("width", &format!("{css_width}px"));
        let _ = style.set_property("height", &format!("{css_height}px"));

        let Some(ctx) = canvas_2d(canvas) else {
            error!("Failed to get 2d context for rendering");
            return;
        };
        // Draw in logical pixels; the scale maps them onto the device-resolution
        // backing store. `set_width` above reset the transform, so this applies once.
        let _ = ctx.scale(dpr, dpr);

        // Fill entire canvas with default background
        ctx.set_fill_style_str(&self.default_bg.to_css());
        ctx.fill_rect(0.0, 0.0, css_width, css_height);

        // Pass 1: draw all cell backgrounds
        for (row_idx, row) in self.grid.iter().enumerate() {
            let row_y = row_idx as f64 * char_height;
            for (col_idx, cell) in row.iter().enumerate() {
                let col_x = col_idx as f64 * char_width;
                let bg = if cell.style.reverse {
                    cell.style.fg
                } else {
                    cell.style.bg
                };
                if bg != self.default_bg || cell.style.reverse {
                    ctx.set_fill_style_str(&bg.to_css());
                    ctx.fill_rect(col_x, row_y, char_width, char_height);
                }
            }
        }

        // Pass 2: draw all characters and decorations on top.
        // Anchor on the alphabetic baseline (row_y + ascent) so every glyph —
        // text or a full-cell powerline separator — fills its cell consistently
        // and lines up with the background rectangles from Pass 1.
        ctx.set_font(&font);
        ctx.set_text_baseline("alphabetic");
        let baseline_offset = ascent.round();

        for (row_idx, row) in self.grid.iter().enumerate() {
            let row_y = row_idx as f64 * char_height;
            for (col_idx, cell) in row.iter().enumerate() {
                if cell.ch == ' ' && !cell.style.reverse {
                    continue;
                }
                let col_x = col_idx as f64 * char_width;
                let fg = if cell.style.reverse {
                    cell.style.bg
                } else {
                    cell.style.fg
                };

                // Set font style
                let style = match (cell.style.bold, cell.style.italic) {
                    (true, true) => format!("bold italic {font_size}px {font_family}"),
                    (true, false) => format!("bold {font_size}px {font_family}"),
                    (false, true) => format!("italic {font_size}px {font_family}"),
                    (false, false) => font.clone(),
                };
                ctx.set_font(&style);

                // Foreground color (dim = half opacity)
                let fg_css = if cell.style.dim {
                    format!("rgba({},{},{},0.5)", fg.red, fg.green, fg.blue)
                } else {
                    fg.to_css()
                };
                ctx.set_fill_style_str(&fg_css);
                let _ = ctx.fill_text(&cell.ch.to_string(), col_x, row_y + baseline_offset);

                // Underline
                if cell.style.underline {
                    ctx.set_stroke_style_str(&fg.to_css());
                    ctx.set_line_width(DECORATION_LINE_WIDTH);
                    ctx.begin_path();
                    ctx.move_to(col_x, row_y + char_height - UNDERLINE_INSET);
                    ctx.line_to(col_x + char_width, row_y + char_height - UNDERLINE_INSET);
                    ctx.stroke();
                }

                // Strikethrough
                if cell.style.strikethrough {
                    ctx.set_stroke_style_str(&fg.to_css());
                    ctx.set_line_width(DECORATION_LINE_WIDTH);
                    ctx.begin_path();
                    ctx.move_to(col_x, row_y + char_height / 2.0);
                    ctx.line_to(col_x + char_width, row_y + char_height / 2.0);
                    ctx.stroke();
                }
            }
        }

        // Draw cursor
        let cursor_x = f64::from(self.cursor_col) * char_width;
        let cursor_y = f64::from(self.cursor_row) * char_height;
        ctx.set_fill_style_str(CURSOR_FILL);
        ctx.fill_rect(cursor_x, cursor_y, char_width, char_height);
    }

    fn put_char(&mut self, ch: char) {
        // If a wrap is pending from the previous character, do it now
        if self.wrap_pending {
            self.wrap_pending = false;
            if self.autowrap {
                self.cursor_col = 0;
                self.advance_row();
            }
        }

        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            let row = self.cursor_row as usize;
            let col = self.cursor_col as usize;
            if let Some(cell) = self.grid.get_mut(row).and_then(|r| r.get_mut(col)) {
                cell.style = self.attrs;
                cell.style.underline &= !self.in_hyperlink;
                cell.ch = ch;
            }
            self.cursor_col += 1;
            if self.cursor_col >= self.cols {
                if self.autowrap {
                    // Don't wrap yet — defer until next character is printed
                    self.cursor_col = self.cols.saturating_sub(1);
                    self.wrap_pending = true;
                } else {
                    self.cursor_col = self.cols.saturating_sub(1);
                }
            }
        }
    }

    fn advance_row(&mut self) {
        self.cursor_row += 1;
        if self.cursor_row > self.scroll_bottom {
            // Scroll up within scroll region
            self.scroll_up(1);
            self.cursor_row = self.scroll_bottom;
        }
    }

    fn scroll_up(&mut self, count: u16) {
        let top = self.scroll_top as usize;
        let bottom = self.scroll_bottom as usize;
        let default = self.default_cell();
        for _ in 0..count {
            if top < self.grid.len() && top <= bottom {
                self.grid.remove(top);
                let insert_at = bottom.min(self.grid.len());
                self.grid
                    .insert(insert_at, vec![default; self.cols as usize]);
            }
        }
    }

    fn scroll_down(&mut self, count: u16) {
        let top = self.scroll_top as usize;
        let bottom = self.scroll_bottom as usize;
        let default = self.default_cell();
        for _ in 0..count {
            if bottom < self.grid.len() && top <= bottom {
                self.grid.remove(bottom);
                self.grid.insert(top, vec![default; self.cols as usize]);
            }
        }
    }

    fn clear_row(&mut self, row: u16) {
        let default = self.default_cell();
        if let Some(grid_row) = self.grid.get_mut(row as usize) {
            for cell in grid_row.iter_mut() {
                *cell = default;
            }
        }
    }

    fn sgr_color(&self, idx: usize) -> Rgb {
        self.colors.get(idx).copied().unwrap_or(self.default_fg)
    }

    fn default_cell(&self) -> Cell {
        Cell::blank(self.default_fg, self.default_bg)
    }

    fn enter_alternate_screen(&mut self) {
        // Drop the oldest snapshot rather than growing without bound: each one is
        // a full grid clone, and a stream of `\e[?1049h` with no matching `l`
        // would otherwise consume memory for the life of the page.
        if self.saved_screens.len() >= MAX_SAVED_SCREENS {
            self.saved_screens.remove(0);
        }
        // Push current screen state onto the stack
        self.saved_screens.push(SavedScreen {
            grid: self.grid.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        });
        // Clear the screen for the alternate buffer
        let default = self.default_cell();
        for row in &mut self.grid {
            for cell in row.iter_mut() {
                *cell = default;
            }
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
    }

    fn leave_alternate_screen(&mut self) {
        if let Some(SavedScreen {
            grid,
            cursor_row,
            cursor_col,
        }) = self.saved_screens.pop()
        {
            self.grid = grid;
            self.cursor_row = cursor_row;
            self.cursor_col = cursor_col;
        }
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
    }

    fn reset_attrs(&mut self) {
        self.attrs.fg = self.default_fg;
        self.attrs.bg = self.default_bg;
        self.attrs.bold = false;
        self.attrs.dim = false;
        self.attrs.italic = false;
        self.attrs.underline = false;
        self.attrs.reverse = false;
        self.attrs.strikethrough = false;
    }

    fn handle_dec_set(&mut self, mode: u16) {
        match mode {
            7 => self.autowrap = true,
            47 | 1047 | 1049 => self.enter_alternate_screen(),
            _ => {} // Ignore: cursor visibility, mouse, focus, synced output, etc.
        }
    }

    fn handle_dec_reset(&mut self, mode: u16) {
        match mode {
            7 => self.autowrap = false,
            47 | 1047 | 1049 => self.leave_alternate_screen(),
            _ => {}
        }
    }

    fn erase_in_display(&mut self, mode: u16) {
        match mode {
            0 => {
                // Clear from cursor to end
                let default = self.default_cell();
                for col in self.cursor_col..self.cols {
                    if let Some(cell) = self
                        .grid
                        .get_mut(self.cursor_row as usize)
                        .and_then(|row| row.get_mut(col as usize))
                    {
                        *cell = default;
                    }
                }
                for row in (self.cursor_row + 1)..self.rows {
                    self.clear_row(row);
                }
            }
            1 => {
                // Clear from start to cursor
                let default = self.default_cell();
                for row in 0..self.cursor_row {
                    self.clear_row(row);
                }
                for col in 0..=self.cursor_col {
                    if let Some(cell) = self
                        .grid
                        .get_mut(self.cursor_row as usize)
                        .and_then(|row| row.get_mut(col as usize))
                    {
                        *cell = default;
                    }
                }
            }
            2 | 3 => {
                for row in 0..self.rows {
                    self.clear_row(row);
                }
            }
            _ => {}
        }
    }

    fn erase_in_line(&mut self, mode: u16) {
        let row = self.cursor_row as usize;
        let default = self.default_cell();
        match mode {
            0 => {
                if let Some(grid_row) = self.grid.get_mut(row) {
                    for cell in grid_row.iter_mut().skip(self.cursor_col as usize) {
                        *cell = default;
                    }
                }
            }
            1 => {
                if let Some(grid_row) = self.grid.get_mut(row) {
                    for cell in grid_row.iter_mut().take(self.cursor_col as usize + 1) {
                        *cell = default;
                    }
                }
            }
            2 => self.clear_row(self.cursor_row),
            _ => {}
        }
    }
}

impl vte::Perform for TermScreen {
    fn print(&mut self, ch: char) {
        self.put_char(ch);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.advance_row(),
            b'\r' => self.cursor_col = 0,
            0x08 => {
                // Backspace
                self.cursor_col = self.cursor_col.saturating_sub(1);
            }
            0x09 => {
                // Tab - advance to next 8-col boundary
                self.cursor_col = ((self.cursor_col / 8) + 1) * 8;
                if self.cursor_col >= self.cols {
                    self.cursor_col = self.cols.saturating_sub(1);
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_lines)]
    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params: Vec<u16> = params
            .iter()
            .map(|p| p.first().copied().unwrap_or(0))
            .collect();

        // Soft reset: \e[!p
        if intermediates.first() == Some(&b'!') && action == 'p' {
            self.reset_attrs();
            self.scroll_top = 0;
            self.scroll_bottom = self.rows.saturating_sub(1);
            self.autowrap = true;
            return;
        }

        // DEC private mode: \e[?...h / \e[?...l (can have multiple params)
        if intermediates.first() == Some(&b'?') {
            for &mode in &params {
                match action {
                    'h' => self.handle_dec_set(mode),
                    'l' => self.handle_dec_reset(mode),
                    _ => {}
                }
            }
            return;
        }

        // Cursor style: \e[N SP q — ignore (we don't change cursor shape)
        if intermediates.first() == Some(&b' ') && action == 'q' {
            return;
        }

        // Any explicit cursor movement clears the pending wrap
        if matches!(
            action,
            'A' | 'B' | 'C' | 'D' | 'd' | 'E' | 'F' | 'G' | 'H' | 'f' | 'r' | 's' | 'u'
        ) {
            self.wrap_pending = false;
        }

        match action {
            // Cursor Up
            'A' => {
                let count = param(&params, 0, 1);
                self.cursor_row = self.cursor_row.saturating_sub(count);
            }
            // Cursor Down
            'B' => {
                let count = param(&params, 0, 1);
                self.cursor_row = (self.cursor_row + count).min(self.rows.saturating_sub(1));
            }
            // Cursor Forward
            'C' => {
                let count = param(&params, 0, 1);
                self.cursor_col = (self.cursor_col + count).min(self.cols.saturating_sub(1));
            }
            // Cursor Back
            'D' => {
                let count = param(&params, 0, 1);
                self.cursor_col = self.cursor_col.saturating_sub(count);
            }
            // Vertical Position Absolute (VPA) - move to specific row
            'd' => {
                let row = coord(&params, 0);
                self.cursor_row = row.min(self.rows.saturating_sub(1));
            }
            // Cursor Next Line
            'E' => {
                let count = param(&params, 0, 1);
                self.cursor_row = (self.cursor_row + count).min(self.rows.saturating_sub(1));
                self.cursor_col = 0;
            }
            // Cursor Previous Line
            'F' => {
                let count = param(&params, 0, 1);
                self.cursor_row = self.cursor_row.saturating_sub(count);
                self.cursor_col = 0;
            }
            // Cursor Horizontal Absolute
            'G' => {
                let col = coord(&params, 0);
                self.cursor_col = col.min(self.cols.saturating_sub(1));
            }
            // Cursor Position (CUP)
            'H' | 'f' => {
                let row = coord(&params, 0);
                let col = coord(&params, 1);
                self.cursor_row = row.min(self.rows.saturating_sub(1));
                self.cursor_col = col.min(self.cols.saturating_sub(1));
            }
            // Erase in Display (ED)
            'J' => self.erase_in_display(params.first().copied().unwrap_or(0)),
            // Erase in Line (EL)
            'K' => self.erase_in_line(params.first().copied().unwrap_or(0)),
            // Insert Lines (within scroll region)
            'L' => {
                let count = param(&params, 0, 1) as usize;
                let row = self.cursor_row as usize;
                let bottom = self.scroll_bottom as usize;
                let default = self.default_cell();
                for _ in 0..count {
                    if row <= bottom && bottom < self.grid.len() {
                        self.grid.remove(bottom);
                        self.grid.insert(row, vec![default; self.cols as usize]);
                    }
                }
            }
            // Delete Lines (within scroll region)
            'M' => {
                let count = param(&params, 0, 1) as usize;
                let row = self.cursor_row as usize;
                let bottom = self.scroll_bottom as usize;
                let default = self.default_cell();
                for _ in 0..count {
                    if row <= bottom && row < self.grid.len() {
                        self.grid.remove(row);
                        let insert_at = bottom.min(self.grid.len());
                        self.grid
                            .insert(insert_at, vec![default; self.cols as usize]);
                    }
                }
            }
            // Insert Characters
            '@' => {
                let count = param(&params, 0, 1) as usize;
                let row = self.cursor_row as usize;
                let col = self.cursor_col as usize;
                let default = self.default_cell();
                if let Some(grid_row) = self.grid.get_mut(row) {
                    for _ in 0..count {
                        if col < grid_row.len() {
                            grid_row.insert(col, default);
                            grid_row.truncate(self.cols as usize);
                        }
                    }
                }
            }
            // Delete Characters
            'P' => {
                let count = param(&params, 0, 1) as usize;
                let row = self.cursor_row as usize;
                let col = self.cursor_col as usize;
                let default = self.default_cell();
                if let Some(grid_row) = self.grid.get_mut(row) {
                    for _ in 0..count {
                        if col < grid_row.len() {
                            grid_row.remove(col);
                            grid_row.push(default);
                        }
                    }
                }
            }
            // Set Scrolling Region (DECSTBM)
            'r' => {
                let top = coord(&params, 0);
                let bottom = param(&params, 1, self.rows) - 1;
                self.scroll_top = top.min(self.rows.saturating_sub(1));
                self.scroll_bottom = bottom.min(self.rows.saturating_sub(1));
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            // Scroll Up
            'S' => {
                let count = param(&params, 0, 1);
                self.scroll_up(count);
            }
            // Scroll Down
            'T' => {
                let count = param(&params, 0, 1);
                self.scroll_down(count);
            }
            // Erase Characters
            'X' => {
                let count = param(&params, 0, 1) as usize;
                let row = self.cursor_row as usize;
                let col = self.cursor_col as usize;
                let default = self.default_cell();
                if let Some(grid_row) = self.grid.get_mut(row) {
                    for idx in col..((col + count).min(grid_row.len())) {
                        if let Some(cell) = grid_row.get_mut(idx) {
                            *cell = default;
                        }
                    }
                }
            }
            // Tab backward
            'Z' => {
                let count = param(&params, 0, 1);
                for _ in 0..count {
                    self.cursor_col = if self.cursor_col >= 8 {
                        (self.cursor_col - 1) / 8 * 8
                    } else {
                        0
                    };
                }
            }
            // Cursor Save (ANSI.SYS)
            's' => {
                self.saved_cursor = Some((self.cursor_row, self.cursor_col));
            }
            // Cursor Restore (ANSI.SYS)
            'u' => {
                if let Some((row, col)) = self.saved_cursor {
                    self.cursor_row = row;
                    self.cursor_col = col;
                }
            }
            // Select Graphic Rendition (SGR)
            'm' => self.handle_sgr(&params),
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        // Handle charset selection: ESC ( 0 (line drawing) / ESC ( B (ASCII)
        if intermediates.first() == Some(&b'(') {
            // We ignore charset switching — just use Unicode characters directly
            return;
        }

        match byte {
            // DEC Save Cursor (DECSC)
            b'7' => {
                self.saved_cursor = Some((self.cursor_row, self.cursor_col));
            }
            // DEC Restore Cursor (DECRC)
            b'8' => {
                if let Some((row, col)) = self.saved_cursor {
                    self.cursor_row = row;
                    self.cursor_col = col;
                }
            }
            // Reverse Index (RI) - move cursor up, scroll down if at top of scroll region
            b'M' => {
                if self.cursor_row == self.scroll_top {
                    self.scroll_down(1);
                } else {
                    self.cursor_row = self.cursor_row.saturating_sub(1);
                }
            }
            // Index (IND) - move cursor down, scroll up if at bottom
            b'D' => {
                if self.cursor_row == self.scroll_bottom {
                    self.scroll_up(1);
                } else {
                    self.cursor_row = (self.cursor_row + 1).min(self.rows.saturating_sub(1));
                }
            }
            // Next Line (NEL)
            b'E' => {
                self.cursor_col = 0;
                if self.cursor_row == self.scroll_bottom {
                    self.scroll_up(1);
                } else {
                    self.cursor_row = (self.cursor_row + 1).min(self.rows.saturating_sub(1));
                }
            }
            _ => {}
        }
    }
    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
    }
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        match params.first().copied() {
            // OSC 0/2: set window title
            Some(b"0" | b"2") => {
                if let Some(title_bytes) = params.get(1) {
                    self.title = String::from_utf8(title_bytes.to_vec()).ok();
                }
            }
            // OSC 8: hyperlink — toggle in_hyperlink based on whether URI is present
            Some(b"8") => {
                // OSC 8 ; params ; uri ST — non-empty uri opens, empty uri closes
                let uri = params.get(2).copied().unwrap_or_default();
                self.in_hyperlink = !uri.is_empty();
            }
            _ => {}
        }
    }
}

impl TermScreen {
    #[allow(clippy::cast_possible_truncation)]
    fn handle_sgr(&mut self, params: &[u16]) {
        if params.is_empty() || (params.len() == 1 && params.first() == Some(&0)) {
            self.reset_attrs();
            return;
        }

        let mut iter = params.iter();
        while let Some(&param) = iter.next() {
            match param {
                0 => self.reset_attrs(),
                1 => self.attrs.bold = true,
                2 => self.attrs.dim = true,
                3 => self.attrs.italic = true,
                4 => self.attrs.underline = true,
                7 => self.attrs.reverse = true,
                9 => self.attrs.strikethrough = true,
                22 => {
                    self.attrs.bold = false;
                    self.attrs.dim = false;
                }
                23 => self.attrs.italic = false,
                24 => self.attrs.underline = false,
                27 => self.attrs.reverse = false,
                29 => self.attrs.strikethrough = false,
                // Standard foreground colors (30-37)
                30..=37 => self.attrs.fg = self.sgr_color((param - 30) as usize),
                39 => self.attrs.fg = self.default_fg,
                // Standard background colors (40-47)
                40..=47 => self.attrs.bg = self.sgr_color((param - 40) as usize),
                49 => self.attrs.bg = self.default_bg,
                // Bright foreground (90-97)
                90..=97 => self.attrs.fg = self.sgr_color((param - 90 + 8) as usize),
                // Bright background (100-107)
                100..=107 => self.attrs.bg = self.sgr_color((param - 100 + 8) as usize),
                // 256-color and truecolor
                38 => {
                    if let Some(&mode) = iter.next() {
                        if mode == 5 {
                            if let Some(&idx) = iter.next() {
                                self.attrs.fg = color_256(idx, &self.colors, self.default_fg);
                            }
                        } else if mode == 2 {
                            let red = iter.next().copied().unwrap_or(0) as u8;
                            let green = iter.next().copied().unwrap_or(0) as u8;
                            let blue = iter.next().copied().unwrap_or(0) as u8;
                            self.attrs.fg = Rgb { red, green, blue };
                        }
                    }
                }
                48 => {
                    if let Some(&mode) = iter.next() {
                        if mode == 5 {
                            if let Some(&idx) = iter.next() {
                                self.attrs.bg = color_256(idx, &self.colors, self.default_bg);
                            }
                        } else if mode == 2 {
                            let red = iter.next().copied().unwrap_or(0) as u8;
                            let green = iter.next().copied().unwrap_or(0) as u8;
                            let blue = iter.next().copied().unwrap_or(0) as u8;
                            self.attrs.bg = Rgb { red, green, blue };
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn color_256(idx: u16, base_colors: &[Rgb; 16], default: Rgb) -> Rgb {
    match idx {
        0..=15 => base_colors.get(idx as usize).copied().unwrap_or(default),
        16..=231 => {
            // 6x6x6 color cube
            let idx = idx - 16;
            let blue = (idx % 6) as u8 * 51;
            let green = ((idx / 6) % 6) as u8 * 51;
            let red = (idx / 36) as u8 * 51;
            Rgb { red, green, blue }
        }
        232..=255 => {
            // Grayscale ramp
            let level = (idx - 232) as u8 * 10 + 8;
            Rgb {
                red: level,
                green: level,
                blue: level,
            }
        }
        _ => default,
    }
}
