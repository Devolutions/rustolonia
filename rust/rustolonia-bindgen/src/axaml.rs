//! Compile-time AXAML-to-Rust transpiler for standalone code-first applications.
//!
//! Parses `.axaml` XML markup and emits strongly-typed `rustolonia::El<T>`
//! builder code, a typed `NodeRef<T>` struct for `x:Name` elements, and
//! reactive `Signal<T>` bindings for `{Binding ...}` markup extensions.

use crate::error::GenerationError;
use crate::ir::{ProjectedProperty, ProjectedType, ProjectionIr};

#[derive(Debug, Clone)]
struct XmlAttribute {
    name: String,
    value: String,
}

#[derive(Debug, Clone)]
struct XmlNode {
    tag: String,
    attributes: Vec<XmlAttribute>,
    children: Vec<XmlNode>,
    text: String,
}

#[derive(Debug, Clone)]
struct NamedRef {
    field_name: String,
    control_type: String,
}

#[derive(Debug, Clone)]
struct BindingField {
    field_name: String,
    rust_type: String,
}

#[derive(Debug, Clone)]
struct HandlerField {
    field_name: String,
    arg_type: String,
}

#[derive(Debug, Clone)]
struct ParsedBinding {
    path: String,
    two_way: bool,
    string_format: Option<String>,
}

/// Compiles an `.axaml` document against the embedded `projection.ir.json`.
pub fn compile_axaml(axaml: &str) -> Result<String, GenerationError> {
    let ir: ProjectionIr = serde_json::from_str(include_str!("../../projection.ir.json"))?;
    compile_axaml_to_rust(&ir, axaml)
}

/// Compiles an `.axaml` document into a self-contained Rust module using `ir`
/// to validate control types, properties, attached properties, and enums.
pub fn compile_axaml_to_rust(ir: &ProjectionIr, axaml: &str) -> Result<String, GenerationError> {
    ir.validate()?;
    let root = parse_axaml_document(axaml)?;
    let root_type = resolve_control_type(ir, &root.tag)?;
    let root_safe = interface_suffix(&root_type.name);

    let view_stem = root
        .attributes
        .iter()
        .find(|attr| attr.name == "x:Class")
        .map(|attr| simple_name(&attr.value).to_string())
        .unwrap_or_else(|| root_safe.to_string());
    let fn_stem = to_snake(&view_stem);

    let mut refs = Vec::new();
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let expr = emit_node(ir, &root, &mut refs, &mut bindings, &mut handlers, 1)?;

    let mut out = String::from(
        "//! Generated from AXAML by rustolonia-bindgen. Do not edit.\n\
         #![allow(dead_code, unused_imports)]\n\n",
    );

    out.push_str(&format!(
        "#[derive(Clone, Copy, Debug, Default)]\n\
         pub struct {view_stem}Refs {{\n"
    ));
    for named in &refs {
        out.push_str(&format!(
            "    pub {}: rustolonia::NodeRef<rustolonia::{}>,\n",
            named.field_name, named.control_type
        ));
    }
    out.push_str("}\n\n");

    if !bindings.is_empty() {
        out.push_str(&format!(
            "#[derive(Clone, Copy, Debug)]\n\
             pub struct {view_stem}Bindings {{\n"
        ));
        for binding in &bindings {
            out.push_str(&format!(
                "    pub {}: rustolonia::Signal<{}>,\n",
                binding.field_name, binding.rust_type
            ));
        }
        out.push_str("}\n\n");
    }

    if !handlers.is_empty() {
        out.push_str(&format!("pub struct {view_stem}Handlers {{\n"));
        for handler in &handlers {
            out.push_str(&format!(
                "    pub {}: Box<dyn FnMut({}) + Send + 'static>,\n",
                handler.field_name, handler.arg_type
            ));
        }
        out.push_str("}\n\n");
    }

    let mut params = Vec::new();
    if !bindings.is_empty() {
        params.push(format!("bindings: &{view_stem}Bindings"));
    }
    if !handlers.is_empty() {
        params.push(format!("handlers: {view_stem}Handlers"));
    }
    let params_str = params.join(", ");

    out.push_str(&format!(
        "pub fn build_{fn_stem}({params_str}) -> (rustolonia::El<rustolonia::{root_safe}>, {view_stem}Refs) {{\n\
         \x20   let refs = {view_stem}Refs::default();\n\
         \x20   let root = {expr};\n\
         \x20   (root, refs)\n\
         }}\n"
    ));

    Ok(out)
}

