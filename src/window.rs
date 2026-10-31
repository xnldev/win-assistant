use std::collections::VecDeque;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_DOWN, VK_LEFT, VK_MENU, VK_RIGHT, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::api::{execute_query_stream, QueryPayload};
use crate::capture::capture_screen_base64;
use crate::clipboard::get_clipboard_text;
use crate::config::Config;
use crate::hotkeys::*;
use crate::render::{AssistantStatus, ChatEntry, Renderer};

#[repr(C)]
#[derive(Default)]
struct Win32SystemTime {
    w_year: u16,
    w_month: u16,
    w_day_of_week: u16,
    w_day: u16,
    w_hour: u16,
    w_minute: u16,
    w_second: u16,
    w_milliseconds: u16,
}

unsafe extern "system" {
    fn GetLocalTime(lp_system_time: *mut Win32SystemTime);
}

pub fn get_current_time_str() -> String {
    let mut st = Win32SystemTime::default();
    unsafe {
        GetLocalTime(&mut st);
    }
    format!("{:02}:{:02}:{:02}", st.w_hour, st.w_minute, st.w_second)
}

pub struct QueueItem {
    pub id: usize,
    #[allow(dead_code)]
    pub title: String,
    #[allow(dead_code)]
    pub timestamp: String,
    pub payload: QueryPayload,
}

pub struct AppState {
    pub hwnd_val: isize,
    pub config: Config,
    pub renderer: Mutex<Renderer>,
    pub status: Mutex<AssistantStatus>,
    pub history: Mutex<Vec<ChatEntry>>,
    pub queue: Mutex<VecDeque<QueueItem>>,
    pub queue_cvar: Condvar,
    pub query_counter: AtomicUsize,
    pub opacity_percent: AtomicU8,
    pub current_opacity_f32: Mutex<f32>,
    pub is_visible: AtomicBool,
    pub is_running: AtomicBool,
    pub follow_mouse: AtomicBool,
    pub cur_x: AtomicI32,
    pub cur_y: AtomicI32,
    pub width: AtomicI32,
    pub height: AtomicI32,
    pub preset_index: AtomicU8,
}

impl AppState {
    pub fn hwnd(&self) -> HWND {
        HWND(self.hwnd_val as *mut _)
    }
}

static GLOBAL_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

#[allow(dead_code)]
pub struct AssistantWindow {
    pub hwnd: HWND,
    pub state: Arc<AppState>,
    pub hotkeys: HotkeyManager,
}

