//! Main editor application

use chrono::{Datelike, NaiveDate};
use gpui::prelude::*;
use gpui::{
    AnyView, App, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, div, px,
};
use gpui_component::Root;
use gpui_component::button::ButtonVariants;
use gpui_component::calendar::{CalendarEvent, CalendarState, Date as CalendarDate};
use gpui_component::input::{InputEvent, InputState};
use gpui_component::resizable::{h_resizable, resizable_panel};
use gpui_component::theme::ActiveTheme;
use gpui_component::tree::TreeState;

use crate::actions::{
    ExportTalk, NewPart, NewSlide, NewTalk, OpenTalk, Quit, Redo, SaveTalk, SelectCover,
    SelectPart, SelectSlide, SelectTab, ToggleLeftPanel, ToggleRightPanel, ToggleSlideSkip, Undo,
};
use crate::state::{EditorState, Selection};
use crate::ui::panels::{
    EditorPanel, InspectorPanel, OutlinePanel, SlideInputs, build_tree_items, parse_tree_id,
};
use crate::ui::{StatusBar, Toolbar};

/// Main editor application
pub struct EditorApp {
    state: EditorState,
    focus_handle: FocusHandle,
    /// Input state for talk title
    title_input: Entity<InputState>,
    /// Input state for slide title
    slide_title_input: Entity<InputState>,
    /// Input state for slide body content
    slide_body_input: Entity<InputState>,
    /// Input state for slide notes
    slide_notes_input: Entity<InputState>,
    /// Input state for slide inline style (CSS)
    slide_style_input: Entity<InputState>,
    /// Tree state for outline panel
    tree_state: Entity<TreeState>,
    /// Calendar state for date picker
    calendar_state: Entity<CalendarState>,
}