fn emit_node(
    ir: &ProjectionIr,
    node: &XmlNode,
    refs: &mut Vec<NamedRef>,
    bindings: &mut Vec<BindingField>,
    handlers: &mut Vec<HandlerField>,
    indent: usize,
) -> Result<String, GenerationError> {
    let ty = resolve_control_type(ir, &node.tag)?;
    let safe_name = interface_suffix(&ty.name);
    let pad = "    ".repeat(indent + 1);
    let mut out = format!("rustolonia::{safe_name}::el()");

    let lineage = lineage(ir, ty);
    let reserved_methods: std::collections::HashSet<String> = lineage
        .iter()
        .flat_map(|owner| owner.methods.iter().map(|m| to_snake(&m.name)))
        .collect();
    let flags_enums: std::collections::HashSet<&str> = ir
        .enums
        .iter()
        .filter(|e| e.is_flags)
        .map(|e| e.full_name.as_str())
        .collect();

    for attr in &node.attributes {
        if attr.name == "xmlns"
            || attr.name.starts_with("xmlns:")
            || attr.name == "x:Class"
            || attr.name == "x:DataType"
            || attr.name == "x:CompileBindings"
        {
            continue;
        }

        if attr.name == "x:Name" || attr.name == "Name" {
            let field_name = to_snake(&attr.value);
            if !refs.iter().any(|r| r.field_name == field_name) {
                refs.push(NamedRef {
                    field_name: field_name.clone(),
                    control_type: safe_name.to_string(),
                });
            }
            out.push_str(&format!("\n{pad}.node_ref(refs.{field_name})"));
            if attr.name == "Name" {
                out.push_str(&format!("\n{pad}.name({:?})", attr.value));
            }
            continue;
        }

        if attr.name == "Classes" {
            out.push_str(&format!("\n{pad}.class({:?})", attr.value));
            continue;
        }

        if let Some((owner_name, prop_name)) = attr.name.split_once('.') {
            let attached = ir
                .attached_properties
                .iter()
                .find(|p| p.owner_name == owner_name && p.name == prop_name)
                .ok_or_else(|| {
                    GenerationError::Invalid(format!(
                        "Unknown attached property '{}.{}' on <{}>",
                        owner_name, prop_name, node.tag
                    ))
                })?;
            let method = format!("{}_{}", to_snake(owner_name), to_snake(prop_name));
            let val_expr = format_attached_value(ir, attached, &attr.value, bindings)?;
            out.push_str(&format!("\n{pad}.{method}({val_expr})"));
            continue;
        }

        if let Some(event) = lineage
            .iter()
            .flat_map(|owner| owner.events.iter())
            .find(|e| e.name == attr.name)
        {
            let event_snake = to_snake(&event.name);
            let field_name = to_snake(&attr.value);
            let arg_type = if event.payload_kind == "Args" {
                let args_name = simple_name(
                    event
                        .args_interface_name
                        .as_deref()
                        .expect("argsInterfaceName"),
                );
                format!("&mut rustolonia_sys::ComPtr<rustolonia::{args_name}>")
            } else if !event.parameters.is_empty() {
                let handler = simple_name(&event.handler_interface_name);
                let stem = handler
                    .strip_prefix("IAvn")
                    .unwrap_or(handler)
                    .strip_suffix("Handler")
                    .unwrap_or(handler);
                format!("&mut rustolonia::{stem}EventArgs")
            } else {
                "()".to_string()
            };
            if !handlers.iter().any(|h| h.field_name == field_name) {
                handlers.push(HandlerField {
                    field_name: field_name.clone(),
                    arg_type,
                });
            }
            out.push_str(&format!("\n{pad}.on_{event_snake}(handlers.{field_name})"));
            continue;
        }

        let prop = lineage
            .iter()
            .flat_map(|owner| owner.properties.iter())
            .find(|p| p.name == attr.name && p.can_write)
            .ok_or_else(|| {
                GenerationError::Invalid(format!(
                    "Unknown or read-only property '{}' on <{}>",
                    attr.name, node.tag
                ))
            })?;

        let snake = to_snake(&prop.name);
        let builder = if prop.kind == "Bool" || prop.kind == "NullableBool" {
            match snake.strip_prefix("is_") {
                Some(stripped) if !reserved_methods.contains(stripped) => stripped,
                _ => snake.as_str(),
            }
        } else {
            snake.as_str()
        };

        let val_expr = format_property_value(ir, prop, &flags_enums, &attr.value, bindings)?;
        out.push_str(&format!("\n{pad}.{builder}({val_expr})"));
    }

    for child in &node.children {
        if let Some((owner_tag, prop_name)) = child.tag.split_once('.') {
            if owner_tag != node.tag {
                return Err(GenerationError::Invalid(format!(
                    "Property element <{}> does not match parent <{}>",
                    child.tag, node.tag
                )));
            }
            let prop_snake = to_snake(prop_name);
            if let Some(first_grandchild) = child.children.first() {
                if first_grandchild.tag == "DataTemplate" {
                    let template_child = first_grandchild.children.first().ok_or_else(|| {
                        GenerationError::Invalid(format!(
                            "<DataTemplate> inside <{}> must contain a root control element",
                            child.tag
                        ))
                    })?;
                    let mut tpl_refs = Vec::new();
                    let mut tpl_handlers = Vec::new();
                    let tpl_expr = emit_node(
                        ir,
                        template_child,
                        &mut tpl_refs,
                        bindings,
                        &mut tpl_handlers,
                        indent + 2,
                    )?;
                    out.push_str(&format!(
                        "\n{pad}.with_op(move |scope, control| {{\n\
                         {pad}    let tpl = rustolonia::data_template_reactive(scope, |_| true, move |_item| {tpl_expr})?;\n\
                         {pad}    control.set_{prop_snake}(Some(&tpl))\n\
                         {pad}}})"
                    ));
                } else {
                    let child_expr =
                        emit_node(ir, first_grandchild, refs, bindings, handlers, indent + 1)?;
                    out.push_str(&format!("\n{pad}.{prop_snake}({child_expr})"));
                }
            } else if !child.text.trim().is_empty() {
                out.push_str(&format!("\n{pad}.{prop_snake}({:?})", child.text.trim()));
            }
            continue;
        }

        let child_expr = emit_node(ir, child, refs, bindings, handlers, indent + 1)?;
        out.push_str(&format!("\n{pad}.rsx_child({child_expr})"));
    }

    if !node.text.trim().is_empty() {
        let trimmed = node.text.trim();
        if let Some(parsed) = parse_binding_extension(trimmed) {
            let expr = format_binding_expr(&parsed, "String", bindings);
            out.push_str(&format!("\n{pad}.rsx_child({expr})"));
        } else {
            out.push_str(&format!("\n{pad}.rsx_child({:?})", trimmed));
        }
    }

    Ok(out)
}

