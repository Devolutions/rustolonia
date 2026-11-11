use crate::tokens::{is_punct, parse_stream, to_snake_case, MacroError};
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};

struct ComponentParam {
    name: String,
    ty_src: String,
    fn_param_src: String,
    default_kind: DefaultKind,
    is_children_vec: bool,
}

enum DefaultKind {
    Required,
    DefaultTrait,
    OptionNone,
    Expr(String),
}

pub fn expand_component(attr: TokenStream, item: TokenStream) -> Result<TokenStream, MacroError> {
    if let Some(first) = attr.into_iter().next() {
        return Err(MacroError::new(
            first.span(),
            "#[component] does not accept attribute arguments",
        ));
    }

    let tokens: Vec<TokenTree> = item.into_iter().collect();
    let mut i = 0;

    // Collect outer attributes (`///` doc comments, `#[allow(...)]`, etc.)
    let mut attrs_end = 0;
    while i < tokens.len() && is_punct(&tokens[i], '#') {
        i += 1;
        if i < tokens.len() {
            i += 1;
        }
        attrs_end = i;
    }
    let outer_attrs: TokenStream = tokens[..attrs_end].iter().cloned().collect();

    // Parse visibility (`pub`, `pub(crate)`, etc.)
    let vis_start = i;
    if let Some(TokenTree::Ident(id)) = tokens.get(i) {
        if id.to_string() == "pub" {
            i += 1;
            if let Some(TokenTree::Group(g)) = tokens.get(i) {
                if g.delimiter() == Delimiter::Parenthesis {
                    i += 1;
                }
            }
        }
    }
    let vis_src = tokens[vis_start..i]
        .iter()
        .cloned()
        .collect::<TokenStream>()
        .to_string();

    // Expect `fn <ComponentName>`
    match tokens.get(i) {
        Some(TokenTree::Ident(id)) if id.to_string() == "fn" => i += 1,
        other => {
            return Err(MacroError::new(
                other.map_or_else(Span::call_site, |t| t.span()),
                "#[component] can only be applied to a `fn` item",
            ));
        }
    }

    let Some(TokenTree::Ident(fn_ident)) = tokens.get(i) else {
        return Err(MacroError::new(
            Span::call_site(),
            "#[component] expected component function name",
        ));
    };
    let comp_name = fn_ident.to_string();
    i += 1;

    // Reject generic type parameters for simplicity unless empty
    if i < tokens.len() && is_punct(&tokens[i], '<') {
        return Err(MacroError::new(
            tokens[i].span(),
            "#[component] functions must use concrete parameter types rather than generic type parameters",
        ));
    }

    let Some(TokenTree::Group(param_group)) = tokens.get(i) else {
        return Err(MacroError::new(
            fn_ident.span(),
            "#[component] expected parameter list `(...)`",
        ));
    };
    if param_group.delimiter() != Delimiter::Parenthesis {
        return Err(MacroError::new(
            param_group.span(),
            "#[component] expected parameter list `(...)`",
        ));
    }
    let params = parse_component_params(param_group.stream())?;
    i += 1;

    // Parse `-> ReturnType` up to `{ body }`
    let mut ret_tokens = Vec::new();
    if i < tokens.len() && is_punct(&tokens[i], '-') {
        i += 1;
        if i < tokens.len() && is_punct(&tokens[i], '>') {
            i += 1;
        }
        while i < tokens.len() {
            if let TokenTree::Group(g) = &tokens[i] {
                if g.delimiter() == Delimiter::Brace {
                    break;
                }
            }
            ret_tokens.push(tokens[i].clone());
            i += 1;
        }
    }
    if ret_tokens.is_empty() {
        return Err(MacroError::new(
            fn_ident.span(),
            "#[component] function must declare an explicit return type (e.g. `-> El<StackPanel>` or `-> AnyEl`)",
        ));
    }
    let ret_ty = ret_tokens.into_iter().collect::<TokenStream>().to_string();

    let Some(TokenTree::Group(body_group)) = tokens.get(i) else {
        return Err(MacroError::new(
            fn_ident.span(),
            "#[component] expected function body `{ ... }`",
        ));
    };
    let body_src = body_group.to_string();

    let impl_fn_name = format!("__rustolonia_component_{}", to_snake_case(&comp_name));
    let fn_params = params
        .iter()
        .map(|p| p.fn_param_src.clone())
        .collect::<Vec<_>>()
        .join(", ");

    let mut struct_fields = String::new();
    let mut el_inits = String::new();
    let mut setters = String::new();
    let mut call_args = String::new();

    for p in &params {
        if p.is_children_vec {
            struct_fields.push_str(&format!(
                "    {}: ::std::vec::Vec<::rustolonia::AnyEl>,\n",
                p.name
            ));
            el_inits.push_str(&format!(
                "            {}: ::std::vec::Vec::new(),\n",
                p.name
            ));
            setters.push_str(&format!(
                "    pub fn rsx_child(mut self, child: impl ::rustolonia::IntoChildEl) -> Self {{\n\
                 \x20       self.{name}.push(child.into_child_el());\n\
                 \x20       self\n\
                 \x20   }}\n\
                 \x20   pub fn child(self, child: impl ::rustolonia::IntoChildEl) -> Self {{\n\
                 \x20       self.rsx_child(child)\n\
                 \x20   }}\n",
                name = p.name
            ));
            call_args.push_str(&format!("self.{}, ", p.name));
            continue;
        }

        match &p.default_kind {
            DefaultKind::Required => {
                struct_fields.push_str(&format!(
                    "    {}: ::core::option::Option<{}>,\n",
                    p.name, p.ty_src
                ));
                el_inits.push_str(&format!(
                    "            {}: ::core::option::Option::None,\n",
                    p.name
                ));
                call_args.push_str(&format!(
                    "self.{name}.expect(\"missing required prop '{name}' on component '{comp_name}'\"), ",
                    name = p.name
                ));
            }
            DefaultKind::DefaultTrait => {
                struct_fields.push_str(&format!(
                    "    {}: ::core::option::Option<{}>,\n",
                    p.name, p.ty_src
                ));
                el_inits.push_str(&format!(
                    "            {}: ::core::option::Option::None,\n",
                    p.name
                ));
                call_args.push_str(&format!(
                    "self.{name}.unwrap_or_else(::core::default::Default::default), ",
                    name = p.name
                ));
            }
            DefaultKind::OptionNone => {
                struct_fields.push_str(&format!("    {}: {},\n", p.name, p.ty_src));
                el_inits.push_str(&format!(
                    "            {}: ::core::option::Option::None,\n",
                    p.name
                ));
                call_args.push_str(&format!("self.{}, ", p.name));
            }
            DefaultKind::Expr(expr) => {
                struct_fields.push_str(&format!(
                    "    {}: ::core::option::Option<{}>,\n",
                    p.name, p.ty_src
                ));
                el_inits.push_str(&format!(
                    "            {}: ::core::option::Option::None,\n",
                    p.name
                ));
                call_args.push_str(&format!(
                    "self.{name}.unwrap_or_else(|| {expr}), ",
                    name = p.name
                ));
            }
        }

        if matches!(p.default_kind, DefaultKind::OptionNone) {
            setters.push_str(&format!(
                "    pub fn {name}(mut self, value: impl ::rustolonia::IntoValue<{ty}>) -> Self {{\n\
                 \x20       self.{name} = value.into_value();\n\
                 \x20       self\n\
                 \x20   }}\n",
                name = p.name,
                ty = p.ty_src
            ));
        } else {
            setters.push_str(&format!(
                "    pub fn {name}(mut self, value: impl ::rustolonia::IntoValue<{ty}>) -> Self {{\n\
                 \x20       self.{name} = ::core::option::Option::Some(value.into_value());\n\
                 \x20       self\n\
                 \x20   }}\n",
                name = p.name,
                ty = p.ty_src
            ));
        }
    }

    let vis_prefix = if vis_src.is_empty() {
        String::new()
    } else {
        format!("{vis_src} ")
    };

    let attrs_src = outer_attrs.to_string();
    let code = format!(
        "#[allow(non_snake_case)]\n\
         fn {impl_fn_name}({fn_params}) -> {ret_ty} {body_src}\n\n\
         {attrs_src}\n\
         #[allow(non_camel_case_types)]\n\
         {vis_prefix}struct {comp_name} {{\n\
         {struct_fields}\
         }}\n\n\
         impl {comp_name} {{\n\
         \x20   pub fn el() -> Self {{\n\
         \x20       Self {{\n\
         {el_inits}\
         \x20       }}\n\
         \x20   }}\n\
         {setters}\
         \x20   pub fn into_el(self) -> {ret_ty} {{\n\
         \x20       {impl_fn_name}({call_args})\n\
         \x20   }}\n\
         \x20   pub fn build(self, scope: &::rustolonia::AppScope) -> ::rustolonia::Result<::rustolonia::Control> {{\n\
         \x20       ::rustolonia::AnyEl::from(self).build(scope)\n\
         \x20   }}\n\
         }}\n\n\
         impl ::core::convert::From<{comp_name}> for ::rustolonia::AnyEl {{\n\
         \x20   fn from(component: {comp_name}) -> Self {{\n\
         \x20       ::rustolonia::AnyEl::from_deferred(move |scope| {{\n\
         \x20           ::rustolonia::IntoChildEl::into_child_el(component.into_el()).build(scope)\n\
         \x20       }})\n\
         \x20   }}\n\
         }}\n"
    );

    Ok(parse_stream(&code))
}

