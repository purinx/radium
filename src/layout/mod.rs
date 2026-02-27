use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::parser::dom::Node;

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct LayoutBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub cmd: PaintCmd,
}

#[derive(Debug)]
pub enum PaintCmd {
    Text {
        content: String,
        font_size: f32,
        bold: bool,
        italic: bool,
        color: u32,
        underline: bool,
    },
    FillRect {
        color: u32,
    },
    HLine {
        color: u32,
    },
    Image {
        /// Raw RGBA8 pixel data.
        data: Vec<u8>,
        img_width: u32,
        img_height: u32,
    },
}

// ── Internal style state ──────────────────────────────────────────────────────

#[derive(Clone)]
struct Style {
    font_size: f32,
    bold: bool,
    italic: bool,
    color: u32,
    underline: bool,
    /// Extra left indent relative to the page margin (for list nesting).
    indent: f32,
}

impl Default for Style {
    fn default() -> Self {
        Style { font_size: 16.0, bold: false, italic: false, color: 0x000000, underline: false, indent: 0.0 }
    }
}

struct Ctx {
    pad: f32,
    width: f32,
    /// Full viewport width — used for full-bleed heading backgrounds.
    viewport_width: f32,
    /// Base directory for resolving relative paths (e.g. image src).
    base_dir: PathBuf,
    boxes: Vec<LayoutBox>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

const PAGE_PAD: f32 = 16.0;
/// Width of the gutter reserved for list markers (bullet / number).
const MARKER_INDENT: f32 = 24.0;

pub fn layout(nodes: &[Node], viewport_width: f32, base_dir: &Path) -> Vec<LayoutBox> {
    let mut ctx = Ctx {
        pad: PAGE_PAD,
        width: viewport_width - PAGE_PAD * 2.0,
        viewport_width,
        base_dir: base_dir.to_path_buf(),
        boxes: Vec::new(),
    };
    let mut y = PAGE_PAD;
    for node in nodes {
        y = layout_node(node, &mut ctx, y, &Style::default());
    }
    ctx.boxes
}

// ── Layout helpers ────────────────────────────────────────────────────────────

fn line_height(font_size: f32) -> f32 {
    font_size * 1.4
}

fn layout_node(node: &Node, ctx: &mut Ctx, y: f32, style: &Style) -> f32 {
    match node {
        Node::Text(content) => {
            let text = content.trim();
            if text.is_empty() {
                return y;
            }
            let h = line_height(style.font_size);
            ctx.boxes.push(LayoutBox {
                x: ctx.pad + style.indent,
                y,
                width: ctx.width - style.indent,
                height: h,
                cmd: PaintCmd::Text {
                    content: text.to_string(),
                    font_size: style.font_size,
                    bold: style.bold,
                    italic: style.italic,
                    color: style.color,
                    underline: style.underline,
                },
            });
            y + h
        }
        Node::Element { tag, attrs, children } => layout_element(tag, attrs, children, ctx, y, style),
    }
}

fn layout_element(tag: &str, attrs: &HashMap<String, String>, children: &[Node], ctx: &mut Ctx, y: f32, style: &Style) -> f32 {
    match tag {
        // ── Skip entirely ──────────────────────────────────────────────────
        "head" | "title" | "script" | "style" | "meta" | "link" => y,

        // ── Transparent containers ─────────────────────────────────────────
        "html" | "body" | "div" | "section" | "article" | "main" | "header" | "footer" => {
            layout_children(children, ctx, y, style)
        }

        // ── Headings ───────────────────────────────────────────────────────
        "h1" => heading(children, ctx, y, style, 32.0, 24.0, 16.0, None, None),
        "h2" => heading(children, ctx, y, style, 24.0, 20.0, 12.0, None, None),
        "h3" => heading(children, ctx, y, style, 20.0, 16.0,  8.0, None, None),

        // ── Paragraph ─────────────────────────────────────────────────────
        "p" => block(children, ctx, y, style, 0.0, 16.0, style.clone()),

        // ── Lists ──────────────────────────────────────────────────────────
        "ul" | "ol" => {
            let inner = Style { indent: style.indent + MARKER_INDENT, ..style.clone() };
            let y = y + 8.0;
            let y = layout_list(tag, children, ctx, y, &inner);
            y + 8.0
        }

        // ── Inline elements (v1: treat as block, pass style through) ───────
        "strong" => layout_children(children, ctx, y, &Style { bold: true, ..style.clone() }),
        "em"     => layout_children(children, ctx, y, &Style { italic: true, ..style.clone() }),
        "a"    => layout_children(children, ctx, y, &Style { color: 0x0000EE, underline: true, ..style.clone() }),
        "span" => layout_children(children, ctx, y, style),

        // ── Void ──────────────────────────────────────────────────────────
        "br" => y + line_height(style.font_size),
        "hr" => {
            let mid = y + 8.0;
            ctx.boxes.push(LayoutBox {
                x: ctx.pad,
                y: mid,
                width: ctx.width,
                height: 1.0,
                cmd: PaintCmd::HLine { color: 0xAAAAAA },
            });
            mid + 1.0 + 8.0
        }

        // ── Image ─────────────────────────────────────────────────────────
        "img" => layout_img(attrs, ctx, y),

        // ── Unknown: transparent ───────────────────────────────────────────
        _ => layout_children(children, ctx, y, style),
    }
}

fn layout_img(attrs: &HashMap<String, String>, ctx: &mut Ctx, y: f32) -> f32 {
    let src = match attrs.get("src") {
        Some(s) => s,
        None => return y,
    };

    let path = ctx.base_dir.join(src);
    let img = match image::open(&path) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("radium: failed to load image {}: {e}", path.display());
            return y;
        }
    };

    let rgba = img.to_rgba8();
    let (img_w, img_h) = rgba.dimensions();
    let data = rgba.into_raw();

    // Scale down proportionally if wider than the content area.
    let display_w = ctx.width.min(img_w as f32);
    let scale = display_w / img_w as f32;
    let display_h = img_h as f32 * scale;

    ctx.boxes.push(LayoutBox {
        x: ctx.pad,
        y,
        width: display_w,
        height: display_h,
        cmd: PaintCmd::Image { data, img_width: img_w, img_height: img_h },
    });

    y + display_h + 8.0
}

