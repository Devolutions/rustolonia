use crate::tokens::{is_punct, parse_stream, unquote_string_literal, MacroError};
use proc_macro::{Delimiter, Group, Span, TokenStream, TokenTree};

pub fn expand_color(input: TokenStream) -> Result<TokenStream, MacroError> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    if tokens.is_empty() {
        return Err(MacroError::new(
            Span::call_site(),
            "color! expects a hex string literal such as color!(\"#1e293b\") or color!(\"#1e293b80\")",
        ));
    }
    if tokens.len() == 1 {
        if let TokenTree::Literal(lit) = &tokens[0] {
            if let Some(s) = unquote_string_literal(lit) {
                let (r, g, b, a) = parse_css_hex(&s, lit.span())?;
                return Ok(parse_stream(&format!(
                    "::rustolonia::Color::rgba({r}u8, {g}u8, {b}u8, {a}u8)"
                )));
            }
            let raw = lit.to_string();
            if let Some(hex_digits) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
                let clean: String = hex_digits.chars().filter(|&c| c != '_').collect();
                if clean.len() == 6 {
                    let val = u32::from_str_radix(&clean, 16).map_err(|_| {
                        MacroError::new(lit.span(), format!("invalid hex color literal '{raw}'"))
                    })?;
                    let r = ((val >> 16) & 0xFF) as u8;
                    let g = ((val >> 8) & 0xFF) as u8;
                    let b = (val & 0xFF) as u8;
                    return Ok(parse_stream(&format!(
                        "::rustolonia::Color::rgb({r}u8, {g}u8, {b}u8)"
                    )));
                }
                if clean.len() == 8 {
                    let val = u32::from_str_radix(&clean, 16).map_err(|_| {
                        MacroError::new(lit.span(), format!("invalid hex color literal '{raw}'"))
                    })?;
                    let r = ((val >> 24) & 0xFF) as u8;
                    let g = ((val >> 16) & 0xFF) as u8;
                    let b = ((val >> 8) & 0xFF) as u8;
                    let a = (val & 0xFF) as u8;
                    return Ok(parse_stream(&format!(
                        "::rustolonia::Color::rgba({r}u8, {g}u8, {b}u8, {a}u8)"
                    )));
                }
                return Err(MacroError::new(
                    lit.span(),
                    "integer color! literal must have 6 (0xRRGGBB) or 8 (0xRRGGBBAA) hex digits",
                ));
            }
        }
    }
    Err(MacroError::new(
        tokens[0].span(),
        "color! expects a hex string literal (\"#RRGGBB\" or \"#RRGGBBAA\") or 0xRRGGBB / 0xRRGGBBAA integer",
    ))
}

fn parse_css_hex(s: &str, span: Span) -> Result<(u8, u8, u8, u8), MacroError> {
    let hex = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if !hex.is_ascii() {
        return Err(MacroError::new(
            span,
            format!("invalid hex color '{s}': non-ASCII characters"),
        ));
    }
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?
                * 17;
            let g = u8::from_str_radix(&hex[1..2], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?
                * 17;
            let b = u8::from_str_radix(&hex[2..3], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?
                * 17;
            Ok((r, g, b, 255))
        }
        4 => {
            let r = u8::from_str_radix(&hex[0..1], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?
                * 17;
            let g = u8::from_str_radix(&hex[1..2], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?
                * 17;
            let b = u8::from_str_radix(&hex[2..3], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?
                * 17;
            let a = u8::from_str_radix(&hex[3..4], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?
                * 17;
            Ok((r, g, b, a))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?;
            Ok((r, g, b, 255))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?;
            let a = u8::from_str_radix(&hex[6..8], 16)
                .map_err(|_| MacroError::new(span, format!("invalid hex color '{s}'")))?;
            Ok((r, g, b, a))
        }
        _ => Err(MacroError::new(
            span,
            format!("invalid hex color '{s}': expected 3, 4, 6, or 8 hex digits (#RGB, #RGBA, #RRGGBB, #RRGGBBAA)"),
        )),
    }
}

pub fn expand_tracks(input: TokenStream) -> Result<TokenStream, MacroError> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    if tokens.is_empty() {
        return Ok(parse_stream("::rustolonia::GridTracks::new()"));
    }

    if tokens.len() == 1 {
        if let TokenTree::Literal(lit) = &tokens[0] {
            if let Some(s) = unquote_string_literal(lit) {
                return compile_track_string(&s, lit.span());
            }
        }
    }

    let mut segments: Vec<(Span, String)> = Vec::new();
    let mut current = String::new();
    let mut current_span: Option<Span> = None;
    for tt in &tokens {
        if is_punct(tt, ',') {
            if let Some(span) = current_span.take() {
                segments.push((span, std::mem::take(&mut current)));
            } else {
                return Err(MacroError::new(
                    tt.span(),
                    "empty track segment before ',' in tracks!",
                ));
            }
        } else {
            if current_span.is_none() {
                current_span = Some(tt.span());
            }
            current.push_str(&tt.to_string());
        }
    }
    if let Some(span) = current_span {
        segments.push((span, current));
    }

    let mut exprs = Vec::with_capacity(segments.len());
    for (span, seg) in segments {
        exprs.push(compile_single_track(&seg, span)?);
    }
    Ok(parse_stream(&format!(
        "::rustolonia::GridTracks(::std::vec![{}])",
        exprs.join(", ")
    )))
}

fn compile_track_string(s: &str, span: Span) -> Result<TokenStream, MacroError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(parse_stream("::rustolonia::GridTracks::new()"));
    }
    let mut exprs = Vec::new();
    for part in trimmed
        .split([',', ' '])
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        exprs.push(compile_single_track(part, span)?);
    }
    Ok(parse_stream(&format!(
        "::rustolonia::GridTracks(::std::vec![{}])",
        exprs.join(", ")
    )))
}