fn format_attached_value(
    ir: &ProjectionIr,
    property: &crate::ir::ProjectedAttachedProperty,
    raw: &str,
    bindings: &mut Vec<BindingField>,
) -> Result<String, GenerationError> {
    let enum_name = simple_name(&property.managed_type_name);
    let is_enum = ir
        .enums
        .iter()
        .any(|e| e.full_name == property.managed_type_name);
    let rust_type = if property.kind == "StringUtf16" {
        "String".to_string()
    } else if is_enum {
        format!("rustolonia::{enum_name}")
    } else {
        rust_scalar_kind(&property.kind).to_string()
    };

    if let Some(parsed) = parse_binding_extension(raw) {
        return Ok(format_binding_expr(&parsed, &rust_type, bindings));
    }

    if property.kind == "StringUtf16" {
        return Ok(format!("{raw:?}"));
    }
    if is_enum {
        return format_enum_literal(ir, &property.managed_type_name, raw);
    }
    format_scalar_literal(&property.kind, raw)
}

fn format_property_value(
    ir: &ProjectionIr,
    property: &ProjectedProperty,
    flags_enums: &std::collections::HashSet<&str>,
    raw: &str,
    bindings: &mut Vec<BindingField>,
) -> Result<String, GenerationError> {
    let rust_type = property_rust_type(property, flags_enums);
    if let Some(parsed) = parse_binding_extension(raw) {
        return Ok(format_binding_expr(&parsed, &rust_type, bindings));
    }

    match property.kind.as_str() {
        "StringUtf16" => Ok(format!("{raw:?}")),
        "ComInterface"
            if property.interface_name.as_deref().map(simple_name) == Some("IAvnControl") =>
        {
            Ok(format!("{raw:?}"))
        }
        "Thickness" => format_thickness_literal(raw),
        "CornerRadius" => format_corner_radius_literal(raw),
        "Size" => format_pair_literal("Size", raw),
        "Point" => format_pair_literal("Point", raw),
        "Vector" => format_pair_literal("Vector", raw),
        "Brush" => format_brush_literal(raw),
        "I32" if is_enum_property(property, flags_enums) => {
            format_enum_literal(ir, property.managed_type_name.as_deref().unwrap(), raw)
        }
        kind => format_scalar_literal(kind, raw),
    }
}