/// Lay out a block element with top/bottom margins.
fn block(children: &[Node], ctx: &mut Ctx, y: f32, _parent: &Style, mt: f32, mb: f32, style: Style) -> f32 {
    let y = layout_children(children, ctx, y + mt, &style);
    y + mb
}

/// Layout a heading with optional full-bleed background and bottom border.
fn heading(
    children: &[Node],
    ctx: &mut Ctx,
    y: f32,
    parent_style: &Style,
    font_size: f32,
    mt: f32,
    mb: f32,
    bg: Option<u32>,
    border: Option<u32>,
) -> f32 {
    let style = Style { font_size, bold: true, ..parent_style.clone() };
    let top = y + mt;

    // Emit background BEFORE children so it appears behind the text.
    if let Some(color) = bg {
        let lh = line_height(font_size);
        ctx.boxes.push(LayoutBox {
            x: 0.0,
            y: top - 6.0,
            width: ctx.viewport_width,
            height: lh + 12.0,
            cmd: PaintCmd::FillRect { color },
        });
    }

    let y = layout_children(children, ctx, top, &style);

    // Emit bottom border AFTER children.
    if let Some(color) = border {
        ctx.boxes.push(LayoutBox {
            x: ctx.pad,
            y: y + 4.0,
            width: ctx.width,
            height: 1.0,
            cmd: PaintCmd::HLine { color },
        });
        return y + 5.0 + mb; // 4px gap + 1px line
    }

    y + mb
}

fn layout_children(children: &[Node], ctx: &mut Ctx, y: f32, style: &Style) -> f32 {
    let mut y = y;
    for child in children {
        y = layout_node(child, ctx, y, style);
    }
    y
}

fn layout_list(list_tag: &str, children: &[Node], ctx: &mut Ctx, y: f32, style: &Style) -> f32 {
    let mut y = y;
    let mut counter = 1usize;

    // Nesting depth: how many MARKER_INDENT levels deep are we?
    let depth = (style.indent / MARKER_INDENT).round() as usize;

    for child in children {
        let Node::Element { tag, children: li_children, .. } = child else { continue };
        if tag != "li" { continue }

        let marker = if list_tag == "ol" {
            format!("{}.", counter)
        } else {
            // Different bullet symbol per nesting depth.
            match depth {
                1 => "•",
                2 => "◦",
                _ => "▪",
            }
            .to_string()
        };
        counter += 1;

        // Marker sits in the MARKER_INDENT gutter to the left of content.
        let marker_x = ctx.pad + style.indent - MARKER_INDENT;
        let h = line_height(style.font_size);
        ctx.boxes.push(LayoutBox {
            x: marker_x,
            y,
            width: MARKER_INDENT,
            height: h,
            cmd: PaintCmd::Text {
                content: marker,
                font_size: style.font_size,
                bold: style.bold,
                italic: style.italic,
                // Markers are slightly muted.
                color: 0x555555,
                underline: false,
            },
        });

        // Layout the li's children (text nodes, inline elements, nested lists).
        let after = layout_children(li_children, ctx, y, style);
        // Advance by at least one line height, then add inter-item gap.
        y = after.max(y + h) + 4.0;
    }
    y
}
