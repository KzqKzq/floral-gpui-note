use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use gpui::AsyncApp;
use std::time::{Duration, Instant};

const HOTKEY_FLAG: AtomicBool = AtomicBool::new(false);
static GLOBAL_FLOAT_TIME: OnceLock<Instant> = OnceLock::new();
static GLOBAL_TILE_TIME: OnceLock<Instant> = OnceLock::new();

/// Call this in the main window's render to poll the flag
pub fn consume_hotkey_flag() -> bool {
    HOTKEY_FLAG.swap(false, Ordering::SeqCst)
}

/// Register a global handler for hotkey that opens notepad / tile windows
pub fn register_hotkey_handler(app: &gpui::Application) {
    // 这个函数在 main.rs 里调用
    let _ = app;
}