fn property_rust_type(
    property: &ProjectedProperty,
    flags_enums: &std::collections::HashSet<&str>,
) -> String {
    match property.kind.as_str() {
        "StringUtf16" => "String".to_string(),
        "Brush" => "Option<rustolonia::Brush>".to_string(),
        "Thickness" => "rustolonia::Thickness".to_string(),
        "CornerRadius" => "rustolonia::CornerRadius".to_string(),
        "Size" => "rustolonia::Size".to_string(),
        "Point" => "rustolonia::Point".to_string(),
        "Vector" => "rustolonia::Vector".to_string(),
        "Rect" => "rustolonia::Rect".to_string(),
        "Color" => "rustolonia::Color".to_string(),
        "NullableBool" => "bool".to_string(),
        "I32" if is_enum_property(property, flags_enums) => {
            format!(
                "rustolonia::{}",
                simple_name(property.managed_type_name.as_deref().unwrap())
            )
        }
        kind => rust_scalar_kind(kind).to_string(),
    }
}

fn register_binding(bindings: &mut Vec<BindingField>, field_name: &str, rust_type: &str) {
    if !bindings.iter().any(|b| b.field_name == field_name) {
        bindings.push(BindingField {
            field_name: field_name.to_string(),
            rust_type: rust_type.to_string(),
        });
    }
}

fn format_binding_expr(
    parsed: &ParsedBinding,
    rust_type: &str,
    bindings: &mut Vec<BindingField>,
) -> String {
    let field_name = to_snake(&parsed.path);
    register_binding(bindings, &field_name, rust_type);
    if let Some(fmt_raw) = &parsed.string_format {
        let unescaped = fmt_raw.strip_prefix("{}").unwrap_or(fmt_raw);
        let rust_fmt = unescaped.replace("{0}", "{}");
        format!(
            "{{ let __sig = bindings.{field_name}; move || format!({rust_fmt:?}, __sig.get()) }}"
        )
    } else if parsed.two_way {
        format!("bindings.{field_name}.two_way()")
    } else {
        format!("bindings.{field_name}")
    }
}

fn parse_binding_extension(raw: &str) -> Option<ParsedBinding> {
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix("{Binding")
        .or_else(|| trimmed.strip_prefix("{CompiledBinding"))?
        .strip_suffix('}')?
        .trim();
    if inner.is_empty() {
        return None;
    }
    let mut path: Option<String> = None;
    let mut two_way = false;
    let mut string_format: Option<String> = None;
    for part in split_binding_clauses(inner) {
        let part = part.trim();
        if let Some(p) = part.strip_prefix("Path=") {
            path = Some(p.trim().to_string());
        } else if let Some(mode) = part.strip_prefix("Mode=") {
            two_way = mode.trim().eq_ignore_ascii_case("TwoWay");
        } else if let Some(fmt) = part.strip_prefix("StringFormat=") {
            let fmt = fmt.trim().trim_matches('\'').trim_matches('"').to_string();
            string_format = Some(fmt);
        } else if path.is_none() && !part.contains('=') {
            path = Some(part.to_string());
        }
    }
    path.map(|path| ParsedBinding {
        path,
        two_way,
        string_format,
    })
}

