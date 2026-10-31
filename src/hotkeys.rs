use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

use crate::config::HotkeyConfig;

pub const HOTKEY_ID_CAPTURE_SCREEN: i32 = 1001;
pub const HOTKEY_ID_ANALYZE_CLIPBOARD: i32 = 1002;
pub const HOTKEY_ID_TOGGLE_VISIBILITY: i32 = 1003;
pub const HOTKEY_ID_SCROLL_UP: i32 = 1004;
pub const HOTKEY_ID_SCROLL_DOWN: i32 = 1005;
pub const HOTKEY_ID_OPACITY_DEC: i32 = 1006;
pub const HOTKEY_ID_OPACITY_INC: i32 = 1007;
pub const HOTKEY_ID_PANIC_EXIT: i32 = 1008;

pub const HOTKEY_ID_TOGGLE_FOLLOW: i32 = 1009;
pub const HOTKEY_ID_CYCLE_PRESETS: i32 = 1010;
pub const HOTKEY_ID_WIDTH_DEC: i32 = 1011;
pub const HOTKEY_ID_WIDTH_INC: i32 = 1012;
pub const HOTKEY_ID_HEIGHT_DEC: i32 = 1013;
pub const HOTKEY_ID_HEIGHT_INC: i32 = 1014;
pub const HOTKEY_ID_CLEAR_HISTORY: i32 = 1016;

pub struct HotkeyManager {
    hwnd: HWND,
    registered_ids: Vec<i32>,
}

impl HotkeyManager {
    pub fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            registered_ids: Vec::new(),
        }
    }

    pub fn register_all(&mut self, cfg: &HotkeyConfig) -> Result<(), String> {
        let mod_alt = MOD_ALT | MOD_NOREPEAT;
        let mod_alt_shift = MOD_ALT | MOD_SHIFT | MOD_NOREPEAT;

        self.register(HOTKEY_ID_CAPTURE_SCREEN, mod_alt, &cfg.capture_screen, "Alt+Q")?;
        self.register(HOTKEY_ID_ANALYZE_CLIPBOARD, mod_alt, &cfg.analyze_clipboard, "Alt+W")?;
        self.register(HOTKEY_ID_TOGGLE_VISIBILITY, mod_alt, &cfg.toggle_visibility, "Alt+S")?;
        self.register(HOTKEY_ID_SCROLL_UP, mod_alt, &cfg.scroll_up, "Alt+Up")?;
        self.register(HOTKEY_ID_SCROLL_DOWN, mod_alt, &cfg.scroll_down, "Alt+Down")?;
        self.register(HOTKEY_ID_OPACITY_DEC, mod_alt, &cfg.opacity_decrease, "Alt+Left")?;
        self.register(HOTKEY_ID_OPACITY_INC, mod_alt, &cfg.opacity_increase, "Alt+Right")?;
        self.register(HOTKEY_ID_PANIC_EXIT, mod_alt_shift, &cfg.panic_exit, "Alt+Shift+X")?;

        self.register(HOTKEY_ID_TOGGLE_FOLLOW, mod_alt, &cfg.toggle_follow, "Alt+F")?;
        self.register(HOTKEY_ID_CYCLE_PRESETS, mod_alt, &cfg.cycle_presets, "Alt+M")?;
        self.register(HOTKEY_ID_WIDTH_DEC, mod_alt, &cfg.width_decrease, "Alt+[")?;
        self.register(HOTKEY_ID_WIDTH_INC, mod_alt, &cfg.width_increase, "Alt+]")?;
        self.register(HOTKEY_ID_HEIGHT_DEC, mod_alt, &cfg.height_decrease, "Alt+-")?;
        self.register(HOTKEY_ID_HEIGHT_INC, mod_alt, &cfg.height_increase, "Alt+=")?;
        self.register(HOTKEY_ID_CLEAR_HISTORY, mod_alt, &cfg.clear_history, "Alt+C")?;

        Ok(())
    }

    fn register(
        &mut self,
        id: i32,
        modifiers: HOT_KEY_MODIFIERS,
        key_str: &str,
        label: &str,
    ) -> Result<(), String> {
        let vk = parse_key(key_str);
        unsafe {
            if RegisterHotKey(Some(self.hwnd), id, modifiers, vk).is_ok() {
                self.registered_ids.push(id);
                Ok(())
            } else {
                eprintln!("[WARNING] Could not register {}: key in use by another app", label);
                Ok(())
            }
        }
    }

    pub fn unregister_all(&mut self) {
        unsafe {
            for &id in &self.registered_ids {
                let _ = UnregisterHotKey(Some(self.hwnd), id);
            }
            self.registered_ids.clear();
        }
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.unregister_all();
    }
}

fn parse_key(s: &str) -> u32 {
    let lower = s.trim().to_lowercase();
    match lower.as_str() {
        "up" => VK_UP.0 as u32,
        "down" => VK_DOWN.0 as u32,
        "left" => VK_LEFT.0 as u32,
        "right" => VK_RIGHT.0 as u32,
        "pageup" | "pgup" => VK_PRIOR.0 as u32,
        "pagedown" | "pgdown" => VK_NEXT.0 as u32,
        "space" => VK_SPACE.0 as u32,
        "enter" | "return" => VK_RETURN.0 as u32,
        "esc" | "escape" => VK_ESCAPE.0 as u32,
        "[" | "bracket_left" => 0xDB,       // VK_OEM_4
        "]" | "bracket_right" => 0xDD,      // VK_OEM_6
        "-" | "minus" => 0xBD,              // VK_OEM_MINUS
        "=" | "+" | "plus" | "equals" => 0xBB, // VK_OEM_PLUS
        other => {
            if let Some(ch) = other.chars().next() {
                if ch.is_ascii_alphabetic() {
                    ch.to_ascii_uppercase() as u32
                } else if ch.is_ascii_digit() {
                    ch as u32
                } else {
                    b'Q' as u32
                }
            } else {
                b'Q' as u32
            }
        }
    }
}