fn parse_component_params(stream: TokenStream) -> Result<Vec<ComponentParam>, MacroError> {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut out = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let mut attr_default: Option<DefaultKind> = None;
        while i < tokens.len() && is_punct(&tokens[i], '#') {
            let hash_span = tokens[i].span();
            i += 1;
            let Some(TokenTree::Group(attr_group)) = tokens.get(i) else {
                return Err(MacroError::new(hash_span, "expected `[...]` after `#`"));
            };
            attr_default = Some(parse_prop_attr(attr_group.stream(), attr_group.span())?);
            i += 1;
        }
        if i >= tokens.len() {
            break;
        }
        let mut is_mut = false;
        if let TokenTree::Ident(id) = &tokens[i] {
            if id.to_string() == "mut" {
                is_mut = true;
                i += 1;
            }
        }
        let Some(TokenTree::Ident(param_ident)) = tokens.get(i) else {
            return Err(MacroError::new(
                tokens[i].span(),
                "expected parameter name in #[component]",
            ));
        };
        let name = param_ident.to_string();
        i += 1;
        if i >= tokens.len() || !is_punct(&tokens[i], ':') {
            return Err(MacroError::new(
                param_ident.span(),
                "expected ':' after parameter name",
            ));
        }
        i += 1;

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
        let ty_src = ty_tokens
            .iter()
            .cloned()
            .collect::<TokenStream>()
            .to_string();
        let fn_param_src = format!("{}{}: {}", if is_mut { "mut " } else { "" }, name, ty_src);
        let is_children_vec = name == "children";
        let is_option = is_option_type(&ty_tokens);

        let default_kind = match attr_default {
            Some(DefaultKind::OptionNone) if !is_option => {
                return Err(MacroError::new(
                    param_ident.span(),
                    "`#[prop(optional)]` requires an `Option<T>` component parameter",
                ));
            }
            Some(kind) => kind,
            None if is_option => DefaultKind::OptionNone,
            None => DefaultKind::Required,
        };

        out.push(ComponentParam {
            name,
            ty_src,
            fn_param_src,
            default_kind,
            is_children_vec,
        });
    }
    Ok(out)
}