impl AssistantWindow {
    pub fn create_and_show(config: Config) -> Result<Self, String> {
        unsafe {
            let instance = windows::Win32::System::LibraryLoader::GetModuleHandleW(PCWSTR(null_mut()))
                .map_err(|e| format!("GetModuleHandleW error: {:?}", e))?;

            let class_name = w!("WinAssistantTUIClass");

            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: HINSTANCE(instance.0),
                hIcon: HICON::default(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hbrBackground: HBRUSH::default(),
                lpszMenuName: PCWSTR(null_mut()),
                lpszClassName: class_name,
                hIconSm: HICON::default(),
            };

            let _ = RegisterClassExW(&wc);

            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let win_w = config.window.width;
            let win_h = config.window.height;
            let pos_x = ((screen_w - win_w) / 2).max(0);
            let pos_y = config.window.top_offset.max(10);

            // Extended styles: topmost, toolwindow (hidden from taskbar/alt-tab), layered, click-through, non-focus stealing
            let ex_style = WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_LAYERED
                | WS_EX_TRANSPARENT
                | WS_EX_NOACTIVATE;

            let hwnd = CreateWindowExW(
                ex_style,
                class_name,
                w!("Win-Assistant"),
                WS_POPUP,
                pos_x,
                pos_y,
                win_w,
                win_h,
                None,
                None,
                Some(HINSTANCE(instance.0)),
                None,
            ).map_err(|e| format!("CreateWindowExW error: {:?}", e))?;

            // Hardware stealth for screen sharing / OBS capture
            let _ = SetWindowDisplayAffinity(hwnd, WINDOW_DISPLAY_AFFINITY(0x00000011));

            // Apply hardware rounded corners (12px corner radius)
            let rgn = CreateRoundRectRgn(0, 0, win_w + 1, win_h + 1, 24, 24);
            let _ = SetWindowRgn(hwnd, Some(rgn), true);

            // Set initial opacity
            let initial_opacity = config.window.opacity_percent.clamp(30, 98);
            let alpha = ((initial_opacity as u32 * 255) / 100) as u8;
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);

            let renderer = Renderer::new(&config)?;

            let default_welcome = "# WIN-ASSISTANT\n\n\
                * Alt + Q  ›  Scan Screen\n\
                * Alt + W  ›  Analyze Clipboard\n\
                * Alt + C  ›  Clear History\n\
                * Alt + F  ›  Follow Cursor\n\
                * Alt + M  ›  Cycle Size Presets\n\
                * Alt + [ / ]  ›  Adjust Width\n\
                * Alt + - / =  ›  Adjust Height\n\
                * Alt + S  ›  Hide / Show\n\
                * Alt + ▲ / ▼  ›  Scroll\n\
                * Alt + ◄ / ►  ›  Opacity\n\
                * Alt + Shift + X  ›  Exit"
                .to_string();

            let initial_history = vec![ChatEntry {
                id: 0,
                title: "SYSTEM".to_string(),
                timestamp: get_current_time_str(),
                content: default_welcome,
                is_streaming: false,
            }];

            let state = Arc::new(AppState {
                hwnd_val: hwnd.0 as isize,
                config: config.clone(),
                renderer: Mutex::new(renderer),
                status: Mutex::new(AssistantStatus::Idle),
                history: Mutex::new(initial_history),
                queue: Mutex::new(VecDeque::new()),
                queue_cvar: Condvar::new(),
                query_counter: AtomicUsize::new(0),
                opacity_percent: AtomicU8::new(initial_opacity),
                current_opacity_f32: Mutex::new(initial_opacity as f32),
                is_visible: AtomicBool::new(true),
                is_running: AtomicBool::new(true),
                follow_mouse: AtomicBool::new(config.window.follow_mouse),
                cur_x: AtomicI32::new(pos_x),
                cur_y: AtomicI32::new(pos_y),
                width: AtomicI32::new(win_w),
                height: AtomicI32::new(win_h),
                preset_index: AtomicU8::new(1),
            });

            let _ = GLOBAL_STATE.set(state.clone());

            // Dedicated background queue worker thread
            let worker_state = state.clone();
            std::thread::spawn(move || {
                while worker_state.is_running.load(Ordering::Relaxed) {
                    let item = {
                        let mut q = worker_state.queue.lock().unwrap();
                        while q.is_empty() {
                            if !worker_state.is_running.load(Ordering::Relaxed) {
                                return;
                            }
                            q = worker_state.queue_cvar.wait(q).unwrap();
                        }
                        q.pop_front().unwrap()
                    };

                    let remaining = worker_state.queue.lock().unwrap().len();
                    {
                        let mut st = worker_state.status.lock().unwrap();
                        if remaining > 0 {
                            *st = AssistantStatus::Queue(remaining + 1);
                        } else {
                            *st = AssistantStatus::Querying;
                        }
                    }
                    {
                        let mut hist = worker_state.history.lock().unwrap();
                        if let Some(entry) = hist.iter_mut().find(|e| e.id == item.id) {
                            entry.content = "⚡ Analyzing question...".to_string();
                            entry.is_streaming = true;
                        }
                    }
                    let _ = InvalidateRect(Some(worker_state.hwnd()), None, false);

                    let active_config = Config::load_or_create().unwrap_or_else(|_| worker_state.config.clone());
                    let item_id = item.id;
                    let worker_chunk_state = worker_state.clone();

                    let on_chunk = move |chunk: &str| {
                        if let Ok(mut hist) = worker_chunk_state.history.lock() {
                            if let Some(entry) = hist.iter_mut().find(|e| e.id == item_id) {
                                if entry.content == "⚡ Analyzing question..." {
                                    entry.content.clear();
                                }
                                entry.content.push_str(chunk);
                            }
                        }
                        let _ = InvalidateRect(Some(worker_chunk_state.hwnd()), None, false);
                    };

                    let result = execute_query_stream(&active_config, item.payload, on_chunk);

                    {
                        let mut hist = worker_state.history.lock().unwrap();
                        if let Some(entry) = hist.iter_mut().find(|e| e.id == item_id) {
                            match result {
                                Ok(ans) => {
                                    entry.content = ans;
                                    entry.is_streaming = false;
                                }
                                Err(e) => {
                                    entry.content = format!("Request failed:\n\n{}", e);
                                    entry.is_streaming = false;
                                }
                            }
                        }
                    }

                    {
                        let remaining_after = worker_state.queue.lock().unwrap().len();
                        let mut st = worker_state.status.lock().unwrap();
                        if remaining_after > 0 {
                            *st = AssistantStatus::Queue(remaining_after);
                        } else {
                            *st = AssistantStatus::Idle;
                        }
                    }
                    let _ = InvalidateRect(Some(worker_state.hwnd()), None, false);
                }
            });

            // High-refresh rate mouse tracking and smooth animation loop synced to monitor refresh rate (DwmFlush)
            let follow_state = state.clone();
            std::thread::spawn(move || {
                while follow_state.is_running.load(Ordering::Relaxed) {
                    // V-Sync to physical monitor refresh rate (144Hz, 165Hz, 240Hz, 60Hz)
                    if DwmFlush().is_err() {
                        std::thread::sleep(std::time::Duration::from_millis(7));
                    }

                    if !follow_state.is_visible.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(25));
                        continue;
                    }

                    let follow = follow_state.follow_mouse.load(Ordering::Relaxed);
                    let screen_w = GetSystemMetrics(SM_CXSCREEN);
                    let screen_h = GetSystemMetrics(SM_CYSCREEN);
                    let win_w = follow_state.width.load(Ordering::Relaxed);
                    let win_h = follow_state.height.load(Ordering::Relaxed);

                    let (target_x, target_y) = if follow {
                        let mut pt = POINT::default();
                        let _ = GetCursorPos(&mut pt);

                        let mut tx = pt.x + 22;
                        let mut ty = pt.y + 22;

                        if tx + win_w > screen_w - 12 {
                            tx = (pt.x - win_w - 22).max(10);
                        }
                        if ty + win_h > screen_h - 12 {
                            ty = (pt.y - win_h - 22).max(10);
                        }
                        (tx, ty)
                    } else {
                        let tx = ((screen_w - win_w) / 2).max(10);
                        let ty = follow_state.config.window.top_offset.max(10);
                        (tx, ty)
                    };

                    let cur_x = follow_state.cur_x.load(Ordering::Relaxed);
                    let cur_y = follow_state.cur_y.load(Ordering::Relaxed);
                    let diff_x = target_x - cur_x;
                    let diff_y = target_y - cur_y;

                    if diff_x != 0 || diff_y != 0 {
                        let factor = if follow { 0.35 } else { 0.22 };
                        let step_x = ((diff_x as f32) * factor).round() as i32;
                        let step_y = ((diff_y as f32) * factor).round() as i32;

                        let next_x = if step_x == 0 { target_x } else { cur_x + step_x };
                        let next_y = if step_y == 0 { target_y } else { cur_y + step_y };

                        if next_x != cur_x || next_y != cur_y {
                            follow_state.cur_x.store(next_x, Ordering::Relaxed);
                            follow_state.cur_y.store(next_y, Ordering::Relaxed);

                            let _ = SetWindowPos(
                                follow_state.hwnd(),
                                None,
                                next_x,
                                next_y,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
                            );
                        }
                    }

                    // Continuous keyboard holding check for smooth gliding scroll & opacity
                    let alt_down = (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
                    if alt_down {
                        let up_down = (GetAsyncKeyState(VK_UP.0 as i32) as u16 & 0x8000) != 0;
                        let down_down = (GetAsyncKeyState(VK_DOWN.0 as i32) as u16 & 0x8000) != 0;
                        let left_down = (GetAsyncKeyState(VK_LEFT.0 as i32) as u16 & 0x8000) != 0;
                        let right_down = (GetAsyncKeyState(VK_RIGHT.0 as i32) as u16 & 0x8000) != 0;

                        if up_down {
                            if let Ok(mut r) = follow_state.renderer.lock() {
                                r.scroll(-6.0);
                            }
                        } else if down_down {
                            if let Ok(mut r) = follow_state.renderer.lock() {
                                r.scroll(6.0);
                            }
                        }

                        if left_down {
                            let cur = follow_state.opacity_percent.load(Ordering::Relaxed);
                            follow_state.opacity_percent.store(cur.saturating_sub(1).max(30), Ordering::Relaxed);
                        } else if right_down {
                            let cur = follow_state.opacity_percent.load(Ordering::Relaxed);
                            follow_state.opacity_percent.store((cur + 1).min(98), Ordering::Relaxed);
                        }
                    }

                    // Smooth opacity interpolation
                    let target_op = follow_state.opacity_percent.load(Ordering::Relaxed) as f32;
                    let mut op_lock = follow_state.current_opacity_f32.lock().unwrap();
                    let op_diff = target_op - *op_lock;
                    if op_diff.abs() > 0.4 {
                        *op_lock += op_diff * 0.18;
                        let alpha = ((*op_lock * 255.0) / 100.0).round().clamp(30.0, 255.0) as u8;
                        let _ = SetLayeredWindowAttributes(follow_state.hwnd(), COLORREF(0), alpha, LWA_ALPHA);
                    } else if *op_lock != target_op {
                        *op_lock = target_op;
                        let alpha = ((target_op * 255.0) / 100.0).round().clamp(30.0, 255.0) as u8;
                        let _ = SetLayeredWindowAttributes(follow_state.hwnd(), COLORREF(0), alpha, LWA_ALPHA);
                    }

                    // Smooth scroll inertia / lerp step
                    let scroll_animating = if let Ok(mut r) = follow_state.renderer.lock() {
                        r.step_animation()
                    } else {
                        false
                    };

                    // Check if active animations (pulsing status, shimmer divider) need repaint
                    let has_active_animations = {
                        let st = follow_state.status.lock().unwrap();
                        matches!(*st, AssistantStatus::Querying | AssistantStatus::Streaming | AssistantStatus::Queue(_))
                    };

                    if scroll_animating || has_active_animations {
                        let _ = InvalidateRect(Some(follow_state.hwnd()), None, false);
                    }
                }
            });

