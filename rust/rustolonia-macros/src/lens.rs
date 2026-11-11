use crate::tokens::{is_punct, parse_stream, MacroError};
use proc_macro::{Delimiter, Ident, Spacing, Span, TokenStream, TokenTree};

pub fn expand_derive_lens(input: TokenStream) -> Result<TokenStream, MacroError> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let mut idx = 0;

    // Skip attributes and visibility to find `struct <Name>`
    while idx < tokens.len() {
        if let TokenTree::Ident(id) = &tokens[idx] {
            if id.to_string() == "struct" {
                idx += 1;
                break;
            } else if id.to_string() == "enum" || id.to_string() == "union" {
                return Err(MacroError::new(
                    id.span(),
                    "#[derive(Lens)] is only supported on named structs",
                ));
            }
        }
        idx += 1;
    }

    let Some(TokenTree::Ident(struct_ident)) = tokens.get(idx) else {
        return Err(MacroError::new(
            Span::call_site(),
            "#[derive(Lens)] could not find struct name",
        ));
    };
    let struct_name = struct_ident.to_string();
    idx += 1;

    // Skip any generics/where clause until `{ ... }` body
    let mut brace_group = None;
    while idx < tokens.len() {
        if let TokenTree::Group(g) = &tokens[idx] {
            if g.delimiter() == Delimiter::Brace {
                brace_group = Some(g.stream());
                break;
            }
        }
        idx += 1;
    }

    let Some(fields_stream) = brace_group else {
        return Err(MacroError::new(
            struct_ident.span(),
            "#[derive(Lens)] requires a struct with named `{ ... }` fields",
        ));
    };

    let fields = parse_named_fields(fields_stream)?;
    let mut methods = String::new();
    for (field_name, field_ty) in fields {
        let method_name = format!("lens_{field_name}");
        methods.push_str(&format!(
            "    pub fn {method_name}(signal: ::rustolonia::Signal<Self>) -> ::rustolonia::PropertyPath<{field_ty}> {{\n\
             \x20       ::rustolonia::PropertyPath::writable(\n\
             \x20           move || ::core::result::Result::Ok(signal.with(|__s| ::core::clone::Clone::clone(&__s.{field_name}))),\n\
             \x20           move |__next| {{\n\
             \x20               signal.update(|__s| __s.{field_name} = __next);\n\
             \x20               ::core::result::Result::Ok(())\n\
             \x20           }},\n\
             \x20       )\n\
             \x20   }}\n"
        ));
    }

    Ok(parse_stream(&format!(
        "impl {struct_name} {{\n{methods}}}\n"
    )))
}

fn parse_named_fields(stream: TokenStream) -> Result<Vec<(String, String)>, MacroError> {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        // Skip field attributes `#[...]`
        while i < tokens.len() && is_punct(&tokens[i], '#') {
            i += 1;
            if i < tokens.len() {
                i += 1;
            }
        }
        if i >= tokens.len() {
            break;
        }
        // Skip visibility (`pub`, `pub(crate)`, etc.)
        if let TokenTree::Ident(id) = &tokens[i] {
            if id.to_string() == "pub" {
                i += 1;
                if let Some(TokenTree::Group(g)) = tokens.get(i) {
                    if g.delimiter() == Delimiter::Parenthesis {
                        i += 1;
                    }
                }
            }
        }
        let Some(TokenTree::Ident(field_ident)) = tokens.get(i) else {
            return Err(MacroError::new(
                tokens[i].span(),
                "expected field identifier in #[derive(Lens)] struct",
            ));
        };
        let field_name = field_ident.to_string();
        i += 1;
        if i >= tokens.len() || !is_punct(&tokens[i], ':') {
            return Err(MacroError::new(
                field_ident.span(),
                "expected ':' after field name",
            ));
        }
        i += 1;
        // Collect type tokens until top-level `,`
        let mut ty_tokens = Vec::new();
        let mut angle_depth = 0usize;
        while i < tokens.len() {
            if angle_depth == 0 && is_punct(&tokens[i], ',') {
                i += 1;
                break;
            }
            if is_punct(&tokens[i], '<') {
                angle_depth += 1;
            } else if is_punct(&tokens[i], '>') && angle_depth > 0 {
                angle_depth -= 1;
            }
            ty_tokens.push(tokens[i].clone());
            i += 1;
        }
        let ty_stream: TokenStream = ty_tokens.into_iter().collect();
        out.push((field_name, ty_stream.to_string()));
    }
    Ok(out)
}