impl EditorApp {
    #[allow(clippy::too_many_lines)]
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Create input states
        let title_input = cx.new(|cx| InputState::new(window, cx).placeholder("Talk title..."));
        let slide_title_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Slide title..."));
        let slide_body_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("markdown")
                .placeholder("Slide content (Markdown)...")
        });
        let slide_notes_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("markdown")
                .placeholder("Speaker notes (Markdown)...")
        });
        let slide_style_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("css")
                .placeholder("/* Custom CSS styles */")
        });

        // Create tree state for outline panel
        let tree_state = cx.new(|cx| TreeState::new(cx));

        // Create calendar state for date picker
        let calendar_state = cx.new(|cx| CalendarState::new(window, cx));

        // Subscribe to talk title changes
        cx.subscribe_in(
            &title_input,
            window,
            |this, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let new_title = this.title_input.read(cx).value().to_string();
                    let should_update = this
                        .state
                        .current_talk
                        .as_ref()
                        .is_some_and(|talk| talk.title != new_title);
                    if should_update {
                        this.state.record_state();
                        if let Some(talk) = &mut this.state.current_talk {
                            talk.title = new_title;
                        }
                    }
                }
            },
        )
        .detach();

        // Subscribe to slide title changes
        cx.subscribe_in(
            &slide_title_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let new_title = this.slide_title_input.read(cx).value().to_string();
                    if let Some(slide) = this.state.current_slide_mut()
                        && slide.title != new_title
                    {
                        this.state.record_state();
                        if let Some(slide) = this.state.current_slide_mut() {
                            slide.title = new_title;
                        }
                        // Update tree to reflect title change
                        this.sync_tree(window, cx);
                    }
                }
            },
        )
        .detach();

        // Subscribe to slide body changes
        cx.subscribe_in(
            &slide_body_input,
            window,
            |this, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let new_body = this.slide_body_input.read(cx).value().to_string();
                    if let Some(slide) = this.state.current_slide_mut()
                        && slide.body != new_body
                    {
                        this.state.record_state();
                        if let Some(slide) = this.state.current_slide_mut() {
                            slide.body = new_body;
                        }
                    }
                }
            },
        )
        .detach();

        // Subscribe to slide notes changes
        cx.subscribe_in(
            &slide_notes_input,
            window,
            |this, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let new_notes = this.slide_notes_input.read(cx).value().to_string();
                    if let Some(slide) = this.state.current_slide_mut()
                        && slide.notes != new_notes
                    {
                        this.state.record_state();
                        if let Some(slide) = this.state.current_slide_mut() {
                            slide.notes = new_notes;
                        }
                    }
                }
            },
        )
        .detach();

        // Subscribe to slide style changes
        cx.subscribe_in(
            &slide_style_input,
            window,
            |this, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let new_style = this.slide_style_input.read(cx).value().to_string();
                    let new_style_opt = if new_style.is_empty() {
                        None
                    } else {
                        Some(new_style)
                    };
                    if let Some(slide) = this.state.current_slide_mut()
                        && slide.inline_style != new_style_opt
                    {
                        this.state.record_state();
                        if let Some(slide) = this.state.current_slide_mut() {
                            slide.inline_style = new_style_opt;
                        }
                    }
                }
            },
        )
        .detach();

        // Subscribe to calendar date changes
        cx.subscribe_in(
            &calendar_state,
            window,
            |this, _, event: &CalendarEvent, _window, cx| {
                let CalendarEvent::Selected(date) = event;
                if let Some(naive_date) = date.start() {
                    // Convert chrono::NaiveDate to toboggan_core::Date
                    #[allow(clippy::cast_possible_truncation)]
                    let date_result = toboggan_core::Date::new(
                        naive_date.year() as i16,
                        naive_date.month() as i8,
                        naive_date.day() as i8,
                    );
                    if let Ok(new_date) = date_result {
                        let should_update = this
                            .state
                            .current_talk
                            .as_ref()
                            .is_some_and(|talk| talk.date != new_date);
                        if should_update {
                            this.state.record_state();
                            if let Some(talk) = &mut this.state.current_talk {
                                talk.date = new_date;
                            }
                            cx.notify();
                        }
                    }
                }
            },
        )
        .detach();

        Self {
            state: EditorState::new(),
            focus_handle: cx.focus_handle(),
            title_input,
            slide_title_input,
            slide_body_input,
            slide_notes_input,
            slide_style_input,
            tree_state,
            calendar_state,
        }
    }

    /// Build the editor app wrapped in a Root component
    pub fn build(window: &mut Window, cx: &mut App) -> Entity<Root> {
        let app = cx.new(|cx| Self::new(window, cx));
        let view: AnyView = app.into();
        cx.new(|cx| Root::new(view, window, cx))
    }

    /// Sync the title input with the current talk state
    fn sync_title_input(&self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self
            .state
            .current_talk
            .as_ref()
            .map_or(String::new(), |talk| talk.title.clone());

        self.title_input.update(cx, |input, cx| {
            let current = input.value().to_string();
            if current != title {
                input.set_value(title, window, cx);
            }
        });
    }

    /// Sync slide inputs with the currently selected slide
    fn sync_slide_inputs(&self, window: &mut Window, cx: &mut Context<Self>) {
        let (title, body, notes, style) = self.state.current_slide().map_or(
            (String::new(), String::new(), String::new(), String::new()),
            |slide| {
                (
                    slide.title.clone(),
                    slide.body.clone(),
                    slide.notes.clone(),
                    slide.inline_style.clone().unwrap_or_default(),
                )
            },
        );

        self.slide_title_input.update(cx, |input, cx| {
            let current = input.value().to_string();
            if current != title {
                input.set_value(title, window, cx);
            }
        });

        self.slide_body_input.update(cx, |input, cx| {
            let current = input.value().to_string();
            if current != body {
                input.set_value(body, window, cx);
            }
        });

        self.slide_notes_input.update(cx, |input, cx| {
            let current = input.value().to_string();
            if current != notes {
                input.set_value(notes, window, cx);
            }
        });

        self.slide_style_input.update(cx, |input, cx| {
            let current = input.value().to_string();
            if current != style {
                input.set_value(style, window, cx);
            }
        });
    }

    /// Sync tree state with current talk
    fn sync_tree(&self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(talk) = &self.state.current_talk {
            let items = build_tree_items(talk);
            self.tree_state.update(cx, |state, cx| {
                state.set_items(items, cx);
            });
        } else {
            self.tree_state.update(cx, |state, cx| {
                state.set_items(Vec::new(), cx);
            });
        }
    }

    /// Sync calendar state with current talk date
    fn sync_calendar(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(talk) = &self.state.current_talk {
            // Convert toboggan_core::Date to chrono::NaiveDate
            let date_str = talk.date.to_string();
            if let Ok(naive_date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                self.calendar_state.update(cx, |state, cx| {
                    state.set_date(CalendarDate::from(naive_date), window, cx);
                });
            }
        }
    }

    /// Handle tree selection change
    fn handle_tree_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selected_id = self
            .tree_state
            .read(cx)
            .selected_entry()
            .map(|entry| entry.item().id.to_string());

        if let Some(id) = selected_id
            && let Some(selection) = parse_tree_id(&id)
        {
            self.state.selection = selection;
            self.sync_slide_inputs(window, cx);
            cx.notify();
        }
    }

    // === Action Handlers ===

    /// Create a new talk
    fn handle_new_talk(&mut self, _: &NewTalk, window: &mut Window, cx: &mut Context<Self>) {
        let today = toboggan_core::Date::today();
        self.state.new_talk("Untitled".into(), today);
        self.sync_title_input(window, cx);
        self.sync_tree(window, cx);
        self.sync_calendar(window, cx);
        cx.notify();
    }

    /// Open an existing talk from file
    #[allow(clippy::unused_self)] // GPUI action handlers require &mut self
    fn handle_open_talk(&mut self, _: &OpenTalk, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async |this, cx| {
            // Show file dialog
            let file = rfd::AsyncFileDialog::new()
                .add_filter("TOML", &["toml"])
                .set_title("Open Talk")
                .pick_file()
                .await;

            if let Some(file) = file {
                let path = file.path().to_path_buf();
                let content = match std::fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(err) => {
                        tracing::error!("Failed to read file: {err}");
                        return;
                    }
                };

                let _ = this.update_in(cx, |this, window, cx| {
                    if this
                        .state
                        .load_talk(&content, path)
                        .inspect_err(|err| tracing::error!("Failed to parse talk: {err}"))
                        .is_ok()
                    {
                        this.sync_title_input(window, cx);
                        this.sync_tree(window, cx);
                        this.sync_calendar(window, cx);
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    /// Save the current talk
    fn handle_save_talk(&mut self, _: &SaveTalk, window: &mut Window, cx: &mut Context<Self>) {
        let Some(talk) = &self.state.current_talk else {
            return;
        };

        // Convert to core Talk for serialization
        let core_talk = talk.to_core();

        // Serialize to TOML
        let content = match toml::to_string_pretty(&core_talk) {
            Ok(content) => content,
            Err(err) => {
                tracing::error!("Failed to serialize talk: {err}");
                return;
            }
        };

        if let Some(path) = &self.state.file_path {
            // Save to existing path
            if let Err(err) = std::fs::write(path, &content) {
                tracing::error!("Failed to write file: {err}");
            } else {
                self.state.dirty = false;
                cx.notify();
            }
        } else {
            // Prompt for path using rfd
            cx.spawn_in(window, async move |this, cx| {
                let file = rfd::AsyncFileDialog::new()
                    .add_filter("TOML", &["toml"])
                    .set_title("Save Talk")
                    .set_file_name("talk.toml")
                    .save_file()
                    .await;

                if let Some(file) = file {
                    let path = file.path().to_path_buf();
                    if let Err(err) = std::fs::write(&path, &content) {
                        tracing::error!("Failed to write file: {err}");
                    } else {
                        let _ = this.update(cx, |this, cx| {
                            this.state.mark_saved(path);
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
        }
    }

    /// Export the talk
    #[allow(clippy::unused_self)] // GPUI action handlers require &mut self
    fn handle_export_talk(
        &mut self,
        _: &ExportTalk,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // TODO: Implement export functionality
        tracing::info!("Export not yet implemented");
    }

    /// Quit the application
    #[allow(clippy::unused_self)] // GPUI action handlers require &mut self
    fn handle_quit(&mut self, _: &Quit, _window: &mut Window, cx: &mut Context<Self>) {
        cx.quit();
    }

    /// Add a new part
    fn handle_new_part(&mut self, _: &NewPart, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.current_talk.is_some() {
            self.state.record_state();
            if let Some(talk) = &mut self.state.current_talk {
                let index = talk.add_part();
                self.state.selection = crate::state::Selection::Part { index };
            }
            self.sync_tree(window, cx);
            cx.notify();
        }
    }

    /// Add a new slide
    fn handle_new_slide(&mut self, _: &NewSlide, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.current_talk.is_none() {
            return;
        }

        self.state.record_state();

        // Get part index before borrowing talk mutably
        let part_index = self.state.selection.current_part_index();

        let Some(talk) = &mut self.state.current_talk else {
            return;
        };

        // Add slide to current part if in a part context, otherwise add as loose slide
        let slide_index = match part_index {
            Some(pi) => talk.add_slide_to_part(pi),
            None => Some(talk.add_loose_slide()),
        };

        if let Some(idx) = slide_index {
            self.state.selection = crate::state::Selection::Slide {
                part_index,
                slide_index: idx,
            };
        }

        self.sync_tree(window, cx);
        cx.notify();
    }

    /// Undo
    fn handle_undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        self.state.undo();
        self.sync_title_input(window, cx);
        self.sync_slide_inputs(window, cx);
        self.sync_tree(window, cx);
        self.sync_calendar(window, cx);
        cx.notify();
    }

    /// Redo
    fn handle_redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        self.state.redo();
        self.sync_title_input(window, cx);
        self.sync_slide_inputs(window, cx);
        self.sync_tree(window, cx);
        self.sync_calendar(window, cx);
        cx.notify();
    }

    /// Toggle left panel visibility
    fn handle_toggle_left_panel(
        &mut self,
        _: &ToggleLeftPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.show_left_panel = !self.state.show_left_panel;
        cx.notify();
    }

    /// Toggle right panel visibility
    fn handle_toggle_right_panel(
        &mut self,
        _: &ToggleRightPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.show_right_panel = !self.state.show_right_panel;
        cx.notify();
    }

    /// Select a tab in the center panel
    fn handle_select_tab(
        &mut self,
        action: &SelectTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.active_tab = action.0;
        cx.notify();
    }

    /// Toggle the skip property of the current slide
    fn handle_toggle_slide_skip(
        &mut self,
        _: &ToggleSlideSkip,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.record_state();
        if let Some(slide) = self.state.current_slide_mut() {
            slide.skip = !slide.skip;
        }
        cx.notify();
    }

    /// Select the cover/talk level
    fn handle_select_cover(
        &mut self,
        _: &SelectCover,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.selection = Selection::Cover;
        self.sync_slide_inputs(window, cx);
        cx.notify();
    }

    /// Select a part
    fn handle_select_part(
        &mut self,
        action: &SelectPart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.selection = Selection::Part { index: action.0 };
        self.sync_slide_inputs(window, cx);
        cx.notify();
    }

    /// Select a slide
    fn handle_select_slide(
        &mut self,
        action: &SelectSlide,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.selection = Selection::Slide {
            part_index: action.part_index,
            slide_index: action.slide_index,
        };
        self.sync_slide_inputs(window, cx);
        cx.notify();
    }

    // === Rendering ===

    fn render_welcome(cx: &Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().background)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(cx.theme().foreground)
                            .child("Toboggan Editor"),
                    )
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child("Create and edit presentations"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                gpui_component::button::Button::new("new-talk")
                                    .label("New Talk")
                                    .primary()
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(Box::new(NewTalk), cx);
                                    }),
                            )
                            .child(
                                gpui_component::button::Button::new("open-talk")
                                    .label("Open...")
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(Box::new(OpenTalk), cx);
                                    }),
                            ),
                    ),
            )
    }
}