            // Register global hotkeys
            let mut hotkeys = HotkeyManager::new(hwnd);
            hotkeys.register_all(&config.hotkeys)?;

            // Show without stealing active focus
            let _ = ShowWindow(hwnd, SW_SHOWNA);
            let _ = UpdateWindow(hwnd);
            let _ = SetWindowDisplayAffinity(hwnd, WINDOW_DISPLAY_AFFINITY(0x00000011));

            Ok(Self {
                hwnd,
                state,
                hotkeys,
            })
        }
    }
}

pub fn trigger_screen_analysis(state: Arc<AppState>) {
    let base64_img = match capture_screen_base64() {
        Ok(img) => img,
        Err(e) => {
            let mut st = state.status.lock().unwrap();
            *st = AssistantStatus::Error("Capture Error".to_string());
            let query_id = state.query_counter.fetch_add(1, Ordering::Relaxed) + 1;
            let mut hist = state.history.lock().unwrap();
            hist.push(ChatEntry {
                id: query_id,
                title: format!("SCREEN #{}", query_id),
                timestamp: get_current_time_str(),
                content: format!("Capture failed: {}", e),
                is_streaming: false,
            });
            unsafe { let _ = InvalidateRect(Some(state.hwnd()), None, false); }
            return;
        }
    };

    let query_id = state.query_counter.fetch_add(1, Ordering::Relaxed) + 1;
    let time_str = get_current_time_str();

    {
        let mut hist = state.history.lock().unwrap();
        hist.push(ChatEntry {
            id: query_id,
            title: format!("SCREEN #{}", query_id),
            timestamp: time_str.clone(),
            content: "⏳ In queue... Waiting for processing.".to_string(),
            is_streaming: false,
        });
    }

    {
        let mut q = state.queue.lock().unwrap();
        q.push_back(QueueItem {
            id: query_id,
            title: format!("SCREEN #{}", query_id),
            timestamp: time_str,
            payload: QueryPayload::ScreenImageBase64(base64_img),
        });
        let q_len = q.len();
        let mut st = state.status.lock().unwrap();
        if q_len > 1 {
            *st = AssistantStatus::Queue(q_len);
        }
    }
    state.queue_cvar.notify_one();
    unsafe { let _ = InvalidateRect(Some(state.hwnd()), None, false); }
}

