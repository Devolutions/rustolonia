use crate::tokens::{is_double_colon, is_punct, to_snake_case, with_span, MacroError};
use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};

pub fn expand_rsx(input: TokenStream) -> Result<TokenStream, MacroError> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    if tokens.is_empty() {
        return Err(MacroError::new(
            Span::call_site(),
            "rsx! expects a root control or component, e.g. `rsx! { Window { ... } }`",
        ));
    }
    let mut pos = 0;
    let out = parse_element(&tokens, &mut pos)?;
    if pos < tokens.len() {
        return Err(MacroError::new(
            tokens[pos].span(),
            "unexpected trailing tokens after root rsx! element; rsx! produces a single root element",
        ));
    }
    Ok(out)
}

fn parse_path(tokens: &[TokenTree], pos: &mut usize) -> Option<Vec<TokenTree>> {
    let start = *pos;
    let mut path = Vec::new();
    if is_double_colon(tokens, *pos) {
        path.push(tokens[*pos].clone());
        path.push(tokens[*pos + 1].clone());
        *pos += 2;
    }
    match tokens.get(*pos) {
        Some(TokenTree::Ident(id)) => {
            let s = id.to_string();
            if matches!(s.as_str(), "if" | "else" | "for" | "show" | "ref") {
                *pos = start;
                return None;
            }
            path.push(tokens[*pos].clone());
            *pos += 1;
        }
        _ => {
            *pos = start;
            return None;
        }
    }
    while is_double_colon(tokens, *pos) {
        if let Some(TokenTree::Ident(_)) = tokens.get(*pos + 2) {
            path.push(tokens[*pos].clone());
            path.push(tokens[*pos + 1].clone());
            path.push(tokens[*pos + 2].clone());
            *pos += 3;
        } else {
            break;
        }
    }
    Some(path)
}

fn parse_element(tokens: &[TokenTree], pos: &mut usize) -> Result<TokenStream, MacroError> {
    let Some(path) = parse_path(tokens, pos) else {
        return Err(MacroError::new(
            tokens.get(*pos).map_or_else(Span::call_site, |t| t.span()),
            "expected control or component name in rsx!",
        ));
    };
    let head_span = path.last().unwrap().span();
    let Some(TokenTree::Group(brace_group)) = tokens.get(*pos) else {
        return Err(MacroError::new(
            head_span,
            "expected `{ ... }` body after element name in rsx!",
        ));
    };
    if brace_group.delimiter() != Delimiter::Brace {
        return Err(MacroError::new(
            brace_group.span(),
            "expected `{ ... }` body after element name in rsx!",
        ));
    }
    *pos += 1;

    // Emit `{ let mut __el = <Path>::el(); <stmts> __el }`
    let mut block_tokens = Vec::<TokenTree>::new();
    let el_ident = Ident::new("__el", Span::mixed_site());

    // `let mut __el = <Path>::el();`
    block_tokens.push(TokenTree::Ident(Ident::new("let", head_span)));
    block_tokens.push(TokenTree::Ident(Ident::new("mut", head_span)));
    block_tokens.push(TokenTree::Ident(el_ident.clone()));
    block_tokens.push(TokenTree::Punct(with_span(
        Punct::new('=', Spacing::Alone),
        head_span,
    )));
    block_tokens.extend(path);
    block_tokens.push(TokenTree::Punct(with_span(
        Punct::new(':', Spacing::Joint),
        head_span,
    )));
    block_tokens.push(TokenTree::Punct(with_span(
        Punct::new(':', Spacing::Alone),
        head_span,
    )));
    block_tokens.push(TokenTree::Ident(Ident::new("el", head_span)));
    let mut empty_parens = Group::new(Delimiter::Parenthesis, TokenStream::new());
    empty_parens.set_span(head_span);
    block_tokens.push(TokenTree::Group(empty_parens));
    block_tokens.push(TokenTree::Punct(with_span(
        Punct::new(';', Spacing::Alone),
        head_span,
    )));

    let body_vec: Vec<TokenTree> = brace_group.stream().into_iter().collect();
    emit_body_items(&body_vec, &el_ident, &mut block_tokens)?;

    block_tokens.push(TokenTree::Ident(el_ident));
    let mut block = Group::new(Delimiter::Brace, block_tokens.into_iter().collect());
    block.set_span(brace_group.span());
    Ok(TokenStream::from(TokenTree::Group(block)))
}

