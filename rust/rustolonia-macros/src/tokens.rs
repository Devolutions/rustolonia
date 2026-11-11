use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

#[derive(Debug)]
pub struct MacroError {
    pub span: Span,
    pub message: String,
}

impl MacroError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    pub fn to_compile_error(&self) -> TokenStream {
        let mut out = Vec::<TokenTree>::new();
        let s = self.span;
        out.push(TokenTree::Punct(with_span(
            Punct::new(':', Spacing::Joint),
            s,
        )));
        out.push(TokenTree::Punct(with_span(
            Punct::new(':', Spacing::Alone),
            s,
        )));
        out.push(TokenTree::Ident(Ident::new("core", s)));
        out.push(TokenTree::Punct(with_span(
            Punct::new(':', Spacing::Joint),
            s,
        )));
        out.push(TokenTree::Punct(with_span(
            Punct::new(':', Spacing::Alone),
            s,
        )));
        out.push(TokenTree::Ident(Ident::new("compile_error", s)));
        out.push(TokenTree::Punct(with_span(
            Punct::new('!', Spacing::Alone),
            s,
        )));
        let mut msg_lit = Literal::string(&self.message);
        msg_lit.set_span(s);
        let mut group = Group::new(
            Delimiter::Parenthesis,
            TokenStream::from(TokenTree::Literal(msg_lit)),
        );
        group.set_span(s);
        out.push(TokenTree::Group(group));
        out.into_iter().collect()
    }
}

pub fn with_span(mut p: Punct, span: Span) -> Punct {
    p.set_span(span);
    p
}

pub fn to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let chars: Vec<char> = name.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_ascii_digit()) {
                out.push('_');
            } else if i > 0
                && chars[i - 1].is_uppercase()
                && chars.get(i + 1).is_some_and(|next| next.is_lowercase())
            {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn is_punct(tt: &TokenTree, ch: char) -> bool {
    matches!(tt, TokenTree::Punct(p) if p.as_char() == ch)
}

pub fn is_double_colon(tokens: &[TokenTree], index: usize) -> bool {
    matches!(
        (tokens.get(index), tokens.get(index + 1)),
        (Some(TokenTree::Punct(a)), Some(TokenTree::Punct(b)))
            if a.as_char() == ':' && a.spacing() == Spacing::Joint && b.as_char() == ':'
    )
}

pub fn unquote_string_literal(lit: &Literal) -> Option<String> {
    let text = lit.to_string();
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                '\'' => out.push('\''),
                '0' => out.push('\0'),
                'x' => {
                    let h1 = chars.next()?;
                    let h2 = chars.next()?;
                    let byte = u8::from_str_radix(&format!("{h1}{h2}"), 16).ok()?;
                    out.push(byte as char);
                }
                'u' => {
                    if chars.next()? != '{' {
                        return None;
                    }
                    let mut hex = String::new();
                    for ch in chars.by_ref() {
                        if ch == '}' {
                            break;
                        }
                        hex.push(ch);
                    }
                    let code = u32::from_str_radix(&hex, 16).ok()?;
                    out.push(char::from_u32(code)?);
                }
                _ => return None,
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

pub fn parse_stream(src: &str) -> TokenStream {
    src.parse().expect("valid generated Rust TokenStream")
}
