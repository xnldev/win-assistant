use std::sync::OnceLock;
use std::time::Instant;
use windows::core::{w, PCWSTR};
use windows_numerics::Vector2;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

use crate::config::Config;
use crate::markdown::{parse_markdown, Block};

fn get_elapsed_seconds() -> f32 {
    static START_TIME: OnceLock<Instant> = OnceLock::new();
    START_TIME.get_or_init(Instant::now).elapsed().as_secs_f32()
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssistantStatus {
    Idle,
    #[allow(dead_code)]
    Capturing,
    Querying,
    #[allow(dead_code)]
    Streaming,
    Queue(usize),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ChatEntry {
    #[allow(dead_code)]
    pub id: usize,
    pub title: String,
    pub timestamp: String,
    pub content: String,
    pub is_streaming: bool,
}

pub struct Renderer {
    d2d_factory: ID2D1Factory,
    dwrite_factory: IDWriteFactory,
    render_target: Option<ID2D1HwndRenderTarget>,
    format_header: IDWriteTextFormat,
    format_body: IDWriteTextFormat,
    format_code: IDWriteTextFormat,
    format_badge: IDWriteTextFormat,
    format_hint: IDWriteTextFormat,
    pub width: i32,
    pub height: i32,
    pub target_scroll_y: f32,
    pub current_scroll_y: f32,
    pub max_scroll_y: f32,
}

impl Renderer {
    pub fn new(config: &Config) -> Result<Self, String> {
        unsafe {
            let d2d_factory: ID2D1Factory = D2D1CreateFactory(
                D2D1_FACTORY_TYPE_SINGLE_THREADED,
                None,
            ).map_err(|e| format!("D2D1CreateFactory error: {:?}", e))?;

            let dwrite_factory: IDWriteFactory = DWriteCreateFactory(
                DWRITE_FACTORY_TYPE_SHARED,
            ).map_err(|e| format!("DWriteCreateFactory error: {:?}", e))?;

            let font_mono_wide: Vec<u16> = config.window.font_family.encode_utf16().chain(Some(0)).collect();

            let format_header = dwrite_factory.CreateTextFormat(
                PCWSTR(font_mono_wide.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                config.window.font_size * 1.15,
                w!("en-us"),
            ).map_err(|e| format!("CreateTextFormat header error: {:?}", e))?;

            let format_body = dwrite_factory.CreateTextFormat(
                PCWSTR(font_mono_wide.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                config.window.font_size,
                w!("en-us"),
            ).map_err(|e| format!("CreateTextFormat body error: {:?}", e))?;

            let format_code = dwrite_factory.CreateTextFormat(
                PCWSTR(font_mono_wide.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                config.window.font_size * 0.95,
                w!("en-us"),
            ).map_err(|e| format!("CreateTextFormat code error: {:?}", e))?;

            let format_badge = dwrite_factory.CreateTextFormat(
                PCWSTR(font_mono_wide.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                11.0,
                w!("en-us"),
            ).map_err(|e| format!("CreateTextFormat badge error: {:?}", e))?;

            let format_hint = dwrite_factory.CreateTextFormat(
                PCWSTR(font_mono_wide.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                10.5,
                w!("en-us"),
            ).map_err(|e| format!("CreateTextFormat hint error: {:?}", e))?;

            Ok(Self {
                d2d_factory,
                dwrite_factory,
                render_target: None,
                format_header,
                format_body,
                format_code,
                format_badge,
                format_hint,
                width: config.window.width,
                height: config.window.height,
                target_scroll_y: 0.0,
                current_scroll_y: 0.0,
                max_scroll_y: 0.0,
            })
        }
    }

    pub fn resize(&mut self, new_w: i32, new_h: i32) {
        self.width = new_w;
        self.height = new_h;
        if let Some(ref rt) = self.render_target {
            unsafe {
                let _ = rt.Resize(&D2D_SIZE_U {
                    width: new_w as u32,
                    height: new_h as u32,
                });
            }
        }
    }

    pub fn ensure_render_target(&mut self, hwnd: HWND) -> Result<(), String> {
        if self.render_target.is_some() {
            return Ok(());
        }

        unsafe {
            let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 0.0,
                dpiY: 0.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };

            let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd,
                pixelSize: D2D_SIZE_U {
                    width: self.width as u32,
                    height: self.height as u32,
                },
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };

            let rt = self.d2d_factory.CreateHwndRenderTarget(&rt_props, &hwnd_props)
                .map_err(|e| format!("CreateHwndRenderTarget error: {:?}", e))?;

            self.render_target = Some(rt);
            Ok(())
        }
    }

    pub fn scroll(&mut self, delta: f32) {
        self.target_scroll_y = (self.target_scroll_y + delta).clamp(0.0, self.max_scroll_y);
    }

    pub fn reset_scroll(&mut self) {
        self.target_scroll_y = 0.0;
        self.current_scroll_y = 0.0;
    }

    pub fn step_animation(&mut self) -> bool {
        let diff = self.target_scroll_y - self.current_scroll_y;
        if diff.abs() > 0.3 {
            self.current_scroll_y += diff * 0.18;
            true
        } else if self.current_scroll_y != self.target_scroll_y {
            self.current_scroll_y = self.target_scroll_y;
            true
        } else {
            false
        }
    }

    pub fn render(
        &mut self,
        hwnd: HWND,
        status: &AssistantStatus,
        history: &[ChatEntry],
        _opacity_percent: u8,
        follow_mouse: bool,
    ) -> Result<(), String> {
        self.ensure_render_target(hwnd)?;
        let rt = self.render_target.as_ref().unwrap();

        unsafe {
            rt.BeginDraw();

            let w = self.width as f32;
            let h = self.height as f32;
            let time = get_elapsed_seconds();

            // 1. PURE DEEP OBSIDIAN DARK BACKGROUND (#090A0E)
            // Clean, non-distracting matte black for maximum contrast and eye comfort
            rt.Clear(Some(&D2D1_COLOR_F { r: 0.035, g: 0.040, b: 0.050, a: 1.0 }));

            // High-End Color Palette Brushes: Obsidian + Elegant Orange
            let border_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.16, g: 0.19, b: 0.24, a: 1.0 }, // Subtle dark slate border (#29303D)
                None,
            ).unwrap();

            let accent_orange_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 1.0, g: 0.48, b: 0.12, a: 1.0 }, // Radiant electric orange (#FF7A1F)
                None,
            ).unwrap();

            let accent_amber_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 1.0, g: 0.62, b: 0.24, a: 1.0 }, // Soft warm amber (#FF9E3D)
                None,
            ).unwrap();

            let header_bg_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.060, g: 0.066, b: 0.078, a: 1.0 }, // Polished header charcoal (#0F1114)
                None,
            ).unwrap();

            let footer_bg_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.048, g: 0.052, b: 0.064, a: 1.0 }, // Minimalist footer (#0C0D10)
                None,
            ).unwrap();

            let text_bright_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.95, g: 0.96, b: 0.98, a: 1.0 }, // Crisp header white (#F3F4F6)
                None,
            ).unwrap();

            let text_primary_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.84, g: 0.86, b: 0.90, a: 1.0 }, // Pleasant off-white body (#D7DADC)
                None,
            ).unwrap();

            let text_muted_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.46, g: 0.50, b: 0.56, a: 1.0 }, // Sleek steel muted (#75808F)
                None,
            ).unwrap();

            let code_bg_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.018, g: 0.022, b: 0.030, a: 1.0 }, // Pitch deep editor black (#050608)
                None,
            ).unwrap();

            let code_border_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.12, g: 0.15, b: 0.20, a: 1.0 }, // Fine code container border
                None,
            ).unwrap();

            let brand_bg_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.13, g: 0.07, b: 0.03, a: 0.95 }, // Deep warm dark amber (#211208)
                None,
            ).unwrap();

            let brand_border_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.46, g: 0.24, b: 0.08, a: 0.90 }, // Warm amber hairline border
                None,
            ).unwrap();

            let text_code_brush = rt.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.18, g: 0.90, b: 0.62, a: 1.0 }, // Neon terminal emerald (#2EE59D)
                None,
            ).unwrap();

            // 2. PRECISION ROUNDED OUTER FRAME (1.2px)
            let rounded_frame = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: 0.5,
                    top: 0.5,
                    right: w - 0.5,
                    bottom: h - 0.5,
                },
                radiusX: 12.0,
                radiusY: 12.0,
            };
            rt.DrawRoundedRectangle(&rounded_frame, &border_brush, 1.2, None);

            // 3. HEADER BAR (H = 34px)
            let header_rect = D2D_RECT_F {
                left: 1.0,
                top: 1.5,
                right: w - 1.0,
                bottom: 34.0,
            };
            rt.FillRectangle(&header_rect, &header_bg_brush);

            // Header bottom accent line: lowered below the top bar, spanning flush edge-to-edge
            rt.DrawLine(
                Vector2 { X: 1.0, Y: 34.0 },
                Vector2 { X: w - 1.0, Y: 34.0 },
                &accent_orange_brush,
                1.0,
                None,
            );

            // App Brand: Small orange indicator pip + "WIN-ASSISTANT"
            let pip_rect = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: 16.0,
                    top: 13.0,
                    right: 19.5,
                    bottom: 23.0,
                },
                radiusX: 1.5,
                radiusY: 1.5,
            };
            rt.FillRoundedRectangle(&pip_rect, &accent_orange_brush);

            let title_text = "WIN-ASSISTANT";
            let title_wide: Vec<u16> = title_text.encode_utf16().collect();
            let title_rect = D2D_RECT_F {
                left: 26.0,
                top: 8.5,
                right: 145.0,
                bottom: 28.5,
            };
            rt.DrawText(
                &title_wide,
                &self.format_badge,
                &title_rect,
                &text_bright_brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Status Indicator Badge: ● READY / ● CAPTURING / ● SOLVING / ● QUEUE / ● ERROR
            let status_err_str: String;
            let status_queue_str: String;
            let (status_label, status_color) = match status {
                AssistantStatus::Idle => ("● READY", D2D1_COLOR_F { r: 0.16, g: 0.82, b: 0.45, a: 1.0 }),
                AssistantStatus::Capturing => ("● CAPTURING...", D2D1_COLOR_F { r: 1.0, g: 0.62, b: 0.18, a: 1.0 }),
                AssistantStatus::Querying => ("● SOLVING...", D2D1_COLOR_F { r: 1.0, g: 0.48, b: 0.12, a: 1.0 }),
                AssistantStatus::Streaming => ("● STREAMING...", D2D1_COLOR_F { r: 1.0, g: 0.55, b: 0.15, a: 1.0 }),
                AssistantStatus::Queue(count) => {
                    status_queue_str = format!("● QUEUE ({} ACTIVE)...", count);
                    (status_queue_str.as_str(), D2D1_COLOR_F { r: 1.0, g: 0.55, b: 0.15, a: 1.0 })
                }
                AssistantStatus::Error(msg) => {
                    let truncated = if msg.len() > 22 { &msg[..22] } else { msg.as_str() };
                    status_err_str = format!("● ERR: {}", truncated);
                    (status_err_str.as_str(), D2D1_COLOR_F { r: 0.98, g: 0.38, b: 0.38, a: 1.0 })
                }
            };

            let pulse_speed = match status {
                AssistantStatus::Querying | AssistantStatus::Streaming | AssistantStatus::Queue(_) => 5.2,
                _ => 2.2,
            };
            let breath = 0.72 + 0.28 * (time * pulse_speed).sin();
            let animated_status_color = D2D1_COLOR_F {
                r: status_color.r,
                g: status_color.g,
                b: status_color.b,
                a: status_color.a * breath,
            };
            let status_brush = rt.CreateSolidColorBrush(&animated_status_color, None).unwrap();
            let status_wide: Vec<u16> = status_label.encode_utf16().collect();
            let status_rect = D2D_RECT_F {
                left: 152.0,
                top: 8.5,
                right: 335.0,
                bottom: 28.5,
            };
            rt.DrawText(
                &status_wide,
                &self.format_badge,
                &status_rect,
                &status_brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Mode Tag: [TRACK: MOUSE] or [DOCKED: TOP]
            let mode_tag = if follow_mouse {
                "[TRACK: MOUSE]"
            } else {
                "[DOCKED: TOP]"
            };
            let mode_color = if follow_mouse {
                D2D1_COLOR_F { r: 1.0, g: 0.62, b: 0.22, a: 0.95 }
            } else {
                D2D1_COLOR_F { r: 0.48, g: 0.52, b: 0.58, a: 0.80 }
            };
            let mode_brush = rt.CreateSolidColorBrush(&mode_color, None).unwrap();
            let mode_wide: Vec<u16> = mode_tag.encode_utf16().collect();
            let mode_rect = D2D_RECT_F {
                left: 335.0,
                top: 8.5,
                right: 480.0,
                bottom: 28.5,
            };
            rt.DrawText(
                &mode_wide,
                &self.format_hint,
                &mode_rect,
                &mode_brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // 4. CLEAN MINIMALIST FOOTER (H = 26px)
            let footer_rect = D2D_RECT_F {
                left: 1.0,
                top: h - 26.0,
                right: w - 1.0,
                bottom: h - 1.0,
            };
            rt.FillRectangle(&footer_rect, &footer_bg_brush);
            rt.DrawLine(
                Vector2 { X: 1.0, Y: h - 26.0 },
                Vector2 { X: w - 1.0, Y: h - 26.0 },
                &border_brush,
                1.0,
                None,
            );

            let footer_text = "Alt+Q: Scan   •   Alt+W: Clip   •   Alt+C: Clear   •   Alt+F: Follow   •   Alt+▲▼: Scroll";
            let footer_wide: Vec<u16> = footer_text.encode_utf16().collect();
            let footer_text_rect = D2D_RECT_F {
                left: 16.0,
                top: h - 20.0,
                right: w - 16.0,
                bottom: h - 4.0,
            };
            rt.DrawText(
                &footer_wide,
                &self.format_hint,
                &footer_text_rect,
                &text_muted_brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // 5. CONTENT AREA WITH AXIS-ALIGNED CLIPPING
            let clip_rect = D2D_RECT_F {
                left: 18.0,
                top: 40.0,
                right: w - 20.0,
                bottom: h - 32.0,
            };
            rt.PushAxisAlignedClip(&clip_rect, D2D1_ANTIALIAS_MODE_ALIASED);

            let content_width = (w - 48.0).max(200.0);
            let scroll = self.current_scroll_y;
            let mut cur_y = 42.0 - scroll;

            for (entry_idx, entry) in history.iter().enumerate() {
                // Divider between distinct chat entries
                if entry_idx > 0 {
                    cur_y += 14.0;
                    let line_y = cur_y + 9.0;
                    if line_y >= 36.0 && line_y <= h - 28.0 {
                        rt.DrawLine(
                            Vector2 { X: 18.0, Y: line_y },
                            Vector2 { X: w - 20.0, Y: line_y },
                            &border_brush,
                            1.0,
                            None,
                        );

                        // Micro-animation: Cyberpunk laser pulse gliding across active task divider
                        let is_processing = entry.is_streaming
                            || entry.content.contains("Analyzing")
                            || entry.content.contains("In queue");

                        if is_processing {
                            let beam_len = 90.0;
                            let track_w = w - 40.0;
                            let beam_pos = 18.0 + ((time * 180.0) % (track_w + beam_len)) - beam_len;
                            let beam_start = beam_pos.max(18.0);
                            let beam_end = (beam_pos + beam_len).min(w - 20.0);
                            if beam_end > beam_start {
                                let laser_brush = rt.CreateSolidColorBrush(
                                    &D2D1_COLOR_F { r: 1.0, g: 0.58, b: 0.16, a: 0.90 },
                                    None,
                                ).unwrap();
                                rt.DrawLine(
                                    Vector2 { X: beam_start, Y: line_y },
                                    Vector2 { X: beam_end, Y: line_y },
                                    &laser_brush,
                                    1.8,
                                    None,
                                );
                            }
                        }

                        let chip_text = format!(" {} • {} ", entry.title, entry.timestamp);
                        let chip_wide: Vec<u16> = chip_text.encode_utf16().collect();
                        let chip_w = (chip_text.len() as f32 * 7.5).max(90.0);
                        let chip_rect = D2D_RECT_F {
                            left: 28.0,
                            top: cur_y,
                            right: 28.0 + chip_w,
                            bottom: cur_y + 18.0,
                        };
                        rt.FillRectangle(&chip_rect, &brand_bg_brush);
                        rt.DrawRectangle(&chip_rect, &brand_border_brush, 1.0, None);
                        rt.DrawText(
                            &chip_wide,
                            &self.format_hint,
                            &chip_rect,
                            &accent_amber_brush,
                            D2D1_DRAW_TEXT_OPTIONS_NONE,
                            DWRITE_MEASURING_MODE_NATURAL,
                        );
                    }
                    cur_y += 26.0;
                }

                let mut text_buf = entry.content.clone();
                if entry.is_streaming {
                    text_buf.push_str(" ▊");
                }

                let blocks = parse_markdown(&text_buf);
                for block in blocks {
                    match block {
                        Block::Header { level: _, text } => {
                            let text_wide: Vec<u16> = text.encode_utf16().collect();
                            let layout = self.dwrite_factory.CreateTextLayout(
                                &text_wide,
                                &self.format_header,
                                content_width - 16.0,
                                1000.0,
                            );

                            if let Ok(layout) = layout {
                                let mut metrics = DWRITE_TEXT_METRICS::default();
                                let _ = layout.GetMetrics(&mut metrics);

                                if cur_y + metrics.height >= 36.0 && cur_y <= h - 28.0 {
                                    let pill_rect = D2D_RECT_F {
                                        left: 18.0,
                                        top: cur_y + 2.0,
                                        right: 21.0,
                                        bottom: cur_y + metrics.height - 2.0,
                                    };
                                    rt.FillRectangle(&pill_rect, &accent_orange_brush);

                                    rt.DrawTextLayout(
                                        Vector2 { X: 28.0, Y: cur_y },
                                        &layout,
                                        &text_bright_brush,
                                        D2D1_DRAW_TEXT_OPTIONS_NONE,
                                    );
                                }
                                cur_y += metrics.height + 12.0;
                            }
                        }
                        Block::Paragraph(text) => {
                            let text_wide: Vec<u16> = text.encode_utf16().collect();
                            let layout = self.dwrite_factory.CreateTextLayout(
                                &text_wide,
                                &self.format_body,
                                content_width,
                                4000.0,
                            );

                            if let Ok(layout) = layout {
                                let mut metrics = DWRITE_TEXT_METRICS::default();
                                let _ = layout.GetMetrics(&mut metrics);

                                if cur_y + metrics.height >= 36.0 && cur_y <= h - 28.0 {
                                    rt.DrawTextLayout(
                                        Vector2 { X: 18.0, Y: cur_y },
                                        &layout,
                                        &text_primary_brush,
                                        D2D1_DRAW_TEXT_OPTIONS_NONE,
                                    );
                                }
                                cur_y += metrics.height + 11.0;
                            }
                        }
                        Block::Bullet { prefix: _, text } => {
                            let bullet_mark = "›";
                            let mark_wide: Vec<u16> = bullet_mark.encode_utf16().collect();
                            let text_wide: Vec<u16> = text.encode_utf16().collect();

                            let layout = self.dwrite_factory.CreateTextLayout(
                                &text_wide,
                                &self.format_body,
                                content_width - 24.0,
                                2000.0,
                            );

                            if let Ok(layout) = layout {
                                let mut metrics = DWRITE_TEXT_METRICS::default();
                                let _ = layout.GetMetrics(&mut metrics);

                                if cur_y + metrics.height >= 36.0 && cur_y <= h - 28.0 {
                                    let mark_rect = D2D_RECT_F {
                                        left: 18.0,
                                        top: cur_y,
                                        right: 32.0,
                                        bottom: cur_y + metrics.height,
                                    };
                                    rt.DrawText(
                                        &mark_wide,
                                        &self.format_badge,
                                        &mark_rect,
                                        &accent_orange_brush,
                                        D2D1_DRAW_TEXT_OPTIONS_NONE,
                                        DWRITE_MEASURING_MODE_NATURAL,
                                    );

                                    rt.DrawTextLayout(
                                        Vector2 { X: 34.0, Y: cur_y },
                                        &layout,
                                        &text_primary_brush,
                                        D2D1_DRAW_TEXT_OPTIONS_NONE,
                                    );
                                }
                                cur_y += metrics.height + 9.0;
                            }
                        }
                        Block::CodeBlock { lang, code } => {
                            let code_wide: Vec<u16> = code.encode_utf16().collect();
                            let layout = self.dwrite_factory.CreateTextLayout(
                                &code_wide,
                                &self.format_code,
                                content_width - 24.0,
                                5000.0,
                            );

                            if let Ok(layout) = layout {
                                let mut metrics = DWRITE_TEXT_METRICS::default();
                                let _ = layout.GetMetrics(&mut metrics);

                                let box_height = metrics.height + 18.0;
                                let box_rect = D2D_RECT_F {
                                    left: 18.0,
                                    top: cur_y,
                                    right: 18.0 + content_width,
                                    bottom: cur_y + box_height,
                                };

                                if cur_y + box_height >= 36.0 && cur_y <= h - 28.0 {
                                    rt.FillRectangle(&box_rect, &code_bg_brush);
                                    rt.DrawRectangle(&box_rect, &code_border_brush, 1.0, None);

                                    if !lang.is_empty() {
                                        let lang_tag = format!("[{}]", lang.to_uppercase());
                                        let lang_wide: Vec<u16> = lang_tag.encode_utf16().collect();
                                        let lang_rect = D2D_RECT_F {
                                            left: 18.0 + content_width - 120.0,
                                            top: cur_y + 3.0,
                                            right: 18.0 + content_width - 8.0,
                                            bottom: cur_y + 18.0,
                                        };
                                        rt.DrawText(
                                            &lang_wide,
                                            &self.format_hint,
                                            &lang_rect,
                                            &text_muted_brush,
                                            D2D1_DRAW_TEXT_OPTIONS_NONE,
                                            DWRITE_MEASURING_MODE_NATURAL,
                                        );
                                    }

                                    rt.DrawTextLayout(
                                        Vector2 { X: 26.0, Y: cur_y + 9.0 },
                                        &layout,
                                        &text_code_brush,
                                        D2D1_DRAW_TEXT_OPTIONS_NONE,
                                    );
                                }
                                cur_y += box_height + 12.0;
                            }
                        }
                    }
                }
                cur_y += 10.0;
            }

            rt.PopAxisAlignedClip();

            // 6. MINIMALIST SLATE SCROLLBAR THUMB
            let total_content_height = cur_y + scroll - 42.0;
            let visible_height = h - 68.0;
            let max_scroll = (total_content_height - visible_height).max(0.0);
            self.max_scroll_y = max_scroll;

            if max_scroll > 0.0 {
                let scrollbar_track_h = visible_height;
                let thumb_h = ((visible_height / total_content_height) * scrollbar_track_h).clamp(28.0, scrollbar_track_h);
                let scroll_ratio = (scroll / max_scroll).clamp(0.0, 1.0);
                let thumb_y = 38.0 + scroll_ratio * (scrollbar_track_h - thumb_h);

                let scroll_brush = rt.CreateSolidColorBrush(
                    &D2D1_COLOR_F { r: 0.36, g: 0.30, b: 0.25, a: 0.80 },
                    None,
                ).unwrap();

                let thumb_rounded = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: w - 7.0,
                        top: thumb_y,
                        right: w - 3.0,
                        bottom: thumb_y + thumb_h,
                    },
                    radiusX: 2.0,
                    radiusY: 2.0,
                };
                rt.FillRoundedRectangle(&thumb_rounded, &scroll_brush);
            }

            let _ = rt.EndDraw(None, None);
            Ok(())
        }
    }
}
