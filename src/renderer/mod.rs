use std::num::NonZeroU32;
use std::sync::Arc;

use fontdue::{Font, FontSettings};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::layout::{LayoutBox, PaintCmd};

// ── Font set ──────────────────────────────────────────────────────────────────

/// The four faces of a typeface family.
struct FontSet {
    regular: Font,
    bold: Font,
    italic: Font,
    bold_italic: Font,
}

impl FontSet {
    fn get(&self, bold: bool, italic: bool) -> &Font {
        match (bold, italic) {
            (true,  true)  => &self.bold_italic,
            (true,  false) => &self.bold,
            (false, true)  => &self.italic,
            (false, false) => &self.regular,
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run(title: String, boxes: Vec<LayoutBox>) {
    let fonts = load_font_set();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        title,
        boxes,
        fonts,
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
    fonts: FontSet,
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

            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    // LineDelta: positive y = scroll up (content moves up = see further down).
                    // We negate so that scroll_y increases when scrolling down.
                    MouseScrollDelta::LineDelta(_, y) => -y * 40.0,
                    MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
                };
                self.scroll_by(dy);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    let page = self.window.as_ref()
                        .map(|w| w.inner_size().height as f32 / w.scale_factor() as f32 * 0.9)
                        .unwrap_or(500.0);

                    let dy: Option<f32> = match &event.logical_key {
                        Key::Named(NamedKey::ArrowDown)  => Some(40.0),
                        Key::Named(NamedKey::ArrowUp)    => Some(-40.0),
                        Key::Named(NamedKey::PageDown)
                        | Key::Named(NamedKey::Space)    => Some(page),
                        Key::Named(NamedKey::PageUp)     => Some(-page),
                        Key::Named(NamedKey::Home)       => { self.scroll_by(-f32::INFINITY); None }
                        Key::Named(NamedKey::End)        => { self.scroll_by(f32::INFINITY);  None }
                        _ => None,
                    };
                    if let Some(d) = dy { self.scroll_by(d); }
                }
            }

            WindowEvent::Resized(_) => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
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
                    buffer.fill(0x00FFFFFF);

                    render_frame(
                        &mut buffer,
                        size.width,
                        size.height,
                        scale,
                        &self.boxes,
                        &self.fonts,
                        self.scroll_y,
                    );

                    buffer.present().unwrap();
                }
            }
            _ => {}
        }
    }
}

// ── Scroll helpers ────────────────────────────────────────────────────────────

impl App {
    /// Maximum logical-pixel scroll offset for the current viewport.
    fn max_scroll(&self) -> f32 {
        let doc_h = self.boxes.iter()
            .map(|b| b.y + b.height)
            .fold(0.0_f32, f32::max);

        let (viewport_h, scale) = self.window.as_ref()
            .map(|w| (w.inner_size().height, w.scale_factor() as f32))
            .unwrap_or((600, 1.0));

        let viewport_logical = viewport_h as f32 / scale;
        (doc_h - viewport_logical + 16.0).max(0.0)
    }

