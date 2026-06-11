//! Windows platform features: global hotkeys.
#![cfg(target_os = "windows")]

use anyhow::Result;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use std::sync::atomic::{AtomicBool, Ordering};

/// A simple flag that gets set to `true` when the global hotkey (Ctrl+Alt+N) is pressed.
pub static HOTKEY_FLAG: AtomicBool = AtomicBool::new(false);

pub struct HotkeyManager {
    _manager: GlobalHotKeyManager,
}

impl HotkeyManager {
    /// Register Ctrl+Alt+N global hotkey. When pressed, sets `HOTKEY_FLAG` to true.
    pub fn register() -> Result<Self> {
        let manager =
            GlobalHotKeyManager::new().map_err(|e| anyhow::anyhow!("全局热键注册失败: {e}"))?;

        let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyN);
        manager
            .register(hotkey)
            .map_err(|e| anyhow::anyhow!("全局热键注册失败: {e}"))?;

        GlobalHotKeyEvent::set_event_handler(Some(move |_event| {
            HOTKEY_FLAG.store(true, Ordering::SeqCst);
        }));

        Ok(Self { _manager: manager })
    }
}