fn emit_body_items(
    tokens: &[TokenTree],
    el_ident: &Ident,
    out: &mut Vec<TokenTree>,
) -> Result<(), MacroError> {
    let mut i = 0;
    while i < tokens.len() {
        while i < tokens.len() && is_punct(&tokens[i], ',') {
            i += 1;
        }
        if i >= tokens.len() {
            break;
        }

        // 1. `ref: <expr>`
        if matches!(&tokens[i], TokenTree::Ident(id) if id.to_string() == "ref")
            && tokens.get(i + 1).is_some_and(|t| is_punct(t, ':'))
        {
            let span = tokens[i].span();
            i += 2;
            let expr = take_prop_expr(tokens, &mut i, span)?;
            emit_method_reassign(el_ident, Ident::new("node_ref", span), expr, span, out);
            continue;
        }

        // 2. Attached property: `Owner.Prop: <expr>`
        if let (
            Some(TokenTree::Ident(owner)),
            Some(TokenTree::Punct(dot)),
            Some(TokenTree::Ident(prop)),
            Some(TokenTree::Punct(colon)),
        ) = (
            tokens.get(i),
            tokens.get(i + 1),
            tokens.get(i + 2),
            tokens.get(i + 3),
        ) {
            if dot.as_char() == '.' && colon.as_char() == ':' && colon.spacing() == Spacing::Alone {
                let span = prop.span();
                let method = format!(
                    "{}_{}",
                    to_snake_case(&owner.to_string()),
                    to_snake_case(&prop.to_string())
                );
                i += 4;
                let expr = take_prop_expr(tokens, &mut i, span)?;
                emit_method_reassign(el_ident, Ident::new(&method, span), expr, span, out);
                continue;
            }
        }

        // 3. Named property: `prop: <expr>`
        if let (Some(TokenTree::Ident(prop)), Some(TokenTree::Punct(colon))) =
            (tokens.get(i), tokens.get(i + 1))
        {
            if colon.as_char() == ':' && colon.spacing() == Spacing::Alone {
                let prop_str = prop.to_string();
                if !matches!(prop_str.as_str(), "if" | "else" | "for" | "show") {
                    let span = prop.span();
                    let prop_ident = prop.clone();
                    i += 2;
                    let expr = take_prop_expr(tokens, &mut i, span)?;
                    emit_method_reassign(el_ident, prop_ident, expr, span, out);
                    continue;
                }
            }
        }

        // 4. Static `if <cond> { ... } [else ...]`
        if matches!(&tokens[i], TokenTree::Ident(id) if id.to_string() == "if") {
            emit_static_if(tokens, &mut i, el_ident, out)?;
            continue;
        }

        // 5. Reactive `show [when] <cond> { ... } [else { ... }]`
        if matches!(&tokens[i], TokenTree::Ident(id) if id.to_string() == "show") {
            emit_reactive_show(tokens, &mut i, el_ident, out)?;
            continue;
        }

        // 6. `for each <ident> in <items> key <key_fn> { <ChildElement> }` OR static `for <pat> in <iter> { ... }`
        if matches!(&tokens[i], TokenTree::Ident(id) if id.to_string() == "for") {
            if matches!(tokens.get(i + 1), Some(TokenTree::Ident(id)) if id.to_string() == "each") {
                emit_reactive_for_each(tokens, &mut i, el_ident, out)?;
            } else {
                emit_static_for(tokens, &mut i, el_ident, out)?;
            }
            continue;
        }

        // 7. Macro invocation child such as `fmt!("...")`
        if let (
            Some(TokenTree::Ident(mac_name)),
            Some(TokenTree::Punct(bang)),
            Some(TokenTree::Group(mac_group)),
        ) = (tokens.get(i), tokens.get(i + 1), tokens.get(i + 2))
        {
            if bang.as_char() == '!' {
                let span = mac_name.span();
                let mut expr = Vec::new();
                if mac_name.to_string() == "fmt" {
                    // Wrap `fmt!(...)` in `TextBlock::el().text(...)` when placed directly as a child!
                    let tb_prefix: TokenStream =
                        "::rustolonia::TextBlock::el().text".parse().unwrap();
                    expr.extend(tb_prefix);
                    let mut inner_mac = Vec::new();
                    inner_mac.push(TokenTree::Ident(mac_name.clone()));
                    inner_mac.push(TokenTree::Punct(bang.clone()));
                    inner_mac.push(TokenTree::Group(mac_group.clone()));
                    expr.push(TokenTree::Group(Group::new(
                        Delimiter::Parenthesis,
                        inner_mac.into_iter().collect(),
                    )));
                } else {
                    expr.push(TokenTree::Ident(mac_name.clone()));
                    expr.push(TokenTree::Punct(bang.clone()));
                    expr.push(TokenTree::Group(mac_group.clone()));
                }
                i += 3;
                emit_method_reassign(
                    el_ident,
                    Ident::new("rsx_child", span),
                    expr.into_iter().collect(),
                    span,
                    out,
                );
                continue;
            }
        }

        // 8. Child element: `ChildControl { ... }` or `ns::ChildControl { ... }`
        let saved = i;
        if parse_path(tokens, &mut i).is_some() {
            if matches!(tokens.get(i), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace)
            {
                i = saved;
                let span = tokens[i].span();
                let child_el = parse_element(tokens, &mut i)?;
                emit_method_reassign(el_ident, Ident::new("rsx_child", span), child_el, span, out);
                continue;
            }
            i = saved;
        }

        // 9. Struct-field shorthand property: `spacing,` (lowercase identifier followed by `,` or end)
        if let TokenTree::Ident(shorthand) = &tokens[i] {
            let next_is_sep = i + 1 >= tokens.len() || is_punct(&tokens[i + 1], ',');
            let first_char = shorthand.to_string().chars().next().unwrap_or('_');
            if next_is_sep && (first_char.is_lowercase() || first_char == '_') {
                let span = shorthand.span();
                let ident = shorthand.clone();
                i += 1;
                emit_method_reassign(
                    el_ident,
                    ident.clone(),
                    TokenStream::from(TokenTree::Ident(ident)),
                    span,
                    out,
                );
                continue;
            }
        }

        // 10. Literal child (`"Submit"`)
        if let TokenTree::Literal(lit) = &tokens[i] {
            let span = lit.span();
            let arg = TokenStream::from(TokenTree::Literal(lit.clone()));
            i += 1;
            emit_method_reassign(el_ident, Ident::new("rsx_child", span), arg, span, out);
            continue;
        }

        // 11. Braced expression child (`{ expr }`)
        if let TokenTree::Group(g) = &tokens[i] {
            if g.delimiter() == Delimiter::Brace {
                let span = g.span();
                let arg = g.stream();
                i += 1;
                emit_method_reassign(el_ident, Ident::new("rsx_child", span), arg, span, out);
                continue;
            }
        }

        return Err(MacroError::new(
            tokens[i].span(),
            format!("unexpected token `{}` inside rsx! element body", tokens[i]),
        ));
    }
    Ok(())
}