fn is_option_type(ty_tokens: &[TokenTree]) -> bool {
    matches!(
        (ty_tokens.first(), ty_tokens.get(1)),
        (Some(TokenTree::Ident(id)), Some(TokenTree::Punct(p)))
            if id.to_string() == "Option" && p.as_char() == '<'
    )
}

fn parse_prop_attr(stream: TokenStream, span: Span) -> Result<DefaultKind, MacroError> {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let Some(TokenTree::Ident(head)) = tokens.first() else {
        return Err(MacroError::new(span, "expected `#[prop(...)]`"));
    };
    if head.to_string() != "prop" {
        return Err(MacroError::new(
            head.span(),
            "only `#[prop(default)]`, `#[prop(default = expr)]`, or `#[prop(optional)]` are supported on component parameters",
        ));
    }
    let Some(TokenTree::Group(inner)) = tokens.get(1) else {
        return Err(MacroError::new(
            head.span(),
            "expected parentheses `#[prop(...)]`",
        ));
    };
    let args: Vec<TokenTree> = inner.stream().into_iter().collect();
    match args.first() {
        Some(TokenTree::Ident(kw)) if kw.to_string() == "optional" => Ok(DefaultKind::OptionNone),
        Some(TokenTree::Ident(kw)) if kw.to_string() == "default" => {
            if args.len() == 1 {
                Ok(DefaultKind::DefaultTrait)
            } else if is_punct(&args[1], '=') {
                let expr: TokenStream = args[2..].iter().cloned().collect();
                if expr.is_empty() {
                    return Err(MacroError::new(
                        kw.span(),
                        "expected default expression after `=`",
                    ));
                }
                Ok(DefaultKind::Expr(expr.to_string()))
            } else {
                Err(MacroError::new(
                    args[1].span(),
                    "expected `#[prop(default)]` or `#[prop(default = expr)]`",
                ))
            }
        }

        _ => Err(MacroError::new(
            inner.span(),
            "expected `default`, `default = <expr>`, or `optional` inside `#[prop(...)]`",
        )),
    }
}
