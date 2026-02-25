use super::Token;

#[derive(Debug)]
pub enum Node {
    Element {
        tag: String,
        children: Vec<Node>,
    },
    Text(String),
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
    children: Vec<Node>,
}

/// Convert a flat token stream into a tree of `Node`s.
pub fn build_tree(tokens: Vec<Token>) -> Vec<Node> {
    let mut stack: Vec<Partial> = vec![Partial {
        tag: String::new(),
        children: Vec::new(),
    }];

    for token in tokens {
        match token {
            Token::Doctype => {}
            Token::OpenTag { name, self_closing, .. } => {
                if self_closing || is_void(&name) {
                    let node = Node::Element { tag: name, children: vec![] };
                    stack.last_mut().unwrap().children.push(node);
                } else {
                    stack.push(Partial { tag: name, children: Vec::new() });
                }
            }
            Token::CloseTag(name) => {
                let pos = stack.iter().rposition(|p| p.tag == name);
                if let Some(pos) = pos {
                    while stack.len() > pos + 1 {
                        let partial = stack.pop().unwrap();
                        let node = Node::Element { tag: partial.tag, children: partial.children };
                        stack.last_mut().unwrap().children.push(node);
                    }
                    let partial = stack.pop().unwrap();
                    let node = Node::Element { tag: partial.tag, children: partial.children };
                    stack.last_mut().unwrap().children.push(node);
                }
            }
            Token::Text(content) => {
                stack.last_mut().unwrap().children.push(Node::Text(content));
            }
        }
    }

    while stack.len() > 1 {
        let partial = stack.pop().unwrap();
        let node = Node::Element { tag: partial.tag, children: partial.children };
        stack.last_mut().unwrap().children.push(node);
    }

    stack.pop().unwrap().children
}