fn take_prop_expr(
    tokens: &[TokenTree],
    pos: &mut usize,
    prop_span: Span,
) -> Result<TokenStream, MacroError> {
    let start = *pos;
    while *pos < tokens.len() && !is_punct(&tokens[*pos], ',') {
        // Also stop if a new `Ident:` or `Ident {` starts on the next line without a comma?
        // Standard Rust / existing rsx! requires `,` after `prop: expr,` unless it's the very last item in `{ ... }`.
        *pos += 1;
    }
    if *pos == start {
        return Err(MacroError::new(
            prop_span,
            "expected property value expression after ':'",
        ));
    }
    let slice = &tokens[start..*pos];
    if *pos < tokens.len() && is_punct(&tokens[*pos], ',') {
        *pos += 1;
    }
    Ok(slice.iter().cloned().collect())
}

fn emit_method_reassign(
    el_ident: &Ident,
    method: Ident,
    arg: TokenStream,
    span: Span,
    out: &mut Vec<TokenTree>,
) {
    // `__el = __el.<method>(<arg>);`
    out.push(TokenTree::Ident(el_ident.clone()));
    out.push(TokenTree::Punct(with_span(
        Punct::new('=', Spacing::Alone),
        span,
    )));
    out.push(TokenTree::Ident(el_ident.clone()));
    out.push(TokenTree::Punct(with_span(
        Punct::new('.', Spacing::Alone),
        span,
    )));
    out.push(TokenTree::Ident(method));
    let mut parens = Group::new(Delimiter::Parenthesis, arg);
    parens.set_span(span);
    out.push(TokenTree::Group(parens));
    out.push(TokenTree::Punct(with_span(
        Punct::new(';', Spacing::Alone),
        span,
    )));
}

