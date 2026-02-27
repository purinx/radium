mod parser;
mod layout;
mod renderer;

use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: radium <directory>");
        std::process::exit(1);
    }

    let dir = Path::new(&args[1]);

    if !dir.is_dir() {
        eprintln!("Error: '{}' is not a directory", dir.display());
        std::process::exit(1);
    }

    let html_path = dir.join("index.html");

    if !html_path.exists() {
        eprintln!("Error: no index.html found in '{}'", dir.display());
        std::process::exit(1);
    }

    let html = std::fs::read_to_string(&html_path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {e}", html_path.display());
        std::process::exit(1);
    });

    let tokens = parser::tokenize(&html);
    let nodes = parser::dom::build_tree(tokens);
    let boxes = layout::layout(&nodes, 800.0, dir);

    renderer::run(format!("radium — {}", dir.display()), boxes);
}
