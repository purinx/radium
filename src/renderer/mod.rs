use std::num::NonZeroU32;
use std::sync::Arc;

use fontdue::{Font, FontSettings};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::layout::{LayoutBox, PaintCmd};

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run(title: String, boxes: Vec<LayoutBox>) {
    let font = load_font();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        title,
        boxes,
        font,
        window: None,
        context: None,
        surface: None,
        scroll_y: 0.0,
    };
    event_loop.run_app(&mut app).unwrap();
}

// ── App state ─────────────────────────────────────────────────────────────────

struct App {
    title: String,
    boxes: Vec<LayoutBox>,
    font: Font,
    window: Option<Arc<Window>>,
    context: Option<Context<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    scroll_y: f32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let context = Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();

        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                // Extract owned values from window before mutably borrowing surface.
                let (size, scale) = match &self.window {
                    Some(w) => (w.inner_size(), w.scale_factor() as f32),
                    None => return,
                };
                let (Some(pw), Some(ph)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                else {
                    return;
                };

                if let Some(surface) = &mut self.surface {
                    surface.resize(pw, ph).unwrap();
                    let mut buffer = surface.buffer_mut().unwrap();
                    buffer.fill(0x00FFFFFF); // white background

                    // Free function: borrows self.boxes/font/scroll_y independently
                    // from self.surface (NLL field disjointness).
                    render_frame(
                        &mut buffer,
                        size.width,
                        size.height,
                        scale,
                        &self.boxes,
                        &self.font,
                        self.scroll_y,
                    );

                    buffer.present().unwrap();
                }
            }
            _ => {}
        }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render_frame(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    scale: f32,
    boxes: &[LayoutBox],
    font: &Font,
    scroll_y: f32,
) {
    for b in boxes {
        let x = b.x * scale;
        let y = (b.y - scroll_y) * scale;

        // Skip boxes fully outside the viewport.
        if y + b.height * scale < 0.0 || y > height as f32 {
            continue;
        }

        match &b.cmd {
            PaintCmd::Text { content, font_size, color, .. } => {
                blit_text(buffer, width, height, font, content, x, y, font_size * scale, *color);
            }
            PaintCmd::HLine { color } => {
                blit_hline(buffer, width, height, x as u32, y as u32, (b.width * scale) as u32, *color);
            }
        }
    }
}

// ── Glyph blitting ────────────────────────────────────────────────────────────

/// Render a string into the pixel buffer starting at (x, y) top-left.
fn blit_text(
    buffer: &mut [u32],
    buf_w: u32,
    buf_h: u32,
    font: &Font,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    color: u32,
) {
    // Ascent gives us the baseline position relative to the box top.
    let ascent = font
        .horizontal_line_metrics(font_size)
        .map(|m| m.ascent)
        .unwrap_or(font_size * 0.8);

    let baseline_y = y + ascent;
    let mut cursor_x = x;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);

        // Top-left of the glyph bitmap in screen coordinates.
        // ymin = offset from baseline to bottom edge of bitmap (positive = above baseline).
        // So top = baseline - ymin - height  (y increases downward on screen).
        let gx = (cursor_x + metrics.xmin as f32) as i32;
        let gy = (baseline_y - metrics.ymin as f32 - metrics.height as f32) as i32;

        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let alpha = bitmap[row * metrics.width + col] as u32;
                if alpha == 0 {
                    continue;
                }

                let px = gx + col as i32;
                let py = gy + row as i32;

                if px < 0 || py < 0 || px >= buf_w as i32 || py >= buf_h as i32 {
                    continue;
                }

                let idx = (py as u32 * buf_w + px as u32) as usize;
                buffer[idx] = alpha_blend(buffer[idx], color, alpha);
            }
        }

        cursor_x += metrics.advance_width;
    }
}

fn blit_hline(buffer: &mut [u32], buf_w: u32, buf_h: u32, x: u32, y: u32, width: u32, color: u32) {
    if y >= buf_h {
        return;
    }
    let x_end = (x + width).min(buf_w);
    for px in x..x_end {
        buffer[(y * buf_w + px) as usize] = color;
    }
}

/// Alpha-blend `fg` over `bg`.  Both use 0x00RRGGBB.  `alpha` is 0–255.
fn alpha_blend(bg: u32, fg: u32, alpha: u32) -> u32 {
    let ia = 255 - alpha;
    let r = ((fg >> 16 & 0xFF) * alpha + (bg >> 16 & 0xFF) * ia) / 255;
    let g = ((fg >> 8 & 0xFF) * alpha + (bg >> 8 & 0xFF) * ia) / 255;
    let b = ((fg & 0xFF) * alpha + (bg & 0xFF) * ia) / 255;
    (r << 16) | (g << 8) | b
}

// ── Font loading ──────────────────────────────────────────────────────────────

fn load_font() -> Font {
    let candidates: &[&str] = &[
        "./assets/font.ttf",
        // macOS (Supplemental fonts, installed by default)
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Verdana.ttf",
        "/System/Library/Fonts/Supplemental/Trebuchet MS.ttf",
        "/Library/Fonts/Arial.ttf",
        // Linux
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
    ];

    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            eprintln!("radium: loaded font from {path}");
            return Font::from_bytes(data.as_slice(), FontSettings::default())
                .expect("Failed to parse font file");
        }
    }

    panic!(
        "No font found. Place a TTF font at ./assets/font.ttf\nTried: {}",
        candidates.join(", ")
    );
}
