use base64::Engine;
use std::ptr::null_mut;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// Captures primary monitor framebuffer directly into RAM.
/// Excludes overlay automatically via `WDA_EXCLUDEFROMCAPTURE`.
pub fn capture_screen_base64() -> Result<String, String> {
    unsafe {
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);

        if screen_w <= 0 || screen_h <= 0 {
            return Err("Failed to query screen resolution".to_string());
        }

        let hdc_screen = GetDC(None);
        if hdc_screen.is_invalid() {
            return Err("GetDC(NULL) returned invalid DC".to_string());
        }

        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        if hdc_mem.is_invalid() {
            let _ = ReleaseDC(None, hdc_screen);
            return Err("CreateCompatibleDC failed".to_string());
        }

        // biHeight < 0 for top-down DIB raster
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: screen_w,
                biHeight: -screen_h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default()],
        };

        let mut bits_ptr: *mut std::ffi::c_void = null_mut();
        let hbm = CreateDIBSection(
            Some(hdc_screen),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            None,
            0,
        ).map_err(|e| {
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            format!("CreateDIBSection error: {:?}", e)
        })?;

        let old_obj = SelectObject(hdc_mem, hbm.into());

        // Copy desktop into memory
        // CAPTUREBLT (0x40000000) includes layered windows, WDA_EXCLUDEFROMCAPTURE excludes our overlay
        let rop = SRCCOPY | ROP_CODE(0x40000000);
        let blt_res = BitBlt(
            hdc_mem,
            0,
            0,
            screen_w,
            screen_h,
            Some(hdc_screen),
            0,
            0,
            rop,
        );

        if blt_res.is_err() {
            SelectObject(hdc_mem, old_obj);
            let _ = DeleteObject(hbm.into());
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            return Err("BitBlt screen capture failed".to_string());
        }

        let total_pixels = (screen_w * screen_h) as usize;
        let bgra_slice = std::slice::from_raw_parts(bits_ptr as *const u8, total_pixels * 4);

        // Smart downscale for monitors wider than 1600px to reduce upload payload by 4x
        let scale = if screen_w > 1600 { 2 } else { 1 };
        let out_w = screen_w / scale;
        let out_h = screen_h / scale;
        let mut rgb_buffer = Vec::with_capacity((out_w * out_h * 3) as usize);

        if scale == 1 {
            for chunk in bgra_slice.chunks_exact(4) {
                rgb_buffer.push(chunk[2]); // R
                rgb_buffer.push(chunk[1]); // G
                rgb_buffer.push(chunk[0]); // B
            }
        } else {
            for y in 0..out_h {
                let src_y = y * scale;
                for x in 0..out_w {
                    let src_x = x * scale;
                    let src_idx = ((src_y * screen_w + src_x) * 4) as usize;
                    if src_idx + 2 < bgra_slice.len() {
                        rgb_buffer.push(bgra_slice[src_idx + 2]); // R
                        rgb_buffer.push(bgra_slice[src_idx + 1]); // G
                        rgb_buffer.push(bgra_slice[src_idx]);     // B
                    }
                }
            }
        }

        // Clean up GDI objects
        SelectObject(hdc_mem, old_obj);
        let _ = DeleteObject(hbm.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);

        // Compress directly in RAM as JPEG (68 quality for ultra-fast TLS upload while keeping text sharp)
        let mut jpeg_bytes = Vec::new();
        let encoder = jpeg_encoder::Encoder::new(&mut jpeg_bytes, 68);
        encoder.encode(
            &rgb_buffer,
            out_w as u16,
            out_h as u16,
            jpeg_encoder::ColorType::Rgb,
        ).map_err(|e| format!("JPEG encoder error: {:?}", e))?;

        let base64_image = base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes);
        Ok(base64_image)
    }
}