/// Syntax: `lens!(signal => field.subfield)` or `lens!(mut signal => field.subfield)`
pub fn expand_lens(input: TokenStream) -> Result<TokenStream, MacroError> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    if tokens.is_empty() {
        return Err(MacroError::new(
            Span::call_site(),
            "lens! expects `signal => field` or `mut signal => field`",
        ));
    }

    let mut i = 0;
    let writable = if let TokenTree::Ident(id) = &tokens[0] {
        if id.to_string() == "mut" {
            i += 1;
            true
        } else {
            false
        }
    } else {
        false
    };

    // Find `=>`
    let mut arrow_pos = None;
    for pos in i..tokens.len().saturating_sub(1) {
        if let (TokenTree::Punct(eq), TokenTree::Punct(gt)) = (&tokens[pos], &tokens[pos + 1]) {
            if eq.as_char() == '=' && eq.spacing() == Spacing::Joint && gt.as_char() == '>' {
                arrow_pos = Some(pos);
                break;
            }
        }
    }

    let Some(arrow) = arrow_pos else {
        return Err(MacroError::new(
            tokens[0].span(),
            "lens! expects `=>` separating the signal expression and field path, e.g. `lens!(profile => title)`",
        ));
    };

    let sig_stream: TokenStream = tokens[i..arrow].iter().cloned().collect();
    let path_tokens = &tokens[arrow + 2..];
    if path_tokens.is_empty() {
        return Err(MacroError::new(
            tokens[arrow].span(),
            "lens! expects one or more `.`-separated field names after `=>`",
        ));
    }

    let mut fields = Vec::<Ident>::new();
    let mut p = 0;
    while p < path_tokens.len() {
        match &path_tokens[p] {
            TokenTree::Ident(id) => fields.push(id.clone()),
            other => {
                return Err(MacroError::new(
                    other.span(),
                    "expected field identifier in lens! path",
                ))
            }
        }
        p += 1;
        if p < path_tokens.len() {
            if !is_punct(&path_tokens[p], '.') {
                return Err(MacroError::new(
                    path_tokens[p].span(),
                    "expected '.' between field names in lens! path",
                ));
            }
            p += 1;
        }
    }

    let access_chain = fields
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .join(".");
    let sig_src = sig_stream.to_string();

    if writable {
        Ok(parse_stream(&format!(
            "{{\n\
             \x20   let __sig = {sig_src};\n\
             \x20   ::rustolonia::PropertyPath::writable(\n\
             \x20       move || ::core::result::Result::Ok(__sig.with(|__v| ::core::clone::Clone::clone(&__v.{access_chain}))),\n\
             \x20       move |__next| {{\n\
             \x20           __sig.update(|__v| __v.{access_chain} = __next);\n\
             \x20           ::core::result::Result::Ok(())\n\
             \x20       }},\n\
             \x20   )\n\
             }}"
        )))
    } else {
        Ok(parse_stream(&format!(
            "{{\n\
             \x20   let __sig = {sig_src};\n\
             \x20   ::rustolonia::PropertyPath::read_only(\n\
             \x20       move || ::core::result::Result::Ok(__sig.with(|__v| ::core::clone::Clone::clone(&__v.{access_chain}))),\n\
             \x20   )\n\
             }}"
        )))
    }
}