fn split_binding_clauses(input: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;
    for (idx, ch) in input.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ',' if !in_single && !in_double => {
                result.push(&input[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }
    result.push(&input[start..]);
    result
}

fn format_thickness_literal(raw: &str) -> Result<String, GenerationError> {
    let nums = parse_f64_list(raw)?;
    match nums.as_slice() {
        [all] => Ok(format!("rustolonia::Thickness::uniform({all:?}_f64)")),
        [h, v] => Ok(format!(
            "rustolonia::Thickness::symmetric({h:?}_f64, {v:?}_f64)"
        )),
        [l, t, r, b] => Ok(format!(
            "rustolonia::Thickness::new({l:?}_f64, {t:?}_f64, {r:?}_f64, {b:?}_f64)"
        )),
        _ => Err(GenerationError::Invalid(format!(
            "Invalid Thickness literal '{raw}'; expected 1, 2, or 4 numbers"
        ))),
    }
}

fn format_corner_radius_literal(raw: &str) -> Result<String, GenerationError> {
    let nums = parse_f64_list(raw)?;
    match nums.as_slice() {
        [all] => Ok(format!("rustolonia::CornerRadius::uniform({all:?}_f64)")),
        [tl, tr, br, bl] => Ok(format!(
            "rustolonia::CornerRadius::new({tl:?}_f64, {tr:?}_f64, {br:?}_f64, {bl:?}_f64)"
        )),
        _ => Err(GenerationError::Invalid(format!(
            "Invalid CornerRadius literal '{raw}'; expected 1 or 4 numbers"
        ))),
    }
}

fn format_pair_literal(type_name: &str, raw: &str) -> Result<String, GenerationError> {
    let nums = parse_f64_list(raw)?;
    match nums.as_slice() {
        [a, b] => Ok(format!(
            "rustolonia::{type_name}::new({a:?}_f64, {b:?}_f64)"
        )),
        _ => Err(GenerationError::Invalid(format!(
            "Invalid {type_name} literal '{raw}'; expected 2 numbers"
        ))),
    }
}

fn format_brush_literal(raw: &str) -> Result<String, GenerationError> {
    let trimmed = raw.trim();
    let (a, r, g, b) = if let Some(rgb) = parse_hex_color(trimmed) {
        rgb
    } else if let Some(named) = parse_named_color(trimmed) {
        named
    } else {
        return Err(GenerationError::Invalid(format!(
            "Unsupported Brush literal '{raw}'; use #RGB, #ARGB, #RRGGBB, #AARRGGBB, or a standard color name"
        )));
    };
    Ok(format!(
        "rustolonia::Brush::solid(rustolonia::Color::new({a}, {r}, {g}, {b}))"
    ))
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8, u8)> {
    let hex = hex.strip_prefix('#')?;
    if !hex.is_ascii() {
        return None;
    }
    match hex.len() {
        3 => Some((
            255,
            u8::from_str_radix(&hex[0..1], 16).ok()? * 17,
            u8::from_str_radix(&hex[1..2], 16).ok()? * 17,
            u8::from_str_radix(&hex[2..3], 16).ok()? * 17,
        )),
        4 => Some((
            u8::from_str_radix(&hex[0..1], 16).ok()? * 17,
            u8::from_str_radix(&hex[1..2], 16).ok()? * 17,
            u8::from_str_radix(&hex[2..3], 16).ok()? * 17,
            u8::from_str_radix(&hex[3..4], 16).ok()? * 17,
        )),
        6 => Some((
            255,
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        )),
        8 => Some((
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            u8::from_str_radix(&hex[6..8], 16).ok()?,
        )),
        _ => None,
    }
}

fn parse_named_color(name: &str) -> Option<(u8, u8, u8, u8)> {
    match name.to_ascii_lowercase().as_str() {
        "transparent" => Some((0, 0, 0, 0)),
        "black" => Some((255, 0, 0, 0)),
        "white" => Some((255, 255, 255, 255)),
        "red" => Some((255, 255, 0, 0)),
        "green" => Some((255, 0, 128, 0)),
        "blue" => Some((255, 0, 0, 255)),
        "yellow" => Some((255, 255, 255, 0)),
        "cyan" => Some((255, 0, 255, 255)),
        "magenta" => Some((255, 255, 0, 255)),
        "gray" | "grey" => Some((255, 128, 128, 128)),
        "orange" => Some((255, 255, 165, 0)),
        "purple" => Some((255, 128, 0, 128)),
        _ => None,
    }
}

fn format_enum_literal(
    ir: &ProjectionIr,
    managed_type_name: &str,
    raw: &str,
) -> Result<String, GenerationError> {
    let enum_def = ir
        .enums
        .iter()
        .find(|e| e.full_name == managed_type_name)
        .ok_or_else(|| GenerationError::Invalid(format!("Unknown enum '{managed_type_name}'")))?;
    let trimmed = raw.trim();
    if !enum_def.values.iter().any(|v| v.name == trimmed) {
        return Err(GenerationError::Invalid(format!(
            "Invalid variant '{trimmed}' for enum '{}'",
            enum_def.name
        )));
    }
    Ok(format!("rustolonia::{}::{}", enum_def.name, trimmed))
}

fn format_scalar_literal(kind: &str, raw: &str) -> Result<String, GenerationError> {
    let trimmed = raw.trim();
    match kind {
        "F64" => {
            let val: f64 = trimmed
                .parse()
                .map_err(|_| GenerationError::Invalid(format!("Invalid f64 literal '{raw}'")))?;
            Ok(format!("{val:?}_f64"))
        }
        "F32" => {
            let val: f32 = trimmed
                .parse()
                .map_err(|_| GenerationError::Invalid(format!("Invalid f32 literal '{raw}'")))?;
            Ok(format!("{val:?}_f32"))
        }
        "I32" => {
            let val: i32 = trimmed
                .parse()
                .map_err(|_| GenerationError::Invalid(format!("Invalid i32 literal '{raw}'")))?;
            Ok(format!("{val}_i32"))
        }
        "I64" => {
            let val: i64 = trimmed
                .parse()
                .map_err(|_| GenerationError::Invalid(format!("Invalid i64 literal '{raw}'")))?;
            Ok(format!("{val}_i64"))
        }
        "Bool" | "NullableBool" => {
            let val: bool = trimmed
                .to_ascii_lowercase()
                .parse()
                .map_err(|_| GenerationError::Invalid(format!("Invalid bool literal '{raw}'")))?;
            Ok(format!("{val}"))
        }
        _ => Err(GenerationError::Invalid(format!(
            "Unsupported AXAML scalar kind '{kind}' for literal '{raw}'"
        ))),
    }
}

fn parse_f64_list(raw: &str) -> Result<Vec<f64>, GenerationError> {
    raw.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(|part| {
            part.parse::<f64>()
                .map_err(|_| GenerationError::Invalid(format!("Invalid numeric literal '{raw}'")))
        })
        .collect()
}

fn resolve_control_type<'a>(
    ir: &'a ProjectionIr,
    tag: &str,
) -> Result<&'a ProjectedType, GenerationError> {
    let expected = format!("IAvn{tag}");
    ir.types
        .iter()
        .find(|t| t.kind == "Class" && t.is_constructible && t.name == expected)
        .ok_or_else(|| {
            GenerationError::Invalid(format!(
                "Unknown or non-constructible AXAML control <{tag}>"
            ))
        })
}

