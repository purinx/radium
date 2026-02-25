use std::collections::HashMap;

use super::Token;

#[derive(Debug)]
pub enum Node {
    Element {
        tag: String,
        attrs: HashMap<String, String>,
        children: Vec<Node>,
    },
    Text(String),
}

impl Node {
    pub fn print(&self, depth: usize) {
        let indent = "  ".repeat(depth);
        match self {
            Node::Element { tag, attrs, children } => {
                if attrs.is_empty() {
                    println!("{indent}<{tag}>");
                } else {
                    let attr_str: String = attrs
                        .iter()
                        .map(|(k, v)| format!(" {k}=\"{v}\""))
                        .collect();
                    println!("{indent}<{tag}{attr_str}>");
                }
                for child in children {
                    child.print(depth + 1);
                }
            }
            Node::Text(content) => {
                println!("{indent}\"{content}\"");
            }
        }
    }
}

/// Tags that are always void (never have children).
fn is_void(tag: &str) -> bool {
    matches!(
        tag,
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input"
            | "link" | "meta" | "param" | "source" | "track" | "wbr"
    )
}

/// Temporary structure used while the tree is being built.
struct Partial {
    tag: String,
    attrs: HashMap<String, String>,
    children: Vec<Node>,
}

/// Convert a flat token stream into a tree of `Node`s.
pub fn build_tree(tokens: Vec<Token>) -> Vec<Node> {
    // The bottom of the stack is a virtual root that collects top-level nodes.
    let mut stack: Vec<Partial> = vec![Partial {
        tag: String::new(),
        attrs: HashMap::new(),
        children: Vec::new(),
    }];

    for token in tokens {
        match token {
            Token::Doctype => {}
            Token::OpenTag { name, attrs, self_closing } => {
                if self_closing || is_void(&name) {
                    let node = Node::Element { tag: name, attrs, children: vec![] };
                    stack.last_mut().unwrap().children.push(node);
                } else {
                    stack.push(Partial { tag: name, attrs, children: Vec::new() });
                }
            }
            Token::CloseTag(name) => {
                // Find the matching open tag in the stack (ignore mismatches).
                let pos = stack.iter().rposition(|p| p.tag == name);
                if let Some(pos) = pos {
                    // Collapse everything above the match first.
                    while stack.len() > pos + 1 {
                        let partial = stack.pop().unwrap();
                        let node = Node::Element {
                            tag: partial.tag,
                            attrs: partial.attrs,
                            children: partial.children,
                        };
                        stack.last_mut().unwrap().children.push(node);
                    }
                    // Now pop the matching element.
                    let partial = stack.pop().unwrap();
                    let node = Node::Element {
                        tag: partial.tag,
                        attrs: partial.attrs,
                        children: partial.children,
                    };
                    stack.last_mut().unwrap().children.push(node);
                }
                // Unmatched close tags are silently ignored.
            }
            Token::Text(content) => {
                stack.last_mut().unwrap().children.push(Node::Text(content));
            }
        }
    }

    // Flush anything left open on the stack into its parent.
    while stack.len() > 1 {
        let partial = stack.pop().unwrap();
        let node = Node::Element {
            tag: partial.tag,
            attrs: partial.attrs,
            children: partial.children,
        };
        stack.last_mut().unwrap().children.push(node);
    }

    stack.pop().unwrap().children
}