pub fn trigger_clipboard_analysis(state: Arc<AppState>) {
    let clip_text = match get_clipboard_text() {
        Ok(txt) => txt,
        Err(e) => {
            let mut st = state.status.lock().unwrap();
            *st = AssistantStatus::Error("Empty Clipboard".to_string());
            let query_id = state.query_counter.fetch_add(1, Ordering::Relaxed) + 1;
            let mut hist = state.history.lock().unwrap();
            hist.push(ChatEntry {
                id: query_id,
                title: format!("CLIPBOARD #{}", query_id),
                timestamp: get_current_time_str(),
                content: format!("Clipboard error: {}", e),
                is_streaming: false,
            });
            unsafe { let _ = InvalidateRect(Some(state.hwnd()), None, false); }
            return;
        }
    };

    let query_id = state.query_counter.fetch_add(1, Ordering::Relaxed) + 1;
    let time_str = get_current_time_str();

    {
        let mut hist = state.history.lock().unwrap();
        hist.push(ChatEntry {
            id: query_id,
            title: format!("CLIPBOARD #{}", query_id),
            timestamp: time_str.clone(),
            content: "⏳ In queue... Waiting for processing.".to_string(),
            is_streaming: false,
        });
    }

    {
        let mut q = state.queue.lock().unwrap();
        q.push_back(QueueItem {
            id: query_id,
            title: format!("CLIPBOARD #{}", query_id),
            timestamp: time_str,
            payload: QueryPayload::ClipboardText(clip_text),
        });
        let q_len = q.len();
        let mut st = state.status.lock().unwrap();
        if q_len > 1 {
            *st = AssistantStatus::Queue(q_len);
        }
    }
    state.queue_cvar.notify_one();
    unsafe { let _ = InvalidateRect(Some(state.hwnd()), None, false); }
}

