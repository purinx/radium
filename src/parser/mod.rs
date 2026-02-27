pub mod dom;

use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug)]
pub enum Token {
    Doctype,
    OpenTag {
        name: String,
        attrs: HashMap<String, String>,
        self_closing: bool,
    },
    CloseTag(String),
    Text(String),
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while chars.peek().is_some() {
        if chars.peek() == Some(&'<') {
            chars.next(); // consume '<'

            match chars.peek() {
                Some(&'/') => {
                    chars.next();
                    let name = read_name(&mut chars);
                    skip_until(&mut chars, '>');
                    chars.next(); // consume '>'
                    if !name.is_empty() {
                        tokens.push(Token::CloseTag(name.to_lowercase()));
                    }
                }
                Some(&'!') => {
                    chars.next();
                    skip_until(&mut chars, '>');
                    chars.next();
                    tokens.push(Token::Doctype);
                }
                Some(&'?') => {
                    skip_until(&mut chars, '>');
                    chars.next();
                }
                _ => {
                    let name = read_name(&mut chars);
                    if name.is_empty() {
                        skip_until(&mut chars, '>');
                        chars.next();
                        continue;
                    }
                    let (attrs, self_closing) = parse_tag_body(&mut chars);
                    tokens.push(Token::OpenTag {
                        name: name.to_lowercase(),
                        attrs,
                        self_closing,
                    });
                }
            }
        } else {
            let text = read_text(&mut chars);
            let collapsed = collapse_whitespace(&text);
            if !collapsed.is_empty() {
                tokens.push(Token::Text(collapsed));
            }
        }
    }

    tokens
}

fn read_name(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut name = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_alphanumeric() || c == '-' || c == '_' || c == ':' {
            name.push(c);
            chars.next();
        } else {
            break;
        }
    }
    name
}

/// Parse tag attributes and consume through the closing `>`.
/// Returns the attribute map and whether the tag is self-closing (`/>`).
fn parse_tag_body(chars: &mut Peekable<Chars<'_>>) -> (HashMap<String, String>, bool) {
    let mut attrs = HashMap::new();
    let mut self_closing = false;

    loop {
        // Skip whitespace between attributes.
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }

        match chars.peek().copied() {
            None => break,
            Some('>') => {
                chars.next();
                break;
            }
            Some('/') => {
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    self_closing = true;
                }
                break;
            }
            _ => {
                let name = read_attr_name(chars);
                if name.is_empty() {
                    chars.next(); // skip unexpected character
                    continue;
                }

                // Skip whitespace before optional `=`.
                while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                    chars.next();
                }

                if chars.peek() == Some(&'=') {
                    chars.next(); // consume '='
                    while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                        chars.next();
                    }
                    let value = read_attr_value(chars);
                    attrs.insert(name.to_lowercase(), value);
                } else {
                    // Boolean attribute (no value).
                    attrs.insert(name.to_lowercase(), String::new());
                }
            }
        }
    }

    (attrs, self_closing)
}

fn read_attr_name(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut name = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() || c == '=' || c == '>' || c == '/' {
            break;
        }
        name.push(c);
        chars.next();
    }
    name
}

fn read_attr_value(chars: &mut Peekable<Chars<'_>>) -> String {
    match chars.peek().copied() {
        Some(q @ '"') | Some(q @ '\'') => {
            chars.next(); // consume opening quote
            let mut value = String::new();
            for c in chars.by_ref() {
                if c == q {
                    break;
                }
                value.push(c);
            }
            value
        }
        _ => {
            // Unquoted value: read until whitespace or '>'.
            let mut value = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == '>' {
                    break;
                }
                value.push(c);
                chars.next();
            }
            value
        }
    }
}

fn read_text(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut text = String::new();
    while let Some(&c) = chars.peek() {
        if c == '<' {
            break;
        }
        text.push(c);
        chars.next();
    }
    text
}

fn skip_until(chars: &mut Peekable<Chars<'_>>, stop: char) {
    while let Some(&c) = chars.peek() {
        if c == stop {
            break;
        }
        chars.next();
    }
}

fn collapse_whitespace(s: &str) -> String {
    let mut result = String::new();
    let mut prev_ws = true; // trim leading whitespace
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                result.push(' ');
                prev_ws = true;
            }
        } else {
            result.push(c);
            prev_ws = false;
        }
    }
    result.trim_end().to_string()
}