impl Focusable for EditorApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_talk = self.state.current_talk.is_some();
        let show_left = self.state.show_left_panel;
        let show_right = self.state.show_right_panel;

        // Handle tree selection changes
        self.handle_tree_selection(window, cx);

        div()
            .id("editor-app")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            // Register action handlers
            .on_action(cx.listener(Self::handle_new_talk))
            .on_action(cx.listener(Self::handle_open_talk))
            .on_action(cx.listener(Self::handle_save_talk))
            .on_action(cx.listener(Self::handle_export_talk))
            .on_action(cx.listener(Self::handle_quit))
            .on_action(cx.listener(Self::handle_undo))
            .on_action(cx.listener(Self::handle_redo))
            .on_action(cx.listener(Self::handle_new_part))
            .on_action(cx.listener(Self::handle_new_slide))
            .on_action(cx.listener(Self::handle_toggle_left_panel))
            .on_action(cx.listener(Self::handle_toggle_right_panel))
            .on_action(cx.listener(Self::handle_select_tab))
            .on_action(cx.listener(Self::handle_toggle_slide_skip))
            .on_action(cx.listener(Self::handle_select_cover))
            .on_action(cx.listener(Self::handle_select_part))
            .on_action(cx.listener(Self::handle_select_slide))
            .child(Toolbar::new(&self.state, self.focus_handle.clone()))
            .child(if has_talk {
                div()
                    .flex_1()
                    .child(
                        h_resizable("main-layout")
                            .when(show_left, |this| {
                                this.child(
                                    resizable_panel()
                                        .size(px(250.))
                                        .size_range(px(200.)..px(400.))
                                        .child(OutlinePanel::new(self.tree_state.clone())),
                                )
                            })
                            .child(
                                resizable_panel()
                                    .size_range(px(400.)..gpui::Pixels::MAX)
                                    .child(EditorPanel::new(
                                        &self.state,
                                        self.focus_handle.clone(),
                                        SlideInputs {
                                            title: self.slide_title_input.clone(),
                                            body: self.slide_body_input.clone(),
                                            notes: self.slide_notes_input.clone(),
                                            style: self.slide_style_input.clone(),
                                        },
                                    )),
                            )
                            .when(show_right, |this| {
                                this.child(
                                    resizable_panel()
                                        .size(px(300.))
                                        .size_range(px(250.)..px(450.))
                                        .child(InspectorPanel::new(
                                            &self.state,
                                            self.focus_handle.clone(),
                                            self.title_input.clone(),
                                            self.calendar_state.clone(),
                                        )),
                                )
                            }),
                    )
                    .into_any_element()
            } else {
                Self::render_welcome(cx).into_any_element()
            })
            .child(StatusBar::new(&self.state))
    }
}