fn lineage<'a>(ir: &'a ProjectionIr, ty: &'a ProjectedType) -> Vec<&'a ProjectedType> {
    let mut result = vec![ty];
    let mut current = ty;
    while let Some(base) = &current.base_full_name {
        current = ir
            .types
            .iter()
            .find(|candidate| &candidate.full_name == base)
            .expect("base type must be present in projection IR");
        result.push(current);
    }
    result.reverse();
    result
}

fn is_enum_property(
    property: &ProjectedProperty,
    flags_enums: &std::collections::HashSet<&str>,
) -> bool {
    property.kind == "I32"
        && property.managed_type_name.as_deref() != Some("System.Int32")
        && !property
            .managed_type_name
            .as_deref()
            .is_some_and(|name| flags_enums.contains(name))
}

fn rust_scalar_kind(kind: &str) -> &str {
    match kind {
        "CharUtf16" => "u16",
        "I32" => "i32",
        "I64" => "i64",
        "TimeSpanI64" => "i64",
        "F32" => "f32",
        "F64" => "f64",
        "Bool" => "bool",
        "NullableBool" => "Option<bool>",
        _ => "()",
    }
}

fn interface_suffix(name: &str) -> &str {
    name.strip_prefix("IAvn").unwrap_or(name)
}

fn simple_name(full_name: &str) -> &str {
    full_name.rsplit('.').next().unwrap_or(full_name)
}

fn to_snake(name: &str) -> String {
    let mut result = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

fn parse_axaml_document(input: &str) -> Result<XmlNode, GenerationError> {
    let cleaned = strip_xml_comments_and_pi(input);
    let mut pos = 0;
    skip_ws(&cleaned, &mut pos);
    parse_element(&cleaned, &mut pos)
}

fn strip_xml_comments_and_pi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<!--") {
            if let Some(end) = after.find("-->") {
                rest = &after[end + 3..];
                continue;
            }
        }
        if let Some(after) = rest.strip_prefix("<?") {
            if let Some(end) = after.find("?>") {
                rest = &after[end + 2..];
                continue;
            }
        }
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

