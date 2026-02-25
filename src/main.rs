mod parser;
mod layout;
mod renderer;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: radium <file.html>");
        std::process::exit(1);
    }

    let html_path = &args[1];
    let html = std::fs::read_to_string(html_path).unwrap_or_else(|e| {
        eprintln!("Error reading {html_path}: {e}");
        std::process::exit(1);
    });

    let tokens = parser::tokenize(&html);
    let nodes = parser::dom::build_tree(tokens);
    let boxes = layout::layout(&nodes, 800.0);

    for b in &boxes {
        let cmd = match &b.cmd {
            layout::PaintCmd::Text { content, font_size, bold, italic, .. } => {
                let style = match (bold, italic) {
                    (true, true)  => "bold+italic",
                    (true, false) => "bold",
                    (false, true) => "italic",
                    _             => "normal",
                };
                format!("Text({style} {font_size}px) \"{content}\"")
            }
            layout::PaintCmd::HLine { .. } => "HLine".to_string(),
        };
        println!("[x={:.0} y={:.0} w={:.0} h={:.0}] {cmd}", b.x, b.y, b.width, b.height);
    }
}
