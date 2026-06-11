mod app;
mod fs;
#[cfg(target_os = "windows")]
mod platform;
mod theme;

use app::FloralNotepad;
use gpui::*;
use gpui_component::{Root, TitleBar};

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    // ── Windows: register global hotkey Ctrl+Alt+N ──
    #[cfg(target_os = "windows")]
    let _hotkey = platform::HotkeyManager::register()
        .ok()
        .map(|mgr| Box::leak(Box::new(mgr)));

    app.run(|cx| {
        gpui_component::init(cx);

        let window_options = WindowOptions {
            titlebar: Some(TitleBar::title_bar_options()),
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1180.), px(760.)),
                cx,
            ))),
            window_min_size: Some(size(px(920.), px(620.))),
            window_background: WindowBackgroundAppearance::MicaBackdrop,
            ..Default::default()
        };

        cx.open_window(window_options, |window, cx| {
            let view = cx.new(|cx| FloralNotepad::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("failed to open app window");
    });
}