fn skip_ws(s: &str, pos: &mut usize) {
    while *pos < s.len() && s[*pos..].starts_with(char::is_whitespace) {
        *pos += s[*pos..].chars().next().unwrap().len_utf8();
    }
}

fn parse_element(s: &str, pos: &mut usize) -> Result<XmlNode, GenerationError> {
    skip_ws(s, pos);
    if !s[*pos..].starts_with('<') {
        return Err(GenerationError::Invalid(
            "Expected '<' starting AXAML element".to_string(),
        ));
    }
    *pos += 1;
    let tag = parse_name(s, pos)?;
    let mut attributes = Vec::new();

    loop {
        skip_ws(s, pos);
        if *pos >= s.len() {
            return Err(GenerationError::Invalid(format!(
                "Unexpected EOF in opening tag <{tag}>"
            )));
        }
        if s[*pos..].starts_with("/>") {
            *pos += 2;
            return Ok(XmlNode {
                tag,
                attributes,
                children: Vec::new(),
                text: String::new(),
            });
        }
        if s[*pos..].starts_with('>') {
            *pos += 1;
            break;
        }
        let attr_name = parse_name(s, pos)?;
        skip_ws(s, pos);
        if !s[*pos..].starts_with('=') {
            return Err(GenerationError::Invalid(format!(
                "Expected '=' after attribute '{attr_name}' in <{tag}>"
            )));
        }
        *pos += 1;
        skip_ws(s, pos);
        let attr_value = parse_quoted_string(s, pos)?;
        attributes.push(XmlAttribute {
            name: attr_name,
            value: unescape_xml(&attr_value),
        });
    }

    let mut children = Vec::new();
    let mut text = String::new();

    loop {
        if *pos >= s.len() {
            return Err(GenerationError::Invalid(format!(
                "Unexpected EOF waiting for </{tag}>"
            )));
        }
        if s[*pos..].starts_with("</") {
            *pos += 2;
            let close_tag = parse_name(s, pos)?;
            skip_ws(s, pos);
            if !s[*pos..].starts_with('>') {
                return Err(GenerationError::Invalid(format!(
                    "Expected '>' closing </{close_tag}>"
                )));
            }
            *pos += 1;
            if close_tag != tag {
                return Err(GenerationError::Invalid(format!(
                    "Mismatched closing tag </{close_tag}> for <{tag}>"
                )));
            }
            break;
        }
        if s[*pos..].starts_with('<') {
            children.push(parse_element(s, pos)?);
        } else {
            let next_lt = s[*pos..].find('<').unwrap_or(s.len() - *pos);
            text.push_str(&unescape_xml(&s[*pos..*pos + next_lt]));
            *pos += next_lt;
        }
    }

    Ok(XmlNode {
        tag,
        attributes,
        children,
        text,
    })
}

fn parse_name(s: &str, pos: &mut usize) -> Result<String, GenerationError> {
    let start = *pos;
    while *pos < s.len() {
        let ch = s[*pos..].chars().next().unwrap();
        if ch.is_alphanumeric() || ch == '_' || ch == ':' || ch == '.' || ch == '-' {
            *pos += ch.len_utf8();
        } else {
            break;
        }
    }
    if *pos == start {
        return Err(GenerationError::Invalid(
            "Expected XML identifier in AXAML".to_string(),
        ));
    }
    Ok(s[start..*pos].to_string())
}

fn parse_quoted_string(s: &str, pos: &mut usize) -> Result<String, GenerationError> {
    let quote = s[*pos..]
        .chars()
        .next()
        .ok_or_else(|| GenerationError::Invalid("Expected quoted attribute value".to_string()))?;
    if quote != '"' && quote != '\'' {
        return Err(GenerationError::Invalid(
            "Expected '\"' or '\\'' starting attribute value".to_string(),
        ));
    }
    *pos += quote.len_utf8();
    let start = *pos;
    let end_offset = s[*pos..].find(quote).ok_or_else(|| {
        GenerationError::Invalid("Unterminated quoted attribute value".to_string())
    })?;
    let val = s[start..start + end_offset].to_string();
    *pos = start + end_offset + quote.len_utf8();
    Ok(val)
}