fn collect_until_body_brace(
    tokens: &[TokenTree],
    pos: &mut usize,
    kw_span: Span,
    kw_name: &str,
) -> Result<(Vec<TokenTree>, Group), MacroError> {
    let mut head = Vec::new();
    // Collect condition / header tokens until a `{ ... }` group that is the block body.
    // Note: in Rust `if let Some(x) = foo { ... }` or `if cond { ... }`, the body brace is
    // the brace after the header expression.
    while *pos < tokens.len() {
        if let TokenTree::Group(g) = &tokens[*pos] {
            if g.delimiter() == Delimiter::Brace && !head.is_empty() {
                let body = g.clone();
                *pos += 1;
                return Ok((head, body));
            }
        }
        head.push(tokens[*pos].clone());
        *pos += 1;
    }
    Err(MacroError::new(
        kw_span,
        format!("expected `{{ ... }}` block after `{kw_name}` condition in rsx!"),
    ))
}

fn emit_static_if(
    tokens: &[TokenTree],
    pos: &mut usize,
    el_ident: &Ident,
    out: &mut Vec<TokenTree>,
) -> Result<(), MacroError> {
    let if_tok = tokens[*pos].clone();
    let if_span = if_tok.span();
    *pos += 1;
    let (cond, then_group) = collect_until_body_brace(tokens, pos, if_span, "if")?;

    out.push(if_tok);
    out.extend(cond);
    let mut then_stmts = Vec::new();
    let then_vec: Vec<TokenTree> = then_group.stream().into_iter().collect();
    emit_body_items(&then_vec, el_ident, &mut then_stmts)?;
    let mut new_then = Group::new(Delimiter::Brace, then_stmts.into_iter().collect());
    new_then.set_span(then_group.span());
    out.push(TokenTree::Group(new_then));

    while *pos < tokens.len()
        && matches!(&tokens[*pos], TokenTree::Ident(id) if id.to_string() == "else")
    {
        let else_tok = tokens[*pos].clone();
        *pos += 1;
        out.push(else_tok);
        if *pos < tokens.len()
            && matches!(&tokens[*pos], TokenTree::Ident(id) if id.to_string() == "if")
        {
            emit_static_if(tokens, pos, el_ident, out)?;
            return Ok(());
        }
        let Some(TokenTree::Group(else_group)) = tokens.get(*pos) else {
            return Err(MacroError::new(
                if_span,
                "expected `{ ... }` or `if` after `else` in rsx!",
            ));
        };
        if else_group.delimiter() != Delimiter::Brace {
            return Err(MacroError::new(
                else_group.span(),
                "expected `{ ... }` after `else` in rsx!",
            ));
        }
        *pos += 1;
        let mut else_stmts = Vec::new();
        let else_vec: Vec<TokenTree> = else_group.stream().into_iter().collect();
        emit_body_items(&else_vec, el_ident, &mut else_stmts)?;
        let mut new_else = Group::new(Delimiter::Brace, else_stmts.into_iter().collect());
        new_else.set_span(else_group.span());
        out.push(TokenTree::Group(new_else));
        break;
    }
    Ok(())
}

fn emit_static_for(
    tokens: &[TokenTree],
    pos: &mut usize,
    el_ident: &Ident,
    out: &mut Vec<TokenTree>,
) -> Result<(), MacroError> {
    let for_tok = tokens[*pos].clone();
    let for_span = for_tok.span();
    *pos += 1;
    let (header, loop_group) = collect_until_body_brace(tokens, pos, for_span, "for")?;
    out.push(for_tok);
    out.extend(header);
    let mut loop_stmts = Vec::new();
    let loop_vec: Vec<TokenTree> = loop_group.stream().into_iter().collect();
    emit_body_items(&loop_vec, el_ident, &mut loop_stmts)?;
    let mut new_loop = Group::new(Delimiter::Brace, loop_stmts.into_iter().collect());
    new_loop.set_span(loop_group.span());
    out.push(TokenTree::Group(new_loop));
    Ok(())
}