fn apply_window_resize(hwnd: HWND, state: &Arc<AppState>, new_w: i32, new_h: i32) {
    let new_w = new_w.clamp(480, 1600);
    let new_h = new_h.clamp(280, 1000);
    state.width.store(new_w, Ordering::Relaxed);
    state.height.store(new_h, Ordering::Relaxed);

    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            new_w,
            new_h,
            SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
        );
        let rgn = CreateRoundRectRgn(0, 0, new_w + 1, new_h + 1, 24, 24);
        let _ = SetWindowRgn(hwnd, Some(rgn), true);
    }

    if let Ok(mut renderer) = state.renderer.lock() {
        renderer.resize(new_w, new_h);
    }
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_opt = GLOBAL_STATE.get().cloned();

    match msg {
        WM_HOTKEY => {
            if let Some(state) = state_opt {
                let hotkey_id = wparam.0 as i32;
                match hotkey_id {
                    HOTKEY_ID_CAPTURE_SCREEN => trigger_screen_analysis(state),
                    HOTKEY_ID_ANALYZE_CLIPBOARD => trigger_clipboard_analysis(state),
                    HOTKEY_ID_TOGGLE_FOLLOW => {
                        let cur = state.follow_mouse.load(Ordering::Relaxed);
                        state.follow_mouse.store(!cur, Ordering::Relaxed);
                        unsafe { let _ = InvalidateRect(Some(hwnd), None, false); }
                    }
                    HOTKEY_ID_CYCLE_PRESETS => {
                        let cur_idx = state.preset_index.load(Ordering::Relaxed);
                        let next_idx = (cur_idx + 1) % 3;
                        state.preset_index.store(next_idx, Ordering::Relaxed);

                        let (new_w, new_h) = match next_idx {
                            0 => (680, 380),  // Compact
                            1 => (860, 500),  // Standard
                            _ => (1120, 680), // Wide
                        };
                        apply_window_resize(hwnd, &state, new_w, new_h);
                    }
                    HOTKEY_ID_WIDTH_DEC => {
                        let cur_w = state.width.load(Ordering::Relaxed);
                        let cur_h = state.height.load(Ordering::Relaxed);
                        apply_window_resize(hwnd, &state, cur_w - 60, cur_h);
                    }
                    HOTKEY_ID_WIDTH_INC => {
                        let cur_w = state.width.load(Ordering::Relaxed);
                        let cur_h = state.height.load(Ordering::Relaxed);
                        apply_window_resize(hwnd, &state, cur_w + 60, cur_h);
                    }
                    HOTKEY_ID_HEIGHT_DEC => {
                        let cur_w = state.width.load(Ordering::Relaxed);
                        let cur_h = state.height.load(Ordering::Relaxed);
                        apply_window_resize(hwnd, &state, cur_w - 50, cur_h);
                    }
                    HOTKEY_ID_HEIGHT_INC => {
                        let cur_w = state.width.load(Ordering::Relaxed);
                        let cur_h = state.height.load(Ordering::Relaxed);
                        apply_window_resize(hwnd, &state, cur_w + 50, cur_h);
                    }
                    HOTKEY_ID_TOGGLE_VISIBILITY => {
                        let cur = state.is_visible.load(Ordering::Relaxed);
                        let next = !cur;
                        state.is_visible.store(next, Ordering::Relaxed);
                        unsafe {
                            if next {
                                let _ = ShowWindow(hwnd, SW_SHOWNA);
                                let _ = SetWindowDisplayAffinity(hwnd, WINDOW_DISPLAY_AFFINITY(0x00000011));
                            } else {
                                let _ = ShowWindow(hwnd, SW_HIDE);
                            }
                        }
                    }
                    HOTKEY_ID_SCROLL_UP => {
                        if let Ok(mut renderer) = state.renderer.lock() {
                            renderer.scroll(-45.0);
                        }
                        unsafe { let _ = InvalidateRect(Some(hwnd), None, false); }
                    }
                    HOTKEY_ID_SCROLL_DOWN => {
                        if let Ok(mut renderer) = state.renderer.lock() {
                            renderer.scroll(45.0);
                        }
                        unsafe { let _ = InvalidateRect(Some(hwnd), None, false); }
                    }
                    HOTKEY_ID_OPACITY_DEC => {
                        let cur = state.opacity_percent.load(Ordering::Relaxed);
                        state.opacity_percent.store(cur.saturating_sub(6).max(30), Ordering::Relaxed);
                    }
                    HOTKEY_ID_OPACITY_INC => {
                        let cur = state.opacity_percent.load(Ordering::Relaxed);
                        state.opacity_percent.store((cur + 6).min(98), Ordering::Relaxed);
                    }
                    HOTKEY_ID_CLEAR_HISTORY => {
                        let default_welcome = "# WIN-ASSISTANT\n\n\
                            * Alt + Q  ›  Scan Screen\n\
                            * Alt + W  ›  Analyze Clipboard\n\
                            * Alt + C  ›  Clear History\n\
                            * Alt + F  ›  Follow Cursor\n\
                            * Alt + M  ›  Cycle Size Presets\n\
                            * Alt + [ / ]  ›  Adjust Width\n\
                            * Alt + - / =  ›  Adjust Height\n\
                            * Alt + S  ›  Hide / Show\n\
                            * Alt + ▲ / ▼  ›  Scroll\n\
                            * Alt + ◄ / ►  ›  Opacity\n\
                            * Alt + Shift + X  ›  Exit"
                            .to_string();

                        {
                            let mut hist = state.history.lock().unwrap();
                            hist.clear();
                            hist.push(ChatEntry {
                                id: 0,
                                title: "SYSTEM".to_string(),
                                timestamp: get_current_time_str(),
                                content: default_welcome,
                                is_streaming: false,
                            });
                        }
                        {
                            let mut q = state.queue.lock().unwrap();
                            q.clear();
                        }
                        {
                            let mut st = state.status.lock().unwrap();
                            *st = AssistantStatus::Idle;
                        }
                        if let Ok(mut renderer) = state.renderer.lock() {
                            renderer.reset_scroll();
                        }
                        unsafe { let _ = InvalidateRect(Some(hwnd), None, false); }
                    }
                    HOTKEY_ID_PANIC_EXIT => {
                        state.is_running.store(false, Ordering::Relaxed);
                        state.queue_cvar.notify_all();
                        unsafe { PostQuitMessage(0); }
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }

        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            unsafe {
                let _hdc = BeginPaint(hwnd, &mut ps);

                if let Some(state) = state_opt {
                    let status = state.status.lock().unwrap().clone();
                    let history = state.history.lock().unwrap().clone();
                    let opacity = state.opacity_percent.load(Ordering::Relaxed);
                    let follow = state.follow_mouse.load(Ordering::Relaxed);

                    if let Ok(mut renderer) = state.renderer.lock() {
                        let _ = renderer.render(hwnd, &status, &history, opacity, follow);
                    }
                }

                let _ = EndPaint(hwnd, &ps);
            }
            LRESULT(0)
        }

        WM_ERASEBKGND => LRESULT(1),

        WM_DESTROY => {
            if let Some(state) = state_opt {
                state.is_running.store(false, Ordering::Relaxed);
                state.queue_cvar.notify_all();
            }
            unsafe { PostQuitMessage(0); }
            LRESULT(0)
        }

        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