    fn scroll_by(&mut self, dy: f32) {
        self.scroll_y = (self.scroll_y + dy).clamp(0.0, self.max_scroll());
        if let Some(w) = &self.window {
            w.request_redraw();
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
    fonts: &FontSet,
    scroll_y: f32,
) {
    // ── Document boxes ────────────────────────────────────────────────────
    for b in boxes {
        let x = b.x * scale;
        let y = (b.y - scroll_y) * scale;

        if y + b.height * scale < 0.0 || y > height as f32 {
            continue;
        }

        match &b.cmd {
            PaintCmd::FillRect { color } => {
                blit_rect(
                    buffer, width, height,
                    x as u32, y as u32,
                    (b.width * scale) as u32, (b.height * scale) as u32,
                    *color,
                );
            }
            PaintCmd::Text { content, font_size, bold, italic, color } => {
                let font = fonts.get(*bold, *italic);
                blit_text(
                    buffer, width, height,
                    font, content,
                    x, y, font_size * scale, *color,
                );
            }
            PaintCmd::HLine { color } => {
                blit_hline(
                    buffer, width, height,
                    x as u32, y as u32,
                    (b.width * scale) as u32, *color,
                );
            }
        }
    }

    // ── Scrollbar ─────────────────────────────────────────────────────────
    let doc_h_phys = boxes.iter()
        .map(|b| (b.y + b.height) * scale)
        .fold(0.0_f32, f32::max);

    if doc_h_phys > height as f32 {
        draw_scrollbar(buffer, width, height, doc_h_phys, scroll_y * scale);
    }
}

// ── Glyph blitting ────────────────────────────────────────────────────────────

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
    let ascent = font
        .horizontal_line_metrics(font_size)
        .map(|m| m.ascent)
        .unwrap_or(font_size * 0.8);

    let baseline_y = y + ascent;
    let mut cursor_x = x;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);

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

fn blit_rect(buffer: &mut [u32], buf_w: u32, buf_h: u32, x: u32, y: u32, w: u32, h: u32, color: u32) {
    let x_end = (x + w).min(buf_w);
    let y_end = (y + h).min(buf_h);
    for row in y..y_end {
        for col in x..x_end {
            buffer[(row * buf_w + col) as usize] = color;
        }
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

/// Draw a minimal scrollbar on the right edge of the buffer.
/// All coordinates are physical pixels.
fn draw_scrollbar(buffer: &mut [u32], width: u32, height: u32, doc_h: f32, scroll_y: f32) {
    const BAR_W: u32 = 6;
    const MIN_THUMB: u32 = 24;
    const TRACK_COLOR: u32 = 0xF0F0F0;
    const THUMB_COLOR: u32 = 0xA8A8A8;

    let bar_x = width.saturating_sub(BAR_W);

    // Track (full height, light gray).
    for row in 0..height {
        for col in bar_x..width {
            buffer[(row * width + col) as usize] = TRACK_COLOR;
        }
    }

    // Thumb: height proportional to viewport / document ratio.
    let ratio = (height as f32 / doc_h).min(1.0);
    let thumb_h = ((height as f32 * ratio) as u32).max(MIN_THUMB);
    let max_scroll = (doc_h - height as f32).max(1.0);
    let thumb_y = ((scroll_y / max_scroll) * (height - thumb_h) as f32) as u32;
    let thumb_y = thumb_y.min(height.saturating_sub(thumb_h));

    for row in thumb_y..(thumb_y + thumb_h).min(height) {
        for col in bar_x..width {
            buffer[(row * width + col) as usize] = THUMB_COLOR;
        }
    }
}

fn alpha_blend(bg: u32, fg: u32, alpha: u32) -> u32 {
    let ia = 255 - alpha;
    let r = ((fg >> 16 & 0xFF) * alpha + (bg >> 16 & 0xFF) * ia) / 255;
    let g = ((fg >>  8 & 0xFF) * alpha + (bg >>  8 & 0xFF) * ia) / 255;
    let b = ((fg       & 0xFF) * alpha + (bg       & 0xFF) * ia) / 255;
    (r << 16) | (g << 8) | b
}

// ── Font loading ──────────────────────────────────────────────────────────────

fn try_load_bytes(candidates: &[&str]) -> Option<Vec<u8>> {
    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            eprintln!("radium: loaded font from {path}");
            return Some(data);
        }
    }
    None
}

fn make_font(data: &[u8]) -> Font {
    Font::from_bytes(data, FontSettings::default()).expect("Failed to parse font file")
}

fn load_font_set() -> FontSet {
    // Regular — required.
    let regular_data = try_load_bytes(&[
        "./assets/font.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Verdana.ttf",
        "/Library/Fonts/Arial.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
    ])
    .expect("No font found. Place a TTF font at ./assets/font.ttf");

    // Variants — fall back to regular if not found.
    let bold_data = try_load_bytes(&[
        "./assets/font-bold.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
    ]);

    let italic_data = try_load_bytes(&[
        "./assets/font-italic.ttf",
        "/System/Library/Fonts/Supplemental/Arial Italic.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Italic.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Oblique.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Oblique.ttf",
    ]);

    let bold_italic_data = try_load_bytes(&[
        "./assets/font-bold-italic.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-BoldItalic.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-BoldOblique.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-BoldOblique.ttf",
    ]);

    let regular    = make_font(&regular_data);
    let bold       = bold_data.as_deref()
                              .map(make_font)
                              .unwrap_or_else(|| make_font(&regular_data));
    let italic     = italic_data.as_deref()
                                .map(make_font)
                                .unwrap_or_else(|| make_font(&regular_data));
    let bold_italic = bold_italic_data.as_deref()
                                      .map(make_font)
                                      // Prefer bold face over regular as fallback.
                                      .or_else(|| bold_data.as_deref().map(make_font))
                                      .unwrap_or_else(|| make_font(&regular_data));

    FontSet { regular, bold, italic, bold_italic }
}
