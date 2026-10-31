use windows::Win32::Foundation::*;
use windows::Win32::System::DataExchange::*;
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};

const CF_UNICODETEXT: u32 = 13;

/// Silently reads text from the Windows Clipboard
pub fn get_clipboard_text() -> Result<String, String> {
    unsafe {
        if OpenClipboard(None).is_err() {
            return Err("Failed to open clipboard".to_string());
        }

        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.is_err() {
            let _ = CloseClipboard();
            return Err("No text available in clipboard".to_string());
        }

        let handle = handle.unwrap();
        let ptr = GlobalLock(HGLOBAL(handle.0));
        if ptr.is_null() {
            let _ = CloseClipboard();
            return Err("Failed to lock clipboard memory".to_string());
        }

        let wide_slice = {
            let mut len = 0;
            let u16_ptr = ptr as *const u16;
            while *u16_ptr.add(len) != 0 {
                len += 1;
                if len > 500_000 {
                    break;
                }
            }
            std::slice::from_raw_parts(u16_ptr, len)
        };

        let result = String::from_utf16_lossy(wide_slice);

        let _ = GlobalUnlock(HGLOBAL(handle.0));
        let _ = CloseClipboard();

        if result.trim().is_empty() {
            Err("Clipboard text is empty".to_string())
        } else {
            Ok(result)
        }
    }
}
