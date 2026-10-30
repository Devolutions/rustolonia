//! A paint surface: drag with the left button to draw, right-click to clear.
//! Rust owns the pixels; the host only shows them and reports the pointer.
//!
//! `RUSTOLONIA_EXAMPLE_EXIT_AFTER_MS` closes it after a delay (for smoke runs).

use rustolonia::pixel_surface::{Cursor, MouseButton, PixelFormat, PointerKind, SurfaceInput};
use rustolonia::{App, Dock, DockPanel, PixelSurface, Result, TextBlock, Window};
use std::sync::{Arc, Mutex};

struct Canvas {
    width: u32,
    height: u32,
    scaling: f64,
    pixels: Vec<u8>,
    drawing: bool,
}

impl Canvas {
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.pixels = vec![0; width as usize * height as usize * 4];
        for (i, px) in self.pixels.chunks_exact_mut(4).enumerate() {
            let x = (i as u32 % width.max(1)) as f32 / width.max(1) as f32;
            px.copy_from_slice(&[(40.0 + 60.0 * x) as u8, 44, 52, 255]);
        }
    }

    /// Paints a dot around (`x`, `y`) in physical pixels; returns the damage.
    fn dot(&mut self, x: f64, y: f64) -> Option<(i32, i32, u32, u32)> {
        let r = (6.0 * self.scaling) as i32;
        let (cx, cy) = (x as i32, y as i32);
        let x0 = (cx - r).max(0);
        let y0 = (cy - r).max(0);
        let x1 = (cx + r).min(self.width as i32);
        let y1 = (cy + r).min(self.height as i32);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        for py in y0..y1 {
            for px in x0..x1 {
                if (px - cx).pow(2) + (py - cy).pow(2) <= r * r {
                    let i = (py as usize * self.width as usize + px as usize) * 4;
                    self.pixels[i..i + 4].copy_from_slice(&[255, 196, 0, 255]);
                }
            }
        }
        Some((x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
    }
}

fn flush(surface: &PixelSurface, canvas: &Canvas, damage: (i32, i32, u32, u32)) -> Result<()> {
    let (x, y, w, h) = damage;
    let stride = canvas.width as usize * 4;
    let start = y as usize * stride + x as usize * 4;
    surface.write_layer(0, x, y, w, h, &canvas.pixels[start..], stride)
}

fn main() -> Result<()> {
    App::load_from_env()?.run(|scope| {
        let canvas = Arc::new(Mutex::new(Canvas {
            width: 0,
            height: 0,
            scaling: 1.0,
            pixels: Vec::new(),
            drawing: false,
        }));
        let slot: Arc<Mutex<Option<PixelSurface>>> = Arc::default();
        let surface = {
            let canvas = canvas.clone();
            let slot = slot.clone();
            scope.create_pixel_surface(move |input| {
                let Some(surface) = slot.lock().unwrap().clone() else {
                    return Ok(false);
                };
                let mut canvas = canvas.lock().unwrap();
                match input {
                    SurfaceInput::Resize(metrics) => {
                        let (w, h) = metrics.physical_size();
                        canvas.scaling = metrics.scaling;
                        canvas.resize(w, h);
                        surface.set_layer_pixels(0, w, h, PixelFormat::Rgba8, &canvas.pixels)?;
                    }
                    SurfaceInput::Pointer(p) => {
                        let (x, y) = (p.x * canvas.scaling, p.y * canvas.scaling);
                        match p.kind {
                            PointerKind::Pressed if p.button == MouseButton::Right => {
                                let (w, h) = (canvas.width, canvas.height);
                                canvas.resize(w, h);
                                surface.write_layer(
                                    0,
                                    0,
                                    0,
                                    w,
                                    h,
                                    &canvas.pixels,
                                    w as usize * 4,
                                )?;
                            }
                            PointerKind::Pressed => {
                                canvas.drawing = true;
                                if let Some(d) = canvas.dot(x, y) {
                                    flush(&surface, &canvas, d)?;
                                }
                            }
                            PointerKind::Moved if canvas.drawing => {
                                if let Some(d) = canvas.dot(x, y) {
                                    flush(&surface, &canvas, d)?;
                                }
                            }
                            PointerKind::Released | PointerKind::CaptureLost => {
                                canvas.drawing = false
                            }
                            _ => return Ok(false),
                        }
                    }
                    _ => return Ok(false),
                }
                Ok(true)
            })?
        };
        surface.set_cursor(Cursor::Cross)?;
        *slot.lock().unwrap() = Some(surface.clone());

        let hint = TextBlock::new()?
            .text("Drag to paint, right-click to clear")?
            .margin(8.0)?;
        DockPanel::set_dock(&hint, Dock::Top)?;
        let window = Window::new()?
            .title("Pixel surface")?
            .width(640.0)?
            .height(420.0)?
            .content(Some(
                &DockPanel::new()?
                    .child(hint)?
                    .child(surface.control().clone())?,
            ))?;
        scope.mount(window)?;

        if let Some(ms) = std::env::var("RUSTOLONIA_EXAMPLE_EXIT_AFTER_MS")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            let scope = scope.clone();
            let probe = surface.clone();
            scope.clone().spawn(async move {
                if let Ok(delay) = scope.delay(std::time::Duration::from_millis(ms)) {
                    let _ = delay.await;
                }
                let s = scope.clone();
                let _ = scope.post(move || {
                    if let Ok(m) = probe.metrics() {
                        println!("surface {}x{} @{}", m.width, m.height, m.scaling);
                    }
                    let _ = s.shutdown();
                });
            })?;
        }
        Ok(())
    })
}
