use gpui::{
    App, Application, Bounds, KeyBinding, Menu, MenuItem, Point, Size, WindowBounds, WindowOptions,
    px,
};
use toboggan_editor::EditorApp;
use toboggan_editor::actions::{
    ExportTalk, NewPart, NewSlide, NewTalk, OpenTalk, Quit, Redo, SaveTalk, ToggleLeftPanel,
    ToggleRightPanel, Undo,
};

fn main() {
    tracing_subscriber::fmt::init();

    let app = Application::new();

    app.run(move |cx| {
        gpui_component::init(cx);
        cx.activate(true);

        setup_menus(cx);
        setup_keybindings(cx);

        let bounds = Bounds {
            origin: Point::default(),
            size: Size {
                width: px(1200.),
                height: px(800.),
            },
        };

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };

        #[allow(clippy::expect_used)] // Acceptable to panic in main if window fails
        cx.open_window(window_options, EditorApp::build)
            .expect("Failed to open window");
    });
}

/// Set up application menus
fn setup_menus(cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Talk", NewTalk),
                MenuItem::action("Open...", OpenTalk),
                MenuItem::separator(),
                MenuItem::action("Save", SaveTalk),
                MenuItem::action("Export...", ExportTalk),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", Undo),
                MenuItem::action("Redo", Redo),
                MenuItem::separator(),
                MenuItem::action("New Part", NewPart),
                MenuItem::action("New Slide", NewSlide),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Left Panel", ToggleLeftPanel),
                MenuItem::action("Toggle Right Panel", ToggleRightPanel),
            ],
        },
    ]);
}

/// Set up keyboard shortcuts
fn setup_keybindings(cx: &mut App) {
    cx.bind_keys([
        // File operations
        KeyBinding::new("cmd-n", NewTalk, None),
        KeyBinding::new("cmd-o", OpenTalk, None),
        KeyBinding::new("cmd-s", SaveTalk, None),
        KeyBinding::new("cmd-e", ExportTalk, None),
        KeyBinding::new("cmd-q", Quit, None),
        // Edit operations
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        // View operations
        KeyBinding::new("cmd-1", ToggleLeftPanel, None),
        KeyBinding::new("cmd-2", ToggleRightPanel, None),
    ]);
}
