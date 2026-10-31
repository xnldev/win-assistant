#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod capture;
mod clipboard;
mod config;
mod hotkeys;
mod markdown;
mod render;
mod window;

use config::Config;
use window::AssistantWindow;
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG};

fn main() {
    let config = Config::load_or_create().unwrap_or_default();

    let _win = match AssistantWindow::create_and_show(config) {
        Ok(w) => w,
        Err(err) => {
            eprintln!("Failed to start Win-Assistant: {}", err);
            return;
        }
    };

    println!("Win-Assistant ready.");
    println!("Alt+Q: Scan | Alt+W: Clipboard | Alt+F: Follow | Alt+S: Hide | Alt+Shift+X: Exit");

    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