fn parse_single_branch_view(group: &Group) -> Result<TokenStream, MacroError> {
    let inner: Vec<TokenTree> = group.stream().into_iter().collect();
    if inner.is_empty() {
        return Ok("::core::option::Option::None::<::rustolonia::AnyEl>"
            .parse()
            .unwrap());
    }
    if inner.len() == 1 {
        if let TokenTree::Literal(lit) = &inner[0] {
            return Ok(TokenStream::from(TokenTree::Literal(lit.clone())));
        }
        if let TokenTree::Group(g) = &inner[0] {
            if g.delimiter() == Delimiter::Brace {
                return Ok(g.stream());
            }
        }
    }
    let mut p = 0;
    let el = parse_element(&inner, &mut p)?;
    if p < inner.len() {
        return Err(MacroError::new(
            inner[p].span(),
            "show/for-each branch expects a single child element (wrap multiple children in StackPanel { ... })",
        ));
    }
    Ok(el)
}

fn emit_reactive_show(
    tokens: &[TokenTree],
    pos: &mut usize,
    el_ident: &Ident,
    out: &mut Vec<TokenTree>,
) -> Result<(), MacroError> {
    let show_span = tokens[*pos].span();
    *pos += 1;
    if matches!(tokens.get(*pos), Some(TokenTree::Ident(id)) if id.to_string() == "when") {
        *pos += 1;
    }
    let (cond, then_group) = collect_until_body_brace(tokens, pos, show_span, "show")?;
    let cond_src = cond.into_iter().collect::<TokenStream>().to_string();
    let then_src = parse_single_branch_view(&then_group)?.to_string();

    let show_expr: TokenStream = if *pos < tokens.len()
        && matches!(&tokens[*pos], TokenTree::Ident(id) if id.to_string() == "else")
    {
        *pos += 1;
        let Some(TokenTree::Group(else_group)) = tokens.get(*pos) else {
            return Err(MacroError::new(
                show_span,
                "expected `{ ... }` after `else` in `show` block",
            ));
        };
        *pos += 1;
        let else_src = parse_single_branch_view(else_group)?.to_string();
        format!("::rustolonia::Show::when_else({cond_src}, move || {then_src}, move || {else_src})")
            .parse()
            .unwrap()
    } else {
        format!("::rustolonia::Show::when({cond_src}, move || {then_src})")
            .parse()
            .unwrap()
    };

    emit_method_reassign(
        el_ident,
        Ident::new("rsx_child", show_span),
        show_expr,
        show_span,
        out,
    );
    Ok(())
}

/// `for each <item_ident> in <items_expr> key <key_closure> { <SingleElement> }`
fn emit_reactive_for_each(
    tokens: &[TokenTree],
    pos: &mut usize,
    el_ident: &Ident,
    out: &mut Vec<TokenTree>,
) -> Result<(), MacroError> {
    let for_span = tokens[*pos].span();
    *pos += 2; // skip `for each`
    let Some(TokenTree::Ident(item_ident)) = tokens.get(*pos) else {
        return Err(MacroError::new(
            for_span,
            "expected item binding identifier after `for each`",
        ));
    };
    let item_name = item_ident.to_string();
    *pos += 1;

    match tokens.get(*pos) {
        Some(TokenTree::Ident(id)) if id.to_string() == "in" => *pos += 1,
        _ => {
            return Err(MacroError::new(
                item_ident.span(),
                "expected `in` after `for each <item>`",
            ))
        }
    }

    let (rest_head, body_group) = collect_until_body_brace(tokens, pos, for_span, "for each")?;
    // Split `rest_head` at `key` keyword
    let key_idx = rest_head
        .iter()
        .position(|t| matches!(t, TokenTree::Ident(id) if id.to_string() == "key"));
    let Some(key_pos) = key_idx else {
        return Err(MacroError::new(
            for_span,
            "`for each` requires a `key` function, e.g. `for each item in items key |x| x.id { ... }`",
        ));
    };
    let items_src = rest_head[..key_pos]
        .iter()
        .cloned()
        .collect::<TokenStream>()
        .to_string();
    let key_src = rest_head[key_pos + 1..]
        .iter()
        .cloned()
        .collect::<TokenStream>()
        .to_string();
    if items_src.trim().is_empty() || key_src.trim().is_empty() {
        return Err(MacroError::new(
            for_span,
            "incomplete `for each <item> in <items> key <key_fn> { ... }` header",
        ));
    }
    let row_view = parse_single_branch_view(&body_group)?.to_string();
    let expr: TokenStream =
        format!("::rustolonia::For::each({items_src}, {key_src}, move |{item_name}| {row_view})")
            .parse()
            .unwrap();

    emit_method_reassign(
        el_ident,
        Ident::new("rsx_child", for_span),
        expr,
        for_span,
        out,
    );
    Ok(())
}