fn unescape_xml(raw: &str) -> String {
    raw.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_axaml_window_with_refs_attached_props_and_two_way_bindings() {
        let axaml = r##"
<Window xmlns="https://github.com/avaloniaui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        x:Class="MyApp.Views.LoginWindow"
        Title="Sign In"
        Width="420"
        Height="280">
  <Border Padding="24,16" CornerRadius="8" Background="#1E1E2E">
    <StackPanel Orientation="Vertical" Spacing="12">
      <TextBlock x:Name="HeaderText" Classes="heading primary" FontSize="20">Welcome back</TextBlock>
      <TextBox x:Name="UsernameInput"
               Grid.Row="0"
               Grid.Column="1"
               DockPanel.Dock="Top"
               ToolTip.Tip="Enter your username"
               PlaceholderText="Username"
               Text="{Binding Username, Mode=TwoWay}" />
      <CheckBox IsChecked="{Binding RememberMe, Mode=TwoWay}">Remember me</CheckBox>
      <Button x:Name="SubmitButton">Sign In</Button>
      <TextBlock Text="{Binding StatusMessage}" />
    </StackPanel>
  </Border>
</Window>
"##;

        let rust_code = compile_axaml(axaml).expect("AXAML should compile to Rust");
        assert!(rust_code.contains("pub struct LoginWindowRefs"));
        assert!(rust_code.contains("pub header_text: rustolonia::NodeRef<rustolonia::TextBlock>"));
        assert!(rust_code.contains("pub username_input: rustolonia::NodeRef<rustolonia::TextBox>"));
        assert!(rust_code.contains("pub submit_button: rustolonia::NodeRef<rustolonia::Button>"));
        assert!(rust_code.contains("pub struct LoginWindowBindings"));
        assert!(rust_code.contains("pub username: rustolonia::Signal<String>"));
        assert!(rust_code.contains("pub remember_me: rustolonia::Signal<bool>"));
        assert!(rust_code.contains("pub status_message: rustolonia::Signal<String>"));
        assert!(rust_code.contains("pub fn build_login_window(bindings: &LoginWindowBindings)"));
        assert!(
            rust_code.contains(".padding(rustolonia::Thickness::symmetric(24.0_f64, 16.0_f64))")
        );
        assert!(rust_code.contains(".corner_radius(rustolonia::CornerRadius::uniform(8.0_f64))"));
        assert!(rust_code.contains(".orientation(rustolonia::Orientation::Vertical)"));
        assert!(rust_code.contains(".grid_row(0_i32)"));
        assert!(rust_code.contains(".grid_column(1_i32)"));
        assert!(rust_code.contains(".dock_panel_dock(rustolonia::Dock::Top)"));
        assert!(rust_code.contains(".tool_tip_tip(\"Enter your username\")"));
        assert!(rust_code.contains(".text(bindings.username.two_way())"));
        assert!(rust_code.contains(".checked(bindings.remember_me.two_way())"));
        assert!(rust_code.contains(".text(bindings.status_message)"));
    }

    #[test]
    fn compiles_axaml_events_string_format_and_data_template() {
        let axaml = r##"
<Window xmlns="https://github.com/avaloniaui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        x:Class="MyApp.Views.TaskWindow"
        Title="Tasks">
  <StackPanel Spacing="8">
    <TextBlock Text="{Binding Count, StringFormat='{}{0} items'}" />
    <Button Click="OnAddTask">Add</Button>
    <ListBox>
      <ListBox.ItemTemplate>
        <DataTemplate>
          <TextBlock FontSize="14">Row item</TextBlock>
        </DataTemplate>
      </ListBox.ItemTemplate>
    </ListBox>
  </StackPanel>
</Window>
"##;

        let rust_code =
            compile_axaml(axaml).expect("AXAML with events and DataTemplate should compile");
        assert!(rust_code.contains("pub struct TaskWindowHandlers"));
        assert!(rust_code.contains("pub on_add_task: Box<dyn FnMut(()) + Send + 'static>"));
        assert!(rust_code.contains(".on_click(handlers.on_add_task)"));
        assert!(rust_code.contains("format!(\"{} items\", __sig.get())"));
        assert!(rust_code.contains("rustolonia::data_template_reactive"));
        assert!(rust_code.contains("control.set_item_template(Some(&tpl))"));
    }

    #[test]
    fn rejects_unknown_control_or_property_in_axaml() {
        assert!(compile_axaml("<NonExistentControl />").is_err());
        assert!(compile_axaml("<Button NoSuchProperty=\"123\" />").is_err());
        assert!(compile_axaml("<Button Grid.NoSuchAttached=\"1\" />").is_err());
    }
}