fn compile_single_track(raw: &str, span: Span) -> Result<String, MacroError> {
    let trimmed = raw.trim();
    let (len_str, group) = match trimmed.split_once('@') {
        Some((l, g)) => {
            let g = g.trim();
            if g.is_empty() || g.contains('@') {
                return Err(MacroError::new(
                    span,
                    format!("invalid shared size group in grid track '{trimmed}'"),
                ));
            }
            (l.trim(), Some(g))
        }
        None => (trimmed, None),
    };
    let base = if len_str.eq_ignore_ascii_case("auto") {
        "::rustolonia::GridTrack::auto()".to_string()
    } else if let Some(star_prefix) = len_str.strip_suffix('*') {
        let star_prefix = star_prefix.trim();
        if star_prefix.is_empty() {
            "::rustolonia::GridTrack::star(1.0_f64)".to_string()
        } else {
            let w: f64 = star_prefix.parse().map_err(|_| {
                MacroError::new(
                    span,
                    format!("invalid star weight in grid track '{trimmed}'"),
                )
            })?;
            if !w.is_finite() || w < 0.0 {
                return Err(MacroError::new(
                    span,
                    format!("grid star weight must be non-negative finite in '{trimmed}'"),
                ));
            }
            format!("::rustolonia::GridTrack::star({w:?}_f64)")
        }
    } else {
        let num_str = len_str
            .strip_suffix("px")
            .or_else(|| len_str.strip_suffix("PX"))
            .unwrap_or(len_str)
            .trim();
        let px: f64 = num_str.parse().map_err(|_| {
            MacroError::new(
                span,
                format!("invalid grid track '{trimmed}': expected Auto, *, <N>*, or pixel number"),
            )
        })?;
        if !px.is_finite() || px < 0.0 {
            return Err(MacroError::new(
                span,
                format!("grid pixel track must be non-negative finite in '{trimmed}'"),
            ));
        }
        format!("::rustolonia::GridTrack::px({px:?}_f64)")
    };

    if let Some(group) = group {
        Ok(format!("{base}.shared_group({group:?})"))
    } else {
        Ok(base)
    }
}

pub fn expand_fmt(input: TokenStream) -> TokenStream {
    let mut body = Vec::<TokenTree>::new();
    let format_prefix: TokenStream = parse_stream("::std::format!");
    body.extend(format_prefix);
    body.push(TokenTree::Group(Group::new(Delimiter::Parenthesis, input)));
    let body_stream: TokenStream = body.into_iter().collect();

    let mut out = parse_stream("move || ");
    out.extend(std::iter::once(TokenTree::Group(Group::new(
        Delimiter::Brace,
        body_stream,
    ))));
    out
}
