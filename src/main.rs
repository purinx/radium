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

    renderer::run(format!("radium — {html_path}"), boxes);
}
