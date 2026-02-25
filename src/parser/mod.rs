pub mod dom;

use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug)]
pub enum Token {
    Doctype,
    OpenTag {
        name: String,
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
                    let self_closing = skip_tag_body(&mut chars);
                    tokens.push(Token::OpenTag {
                        name: name.to_lowercase(),
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

/// Consume everything up to and including the closing `>`, returning whether
/// the tag is self-closing (`/>`).
fn skip_tag_body(chars: &mut Peekable<Chars<'_>>) -> bool {
    let mut self_closing = false;
    let mut in_quote: Option<char> = None;

    loop {
        match chars.next() {
            None => break,
            Some(c) => match (in_quote, c) {
                // Enter a quoted attribute value.
                (None, '"') | (None, '\'') => in_quote = Some(c),
                // Exit a quoted attribute value.
                (Some(q), c) if c == q => in_quote = None,
                // Detect self-closing marker outside quotes.
                (None, '/') => {
                    if chars.peek() == Some(&'>') {
                        chars.next();
                        self_closing = true;
                        break;
                    }
                }
                // End of tag.
                (None, '>') => break,
                _ => {}
            },
        }
    }

    self_closing
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
