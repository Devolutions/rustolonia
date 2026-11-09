//! Compile-time AXAML-to-Rust transpiler for standalone code-first applications.
//!
//! Parses `.axaml` XML markup and emits strongly-typed `rustolonia::El<T>`
//! builder code, a typed `NodeRef<T>` struct for `x:Name` elements, and
//! reactive `Signal<T>` bindings for `{Binding ...}` markup extensions.

use crate::error::GenerationError;
use crate::ir::{ProjectedProperty, ProjectedType, ProjectionIr};
use std::collections::HashSet;

#[derive(Debug, Clone)]
struct XmlAttribute {
    name: String,
    value: String,
    namespace: Option<String>,
    type_arguments: Option<Vec<XmlQualifiedName>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct XmlQualifiedName {
    namespace: String,
    name: String,
}

#[derive(Debug, Clone)]
struct XmlNode {
    tag: String,
    namespace: Option<String>,
    line: u32,
    attributes: Vec<XmlAttribute>,
    children: Vec<XmlNode>,
    text: String,
}

const AVALONIA_XML_NAMESPACE: &str = "https://github.com/avaloniaui";
const XAML_XML_NAMESPACE: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
const MARKUP_COMPATIBILITY_XML_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/markup-compatibility/2006";

/// An application-supplied default for an otherwise ambiguous binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxamlBindingMode {
    OneWay,
    TwoWay,
}

/// A root element explicitly mapped to an existing projected control.
///
/// This substitutes the Rust control; it does not reproduce the original
/// application's base class or code-behind.
#[derive(Debug, Clone)]
pub struct AxamlRootAlias {
    pub namespace_uri: String,
    pub name: String,
    pub projected_control: String,
}

/// A QName used in `x:TypeArguments` and its application-owned Rust type.
#[derive(Debug, Clone)]
pub struct AxamlTypeMapping {
    pub namespace_uri: String,
    pub name: String,
    pub rust_type: String,
}

/// The Rust model type accepted by a custom component's binding property.
#[derive(Debug, Clone)]
pub enum AxamlComponentValueType {
    /// An explicitly named Rust type.
    RustType(String),
    /// A resolved generic argument of the registered component.
    TypeArgument(usize),
    /// A vector whose element is a resolved generic argument.
    VecOfTypeArgument(usize),
}

/// A binding-only property implemented by an application-owned builder.
///
/// Each method receives a `rustolonia::Signal` of the declared value type.
/// The application implements the declared one-way or two-way behavior.
#[derive(Debug, Clone)]
pub struct AxamlComponentProperty {
    pub name: String,
    pub value_type: AxamlComponentValueType,
    pub default_mode: AxamlBindingMode,
    pub one_way_method: Option<String>,
    pub two_way_method: Option<String>,
}

/// An explicit mapping from an XML element to a Rust-owned component.
///
/// `factory` is a Rust function path. It returns an `El<projected_control>`
/// directly, or a component builder converted to that `El` by `finish_method`.
/// `after_children`, when present, is a Rust function applied to the completed
/// `El` after projected properties and children have been added.
#[derive(Debug, Clone)]
pub struct AxamlComponent {
    pub namespace_uri: String,
    pub name: String,
    pub projected_control: String,
    pub factory: String,
    pub type_argument_count: usize,
    pub finish_method: Option<String>,
    pub after_children: Option<String>,
    pub properties: Vec<AxamlComponentProperty>,
}

/// Default binding mode for a property on an exact projected control type.
#[derive(Debug, Clone)]
pub struct AxamlBindingDefault {
    pub projected_control: String,
    pub property: String,
    pub mode: AxamlBindingMode,
}

/// Application-owned mappings used when compiling an AXAML document.
#[derive(Debug, Clone, Default)]
pub struct AxamlCompileOptions {
    pub root_aliases: Vec<AxamlRootAlias>,
    pub components: Vec<AxamlComponent>,
    pub type_mappings: Vec<AxamlTypeMapping>,
    pub binding_defaults: Vec<AxamlBindingDefault>,
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
    source: String,
    is_signal: bool,
    is_copy: bool,
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
    mode_explicit: bool,
    string_format: Option<String>,
    converter: Option<String>,
    converter_parameter: Option<String>,
    converter_culture: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedResource {
    key: String,
    extension: &'static str,
}

#[derive(Debug, Clone)]
struct ItemContext {
    item_type: String,
    key_field: String,
    signal_name: String,
}

#[derive(Clone, Copy)]
struct EmitPosition<'a> {
    indent: usize,
    item_context: Option<&'a ItemContext>,
    is_root: bool,
}

#[derive(Debug)]
struct ItemTemplateRefs {
    type_name: String,
    refs: Vec<NamedRef>,
}

struct GenerationState {
    view_stem: String,
    item_templates: Vec<ItemTemplateRefs>,
    options: AxamlCompileOptions,
}

/// Compiles an `.axaml` document against a `projection.ir.json` string.
pub fn compile_axaml_from_json(ir_json: &str, axaml: &str) -> Result<String, GenerationError> {
    compile_axaml_from_json_with_options(ir_json, axaml, &AxamlCompileOptions::default())
}

/// Compiles an `.axaml` document with application-owned component mappings.
pub fn compile_axaml_from_json_with_options(
    ir_json: &str,
    axaml: &str,
    options: &AxamlCompileOptions,
) -> Result<String, GenerationError> {
    let ir: ProjectionIr = serde_json::from_str(ir_json)?;
    compile_axaml_to_rust_with_options(&ir, axaml, options)
}

/// Compiles an `.axaml` document into a self-contained Rust module using `ir`
/// to validate control types, properties, attached properties, and enums.
pub fn compile_axaml_to_rust(ir: &ProjectionIr, axaml: &str) -> Result<String, GenerationError> {
    compile_axaml_to_rust_with_options(ir, axaml, &AxamlCompileOptions::default())
}

/// Compiles an `.axaml` document with application-owned component mappings.
pub fn compile_axaml_to_rust_with_options(
    ir: &ProjectionIr,
    axaml: &str,
    options: &AxamlCompileOptions,
) -> Result<String, GenerationError> {
    ir.validate()?;
    validate_options(ir, options)?;
    let root = parse_axaml_document(axaml, options)?;
    let root_type = resolve_node_type(ir, &root, options, true)
        .map_err(|error| GenerationError::Invalid(format!("{error} (line {})", root.line)))?;
    let root_safe = interface_suffix(&root_type.name);

    let view_stem = root
        .attributes
        .iter()
        .find(|attr| attr.name == "x:Class")
        .map(|attr| simple_name(&attr.value).to_string())
        .unwrap_or_else(|| root_safe.to_string());
    if !is_rust_identifier(&view_stem) {
        return Err(GenerationError::Invalid(format!(
            "x:Class '{}' does not produce a valid Rust type name (line {})",
            view_stem, root.line
        )));
    }
    let fn_stem = to_snake(&view_stem);

    let mut refs = Vec::new();
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut state = GenerationState {
        view_stem: view_stem.clone(),
        item_templates: Vec::new(),
        options: options.clone(),
    };
    let expr = emit_node(
        ir,
        &root,
        &mut refs,
        &mut bindings,
        &mut handlers,
        &mut state,
        EmitPosition {
            indent: 1,
            item_context: None,
            is_root: true,
        },
    )?;

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

    for template in &state.item_templates {
        out.push_str(&format!(
            "#[derive(Clone, Copy, Debug)]\n\
             pub struct {} {{\n",
            template.type_name
        ));
        for named in &template.refs {
            out.push_str(&format!(
                "    pub {}: rustolonia::NodeRef<rustolonia::{}>,\n",
                named.field_name, named.control_type
            ));
        }
        out.push_str("}\n\n");
        out.push_str(&format!(
            "impl {} {{\n\
             \x20   fn new_scoped() -> Self {{\n\
             \x20       Self {{\n",
            template.type_name
        ));
        for named in &template.refs {
            out.push_str(&format!(
                "            {}: rustolonia::NodeRef::new_scoped(),\n",
                named.field_name
            ));
        }
        out.push_str("        }\n    }\n}\n\n");
    }

    if !bindings.is_empty() {
        let derives = if bindings.iter().all(|binding| binding.is_copy) {
            "Clone, Copy, Debug"
        } else {
            "Clone, Debug"
        };
        out.push_str(&format!(
            "#[derive({derives})]\n\
             pub struct {view_stem}Bindings {{\n"
        ));
        for binding in &bindings {
            if binding.is_signal {
                out.push_str(&format!(
                    "    pub {}: rustolonia::Signal<{}>,\n",
                    binding.field_name, binding.rust_type
                ));
            } else {
                out.push_str(&format!(
                    "    pub {}: {},\n",
                    binding.field_name, binding.rust_type
                ));
            }
        }
        out.push_str("}\n\n");
    }

    if !handlers.is_empty() {
        for (index, handler) in handlers.iter().enumerate() {
            out.push_str(&format!(
                "pub type {view_stem}Handler{} = Box<dyn FnMut({}) + Send + 'static>;\n",
                index + 1,
                handler.arg_type
            ));
        }
        out.push('\n');
        out.push_str(&format!("pub struct {view_stem}Handlers {{\n"));
        for (index, handler) in handlers.iter().enumerate() {
            out.push_str(&format!(
                "    pub {}: {view_stem}Handler{},\n",
                handler.field_name,
                index + 1
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
    state: &mut GenerationState,
    position: EmitPosition<'_>,
) -> Result<String, GenerationError> {
    emit_node_inner(ir, node, refs, bindings, handlers, state, position).map_err(
        |error| match error {
            GenerationError::Invalid(message) if message.contains("(line ") => {
                GenerationError::Invalid(message)
            }
            other => GenerationError::Invalid(format!("{other} (line {})", node.line)),
        },
    )
}

fn emit_node_inner(
    ir: &ProjectionIr,
    node: &XmlNode,
    refs: &mut Vec<NamedRef>,
    bindings: &mut Vec<BindingField>,
    handlers: &mut Vec<HandlerField>,
    state: &mut GenerationState,
    position: EmitPosition<'_>,
) -> Result<String, GenerationError> {
    let EmitPosition {
        indent,
        item_context,
        is_root,
    } = position;
    let component = state
        .options
        .components
        .iter()
        .find(|component| {
            node.namespace.as_deref() == Some(component.namespace_uri.as_str())
                && node.tag == component.name
        })
        .cloned();
    let ty = resolve_node_type(ir, node, &state.options, is_root)?;
    let safe_name = interface_suffix(&ty.name);
    let pad = "    ".repeat(indent + 1);
    let mut out = if let Some(component) = &component {
        emit_component_builder(component, node, &state.options, bindings, item_context)?
    } else {
        format!("rustolonia::{safe_name}::el()")
    };

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
        if component.as_ref().is_some_and(|component| {
            attr.namespace.is_none()
                && component
                    .properties
                    .iter()
                    .any(|property| property.name == attr.name)
        }) {
            continue;
        }
        if attr.name == "x:TypeArguments" && component.is_some() {
            continue;
        }
        if attr.name == "x:Class" || attr.name == "x:DataType" || attr.name == "x:CompileBindings" {
            if !is_root {
                return Err(GenerationError::Invalid(format!(
                    "AXAML attribute '{}' is supported only on the root element",
                    attr.name
                )));
            }
            continue;
        }

        if attr.name == "x:Name" || (attr.name == "Name" && attr.namespace.is_none()) {
            let field_name = to_snake(&attr.value);
            if !is_rust_identifier(&field_name) {
                return Err(GenerationError::Invalid(format!(
                    "Name '{}' does not produce a valid Rust field name",
                    attr.value
                )));
            }
            if let Some(existing) = refs.iter().find(|r| r.field_name == field_name) {
                return Err(GenerationError::Invalid(format!(
                    "Duplicate AXAML name '{}' maps to Rust field '{}'",
                    attr.value, existing.field_name
                )));
            }
            refs.push(NamedRef {
                field_name: field_name.clone(),
                control_type: safe_name.to_string(),
            });
            let refs_variable = if item_context.is_some() {
                "__item_refs"
            } else {
                "refs"
            };
            out.push_str(&format!("\n{pad}.node_ref({refs_variable}.{field_name})"));
            out.push_str(&format!("\n{pad}.name({:?})", attr.value));
            continue;
        }

        if attr.name == "Classes" && attr.namespace.is_none() {
            reject_markup_in_nonbinding_value(&attr.value)?;
            out.push_str(&format!("\n{pad}.class({:?})", attr.value));
            continue;
        }

        if attr.namespace.as_deref() == Some(XAML_XML_NAMESPACE) {
            return Err(GenerationError::Invalid(format!(
                "Unsupported x: attribute '{}'",
                attr.name
            )));
        }
        if attr.namespace.is_some() {
            return Err(GenerationError::Invalid(format!(
                "Unsupported namespaced attribute '{}'",
                attr.name
            )));
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
            if item_context.is_some() {
                return Err(GenerationError::Invalid(format!(
                    "DataTemplate item event '{}' is not supported",
                    attr.name
                )));
            }
            if attr.value.trim().starts_with('{') {
                return Err(GenerationError::Invalid(format!(
                    "Markup extensions are not supported for event handler '{}'",
                    attr.name
                )));
            }
            let event_snake = to_snake(&event.name);
            let field_name = to_snake(&attr.value);
            if !is_rust_identifier(&field_name) {
                return Err(GenerationError::Invalid(format!(
                    "Event handler name '{}' does not produce a valid Rust field name",
                    attr.value
                )));
            }
            let arg_type = if event.payload_kind == "Args" {
                let args_name = simple_name(
                    event
                        .args_interface_name
                        .as_deref()
                        .expect("argsInterfaceName"),
                );
                format!("&mut rustolonia::ComPtr<rustolonia::{args_name}>")
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
            if handlers.iter().any(|h| h.field_name == field_name) {
                return Err(GenerationError::Invalid(format!(
                    "Event handler '{}' is used more than once or conflicts on Rust field '{}'",
                    attr.value, field_name
                )));
            }
            handlers.push(HandlerField {
                field_name: field_name.clone(),
                arg_type,
            });
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

        let builder = property_builder_name(prop, &reserved_methods);

        if prop.kind == "Command"
            && prop.interface_name.as_deref().map(simple_name) == Some("IAvnCommand")
        {
            if item_context.is_some() {
                return Err(GenerationError::Invalid(
                    "DataTemplate item command bindings are not supported".to_string(),
                ));
            }
            let binding = parse_binding_extension(&attr.value)?.ok_or_else(|| {
                GenerationError::Invalid(format!(
                    "Command property '{}' requires a one-way binding",
                    prop.name
                ))
            })?;
            if binding.two_way || binding.string_format.is_some() || binding.converter.is_some() {
                return Err(GenerationError::Invalid(
                    "Command properties support only a one-way binding".to_string(),
                ));
            }
            let field_name = binding_field_name(&binding.path)?;
            let (source, read) = if binding.path.contains('.') {
                register_value_binding(
                    bindings,
                    &field_name,
                    "rustolonia::PropertyPath<Option<rustolonia::Command>>",
                    &binding.path,
                    false,
                )?;
                (format!("bindings.{field_name}.clone()"), "__command.get()?")
            } else {
                register_binding(
                    bindings,
                    &field_name,
                    "Option<rustolonia::Command>",
                    &binding.path,
                )?;
                (format!("bindings.{field_name}"), "__command.get()")
            };
            out.push_str(&format!(
                "\n{pad}.with_op({{ let __command = {source}; move |_, control| {{ let __control = control.clone(); rustolonia::Effect::new(move || {{ let command = {read}; __control.set_{builder}(command.as_ref().map(|command| command.as_com_ptr())) }})?; Ok(()) }} }})"
            ));
            continue;
        }

        if prop.name == "CommandParameter" && prop.kind == "Variant" {
            if item_context.is_some() {
                return Err(GenerationError::Invalid(
                    "DataTemplate item command parameters are not supported".to_string(),
                ));
            }
            let value = if let Some(binding) = parse_binding_extension(&attr.value)? {
                if binding.two_way || binding.string_format.is_some() || binding.converter.is_some()
                {
                    return Err(GenerationError::Invalid(
                        "CommandParameter supports only a one-way scalar binding".to_string(),
                    ));
                }
                let field_name =
                    format!("{}_command_parameter", binding_field_name(&binding.path)?);
                let source = format!("CommandParameter:{}", binding.path);
                if binding.path.contains('.') {
                    register_value_binding(
                        bindings,
                        &field_name,
                        "rustolonia::PropertyPath<rustolonia::Variant>",
                        &source,
                        false,
                    )?;
                    format!("bindings.{field_name}.clone()")
                } else {
                    register_binding(bindings, &field_name, "rustolonia::Variant", &source)?;
                    format!("bindings.{field_name}")
                }
            } else {
                let literal = attr.value.strip_prefix("{}").unwrap_or(&attr.value);
                format!("{literal:?}")
            };
            out.push_str(&format!("\n{pad}.{builder}({value})"));
            continue;
        }

        if attr.name == "ItemsSource" && prop.kind == "ComCollection" {
            let source_binding = parse_binding_extension(&attr.value)?.ok_or_else(|| {
                GenerationError::Invalid(
                    "ItemsSource requires a {Binding Items} expression".to_string(),
                )
            })?;
            if source_binding.two_way
                || source_binding.string_format.is_some()
                || source_binding.converter.is_some()
            {
                return Err(GenerationError::Invalid(
                    "ItemsSource supports only a one-way binding without a converter or StringFormat"
                        .to_string(),
                ));
            }
            if !lineage.iter().any(|owner| owner.name == "IAvnItemsControl") {
                return Err(GenerationError::Invalid(
                    "ItemsSource lowering requires an ItemsControl".to_string(),
                ));
            }
            let has_template = node.children.iter().any(|child| {
                child.tag.split_once('.').is_some_and(|(owner, property)| {
                    owner == node.tag && property == "ItemTemplate"
                })
            });
            if has_template {
                let (item_context, item_view, item_refs) =
                    compile_item_template(ir, node, state, indent + 1)?;
                let items = format_binding_expr(
                    &source_binding,
                    &format!("Vec<{}>", item_context.item_type),
                    bindings,
                )?;
                let render = if item_refs.is_empty() {
                    format!(
                        "move |{}: rustolonia::Signal<{}>| {item_view}",
                        item_context.signal_name, item_context.item_type
                    )
                } else {
                    let index = state.item_templates.len() + 1;
                    let type_name = format!("{}ItemTemplate{index}Refs", state.view_stem);
                    let field_name = format!(
                        "{}_item_refs_{index}",
                        binding_field_name(&source_binding.path)?
                    );
                    let rust_type = format!(
                        "rustolonia::ItemNameScope<{}, {type_name}>",
                        item_context.item_type
                    );
                    register_value_binding(
                        bindings,
                        &field_name,
                        &rust_type,
                        &format!("named item template {index} for {}", source_binding.path),
                        false,
                    )?;
                    state.item_templates.push(ItemTemplateRefs {
                        type_name: type_name.clone(),
                        refs: item_refs,
                    });
                    format!(
                        "{{ let __names = bindings.{field_name}.clone(); move |{}: rustolonia::Signal<{}>| {{ let __item_refs = {type_name}::new_scoped(); __names.register({}, __item_refs); {item_view} }} }}",
                        item_context.signal_name, item_context.item_type, item_context.signal_name
                    )
                };
                out.push_str(&format!(
                    "\n{pad}.items_for({items}, |__item: &{}| std::clone::Clone::clone(&__item.{}), {render})",
                    item_context.item_type, item_context.key_field
                ));
            } else {
                let field_name = binding_field_name(&source_binding.path)?;
                register_value_binding(
                    bindings,
                    &field_name,
                    "rustolonia::ItemsView",
                    &source_binding.path,
                    false,
                )?;
                out.push_str(&format!(
                    "\n{pad}.with_op({{ let __items = bindings.{field_name}.clone(); move |scope, control| __items.attach(scope, control) }})"
                ));
            }
            continue;
        }

        let binding_default = state.options.binding_defaults.iter().find(|default| {
            default.projected_control == safe_name && default.property == prop.name
        });
        let default_mode = binding_default.map(|default| default.mode);
        let val_expr = if let Some(context) = item_context {
            if let Some(value) = format_item_property_value(
                prop,
                &lineage,
                &attr.value,
                context,
                &flags_enums,
                default_mode,
            )? {
                value
            } else {
                format_property_value(
                    ir,
                    prop,
                    &lineage,
                    &flags_enums,
                    &attr.value,
                    bindings,
                    default_mode,
                )?
            }
        } else {
            format_property_value(
                ir,
                prop,
                &lineage,
                &flags_enums,
                &attr.value,
                bindings,
                default_mode,
            )?
        };
        out.push_str(&format!("\n{pad}.{builder}({val_expr})"));
    }

    let has_items_source = node
        .attributes
        .iter()
        .any(|attribute| attribute.name == "ItemsSource");
    if has_items_source
        && (!node.text.trim().is_empty()
            || node.children.iter().any(|child| !child.tag.contains('.')))
    {
        return Err(GenerationError::Invalid(
            "ItemsSource cannot be combined with inline items".to_string(),
        ));
    }

    if !node.text.trim().is_empty() && !node.children.is_empty() {
        return Err(GenerationError::Invalid(format!(
            "Mixed text and control children in <{}> are not supported",
            node.tag
        )));
    }

    for child in &node.children {
        if has_items_source
            && child
                .tag
                .split_once('.')
                .is_some_and(|(owner, property)| owner == node.tag && property == "ItemTemplate")
        {
            continue;
        }
        if let Some((owner_tag, prop_name)) = child.tag.split_once('.') {
            if child.namespace != node.namespace {
                return Err(GenerationError::Invalid(format!(
                    "Property element <{}> must use its parent element's namespace",
                    child.tag
                )));
            }
            if !child.attributes.is_empty() {
                return Err(GenerationError::Invalid(format!(
                    "Attributes on property element <{}> are not supported",
                    child.tag
                )));
            }
            if !child.text.trim().is_empty() && !child.children.is_empty() {
                return Err(GenerationError::Invalid(format!(
                    "Mixed text and elements in property element <{}> are not supported",
                    child.tag
                )));
            }
            if owner_tag != node.tag {
                return Err(GenerationError::Invalid(format!(
                    "Property element <{}> does not match parent <{}>",
                    child.tag, node.tag
                )));
            }
            if lineage.iter().any(|owner| owner.name == "IAvnGrid")
                && matches!(prop_name, "ColumnDefinitions" | "RowDefinitions")
            {
                if node
                    .attributes
                    .iter()
                    .any(|attribute| attribute.name == prop_name)
                    || node
                        .children
                        .iter()
                        .filter(|other| {
                            other.tag == child.tag && other.namespace == child.namespace
                        })
                        .count()
                        != 1
                {
                    return Err(GenerationError::Invalid(format!(
                        "Grid property '{}' is specified more than once",
                        prop_name
                    )));
                }
                let definitions = format_grid_definitions(child, prop_name)?;
                out.push_str(&format!("\n{pad}.{}({definitions:?})", to_snake(prop_name)));
                continue;
            }
            if child.children.len() > 1 {
                return Err(GenerationError::Invalid(format!(
                    "Property element <{}> must contain at most one value element",
                    child.tag
                )));
            }
            let property = lineage
                .iter()
                .flat_map(|owner| owner.properties.iter())
                .find(|property| property.name == prop_name && property.can_write)
                .ok_or_else(|| {
                    GenerationError::Invalid(format!(
                        "Unknown or read-only property element '{}.{}'",
                        node.tag, prop_name
                    ))
                })?;
            let prop_builder = property_builder_name(property, &reserved_methods);
            if let Some(first_grandchild) = child.children.first() {
                if first_grandchild.tag == "DataTemplate" {
                    if first_grandchild.namespace.as_deref() != Some(AVALONIA_XML_NAMESPACE)
                        || !first_grandchild.attributes.is_empty()
                    {
                        return Err(GenerationError::Invalid(
                            "Static DataTemplate requires the Avalonia namespace and no attributes"
                                .to_string(),
                        ));
                    }
                    if property.kind != "DataTemplate" {
                        return Err(GenerationError::Invalid(format!(
                            "Property element <{}> does not accept a DataTemplate",
                            child.tag
                        )));
                    }
                    if first_grandchild.children.len() != 1
                        || !first_grandchild.text.trim().is_empty()
                    {
                        return Err(GenerationError::Invalid(format!(
                            "<DataTemplate> inside <{}> must contain exactly one root control",
                            child.tag
                        )));
                    }
                    let template_child = first_grandchild.children.first().ok_or_else(|| {
                        GenerationError::Invalid(format!(
                            "<DataTemplate> inside <{}> must contain a root control element",
                            child.tag
                        ))
                    })?;
                    let mut tpl_refs = Vec::new();
                    let mut tpl_bindings = Vec::new();
                    let mut tpl_handlers = Vec::new();
                    let tpl_expr = emit_node(
                        ir,
                        template_child,
                        &mut tpl_refs,
                        &mut tpl_bindings,
                        &mut tpl_handlers,
                        state,
                        EmitPosition {
                            indent: indent + 2,
                            item_context: None,
                            is_root: false,
                        },
                    )?;
                    if !tpl_refs.is_empty() || !tpl_bindings.is_empty() || !tpl_handlers.is_empty()
                    {
                        return Err(GenerationError::Invalid(
                            "DataTemplate names, bindings, and events require ItemsSource with an explicit Rust item contract".to_string(),
                        ));
                    }
                    out.push_str(&format!(
                        "\n{pad}.with_op(move |scope, control| {{\n\
                         {pad}    let tpl = rustolonia::data_template_reactive(scope, |_| true, move |_item| {tpl_expr})?;\n\
                         {pad}    control.set_{prop_builder}(Some(&tpl))\n\
                         {pad}}})"
                    ));
                } else {
                    if property.kind != "ComInterface"
                        || property.interface_name.as_deref().map(simple_name)
                            != Some("IAvnControl")
                    {
                        return Err(GenerationError::Invalid(format!(
                            "Property element <{}> requires a projected control property",
                            child.tag
                        )));
                    }
                    let child_expr = emit_node(
                        ir,
                        first_grandchild,
                        refs,
                        bindings,
                        handlers,
                        state,
                        EmitPosition {
                            indent: indent + 1,
                            item_context,
                            is_root: false,
                        },
                    )?;
                    out.push_str(&format!("\n{pad}.{prop_builder}({child_expr})"));
                }
            } else if !child.text.trim().is_empty() {
                let value = format_property_value(
                    ir,
                    property,
                    &lineage,
                    &flags_enums,
                    child.text.trim(),
                    bindings,
                    state
                        .options
                        .binding_defaults
                        .iter()
                        .find(|default| {
                            default.projected_control == safe_name
                                && default.property == property.name
                        })
                        .map(|default| default.mode),
                )?;
                out.push_str(&format!("\n{pad}.{prop_builder}({value})"));
            }
            continue;
        }

        let child_expr = emit_node(
            ir,
            child,
            refs,
            bindings,
            handlers,
            state,
            EmitPosition {
                indent: indent + 1,
                item_context,
                is_root: false,
            },
        )?;
        out.push_str(&format!("\n{pad}.rsx_child({child_expr})"));
    }

    if !node.text.trim().is_empty() {
        let trimmed = node.text.trim();
        if let Some(parsed) = parse_binding_extension(trimmed)? {
            if parsed.converter.is_some() {
                return Err(GenerationError::Invalid(
                    "Converter bindings in element text are not supported; use a property attribute"
                        .to_string(),
                ));
            }
            let expr = if let Some(context) = item_context {
                format_item_binding_expression(&parsed, context)?
            } else {
                format_binding_expr(&parsed, "String", bindings)?
            };
            out.push_str(&format!("\n{pad}.rsx_child({expr})"));
        } else {
            let literal = trimmed.strip_prefix("{}").unwrap_or(trimmed);
            out.push_str(&format!("\n{pad}.rsx_child({:?})", literal));
        }
    }

    if let Some(after_children) = component.and_then(|component| component.after_children) {
        out = format!("{after_children}({out})");
    }
    Ok(out)
}

fn emit_component_builder(
    component: &AxamlComponent,
    node: &XmlNode,
    options: &AxamlCompileOptions,
    bindings: &mut Vec<BindingField>,
    item_context: Option<&ItemContext>,
) -> Result<String, GenerationError> {
    let arguments = node
        .attributes
        .iter()
        .find(|attribute| attribute.name == "x:TypeArguments")
        .and_then(|attribute| attribute.type_arguments.as_ref())
        .map_or(&[][..], Vec::as_slice);
    if arguments.len() != component.type_argument_count {
        return Err(GenerationError::Invalid(format!(
            "<{}> requires {} x:TypeArguments, found {}",
            component.name,
            component.type_argument_count,
            arguments.len()
        )));
    }
    let rust_arguments = arguments
        .iter()
        .map(|argument| {
            options
                .type_mappings
                .iter()
                .find(|mapping| {
                    mapping.namespace_uri == argument.namespace && mapping.name == argument.name
                })
                .map(|mapping| mapping.rust_type.as_str())
                .ok_or_else(|| {
                    GenerationError::Invalid(format!(
                        "Unregistered x:TypeArguments type '{{{}}}{}'",
                        argument.namespace, argument.name
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let generic = if rust_arguments.is_empty() {
        String::new()
    } else {
        format!("::<{}>", rust_arguments.join(", "))
    };
    let mut out = format!("{}{generic}()", component.factory);
    for attribute in &node.attributes {
        let Some(property) = component
            .properties
            .iter()
            .find(|property| attribute.namespace.is_none() && property.name == attribute.name)
        else {
            continue;
        };
        if item_context.is_some() {
            return Err(GenerationError::Invalid(
                "Custom component bindings in DataTemplate items are not supported".to_string(),
            ));
        }
        let binding = parse_binding_extension(&attribute.value)?.ok_or_else(|| {
            GenerationError::Invalid(format!(
                "Custom property '{}.{}' requires a binding",
                component.name, property.name
            ))
        })?;
        if binding.path.contains('.')
            || binding.string_format.is_some()
            || binding.converter.is_some()
        {
            return Err(GenerationError::Invalid(format!(
                "Custom property '{}.{}' supports only a direct signal binding without a converter or StringFormat",
                component.name, property.name
            )));
        }
        let mode = if binding.mode_explicit {
            if binding.two_way {
                AxamlBindingMode::TwoWay
            } else {
                AxamlBindingMode::OneWay
            }
        } else {
            property.default_mode
        };
        let method = match mode {
            AxamlBindingMode::OneWay => &property.one_way_method,
            AxamlBindingMode::TwoWay => &property.two_way_method,
        }
        .as_ref()
        .ok_or_else(|| {
            GenerationError::Invalid(format!(
                "{mode:?} binding is not supported for custom property '{}.{}'",
                component.name, property.name
            ))
        })?;
        let rust_type = match &property.value_type {
            AxamlComponentValueType::RustType(path) => path.clone(),
            AxamlComponentValueType::TypeArgument(index) => rust_arguments[*index].to_string(),
            AxamlComponentValueType::VecOfTypeArgument(index) => {
                format!("Vec<{}>", rust_arguments[*index])
            }
        };
        let field = binding_field_name(&binding.path)?;
        register_binding(bindings, &field, &rust_type, &binding.path)?;
        out.push_str(&format!(".{method}(bindings.{field})"));
    }
    if let Some(method) = &component.finish_method {
        out.push_str(&format!(".{method}()"));
    }
    Ok(out)
}

fn format_grid_definitions(element: &XmlNode, property: &str) -> Result<String, GenerationError> {
    if !element.text.trim().is_empty() {
        return Err(GenerationError::Invalid(format!(
            "Property element <{}> requires definition elements",
            element.tag
        )));
    }
    let (definition, length_property) = if property == "ColumnDefinitions" {
        ("ColumnDefinition", "Width")
    } else {
        ("RowDefinition", "Height")
    };
    let mut tracks = Vec::new();
    for child in &element.children {
        if child.namespace.as_deref() != Some(AVALONIA_XML_NAMESPACE)
            || child.tag != definition
            || !child.children.is_empty()
            || !child.text.trim().is_empty()
        {
            return Err(GenerationError::Invalid(format!(
                "<{}> accepts only empty <{definition}> elements",
                element.tag
            )));
        }
        let mut length = "*";
        let mut group = None;
        for attribute in &child.attributes {
            if attribute.namespace.is_some() {
                return Err(GenerationError::Invalid(format!(
                    "Unsupported attribute '{}' on <{definition}>",
                    attribute.name
                )));
            }
            match attribute.name.as_str() {
                name if name == length_property => length = attribute.value.trim(),
                "SharedSizeGroup" => group = Some(attribute.value.trim()),
                _ => {
                    return Err(GenerationError::Invalid(format!(
                        "Unsupported attribute '{}' on <{definition}>",
                        attribute.name
                    )))
                }
            }
        }
        let number = length.strip_suffix('*').unwrap_or(length);
        if !(length.eq_ignore_ascii_case("Auto")
            || length.ends_with('*') && number.is_empty()
            || number
                .parse::<f64>()
                .is_ok_and(|value| value.is_finite() && value >= 0.0))
        {
            return Err(GenerationError::Invalid(format!(
                "Invalid {length_property} '{length}' on <{definition}>"
            )));
        }
        let track = if let Some(group) = group {
            let mut chars = group.chars();
            if !chars
                .next()
                .is_some_and(|ch| ch == '_' || ch.is_alphabetic())
                || !chars.all(|ch| ch == '_' || ch.is_alphanumeric())
            {
                return Err(GenerationError::Invalid(format!(
                    "Invalid SharedSizeGroup '{group}' on <{definition}>"
                )));
            }
            format!("{length}@{group}")
        } else {
            length.to_string()
        };
        tracks.push(track);
    }
    Ok(tracks.join(","))
}

fn compile_item_template(
    ir: &ProjectionIr,
    owner: &XmlNode,
    state: &mut GenerationState,
    indent: usize,
) -> Result<(ItemContext, String, Vec<NamedRef>), GenerationError> {
    let mut item_templates = owner.children.iter().filter(|child| {
        child
            .tag
            .split_once('.')
            .is_some_and(|(element_owner, property)| {
                element_owner == owner.tag && property == "ItemTemplate"
            })
    });
    let item_template = item_templates.next().ok_or_else(|| {
        GenerationError::Invalid(
            "ItemsSource requires an ItemTemplate with x:DataType and x:Key".to_string(),
        )
    })?;
    if item_templates.next().is_some() {
        return Err(GenerationError::Invalid(
            "ItemsSource supports only one ItemTemplate".to_string(),
        ));
    }
    if item_template.namespace.as_deref() != Some(AVALONIA_XML_NAMESPACE)
        || !item_template.attributes.is_empty()
    {
        return Err(GenerationError::Invalid(
            "ItemTemplate property elements require the Avalonia namespace and no attributes"
                .to_string(),
        ));
    }
    if item_template.children.len() != 1
        || item_template.children[0].tag != "DataTemplate"
        || item_template.children[0].namespace.as_deref() != Some(AVALONIA_XML_NAMESPACE)
        || !item_template.text.trim().is_empty()
    {
        return Err(GenerationError::Invalid(
            "ItemTemplate must contain exactly one DataTemplate".to_string(),
        ));
    }
    let template = &item_template.children[0];
    if template.children.len() != 1 || !template.text.trim().is_empty() {
        return Err(GenerationError::Invalid(
            "DataTemplate must contain exactly one root control".to_string(),
        ));
    }
    let item_type = template
        .attributes
        .iter()
        .find(|attribute| attribute.name == "x:DataType")
        .map(|attribute| attribute.value.trim())
        .filter(|value| is_rust_type_path(value))
        .ok_or_else(|| {
            GenerationError::Invalid(
                "DataTemplate requires x:DataType with a Rust type path".to_string(),
            )
        })?
        .to_string();
    let key = template
        .attributes
        .iter()
        .find(|attribute| attribute.name == "x:Key")
        .map(|attribute| attribute.value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            GenerationError::Invalid(
                "DataTemplate requires x:Key naming a stable Rust key field".to_string(),
            )
        })?;
    for attribute in &template.attributes {
        if attribute.name != "x:DataType" && attribute.name != "x:Key" {
            return Err(GenerationError::Invalid(format!(
                "Unsupported DataTemplate attribute '{}'",
                attribute.name
            )));
        }
    }
    let key_field = to_snake(key);
    if !is_rust_identifier(&key_field) {
        return Err(GenerationError::Invalid(format!(
            "DataTemplate key '{}' does not produce a valid Rust field name",
            key
        )));
    }
    let context = ItemContext {
        item_type,
        key_field,
        signal_name: "__item_signal".to_string(),
    };
    let mut template_refs = Vec::new();
    let mut template_bindings = Vec::new();
    let mut template_handlers = Vec::new();
    let view = emit_node(
        ir,
        &template.children[0],
        &mut template_refs,
        &mut template_bindings,
        &mut template_handlers,
        state,
        EmitPosition {
            indent,
            item_context: Some(&context),
            is_root: false,
        },
    )?;
    if !template_bindings.is_empty() || !template_handlers.is_empty() {
        return Err(GenerationError::Invalid(
            "DataTemplate item bindings cannot add outer resources or events".to_string(),
        ));
    }
    Ok((context, view, template_refs))
}

fn is_rust_type_path(value: &str) -> bool {
    let value = value.strip_prefix("::").unwrap_or(value);
    !value.is_empty()
        && value.split("::").all(|segment| {
            matches!(segment, "crate" | "self" | "super") || is_rust_identifier(segment)
        })
}

fn property_builder_name(
    property: &ProjectedProperty,
    reserved_methods: &HashSet<String>,
) -> String {
    let snake = to_snake(&property.name);
    if property.kind == "Bool" || property.kind == "NullableBool" {
        if let Some(stripped) = snake.strip_prefix("is_") {
            if !reserved_methods.contains(stripped) {
                return stripped.to_string();
            }
        }
    }
    snake
}

fn format_attached_value(
    ir: &ProjectionIr,
    property: &crate::ir::ProjectedAttachedProperty,
    raw: &str,
    bindings: &mut Vec<BindingField>,
) -> Result<String, GenerationError> {
    if !is_supported_binding_kind(&property.kind) && parse_binding_extension(raw)?.is_some() {
        return Err(GenerationError::Invalid(format!(
            "Bindings to attached property '{}.{}' of kind '{}' are not supported",
            property.owner_name, property.name, property.kind
        )));
    }
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

    if let Some(parsed) = parse_binding_extension(raw)? {
        if parsed.two_way {
            return Err(GenerationError::Invalid(format!(
                "TwoWay binding to attached property '{}.{}' is not supported",
                property.owner_name, property.name
            )));
        }
        if parsed.string_format.is_some() && property.kind != "StringUtf16" {
            return Err(GenerationError::Invalid(format!(
                "StringFormat requires a string attached property, not '{}.{}'",
                property.owner_name, property.name
            )));
        }
        return format_binding_expr(&parsed, &rust_type, bindings);
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
    lineage: &[&ProjectedType],
    flags_enums: &std::collections::HashSet<&str>,
    raw: &str,
    bindings: &mut Vec<BindingField>,
    default_mode: Option<AxamlBindingMode>,
) -> Result<String, GenerationError> {
    if property.name == "Content"
        && property.kind == "ComInterface"
        && property.interface_name.as_deref().map(simple_name) == Some("IAvnControl")
    {
        if let Some(binding) = parse_binding_extension(raw)? {
            if binding.two_way || binding.string_format.is_some() || binding.converter.is_some() {
                return Err(GenerationError::Invalid(
                    "Content bindings support only a one-way model binding".to_string(),
                ));
            }
            let field_name = binding_field_name(&binding.path)?;
            register_control_slot_binding(bindings, &field_name, &binding.path)?;
            return Ok(format!("bindings.{field_name}.host()"));
        }
    }

    if let Some(key) = parse_ui_res_extension(raw)? {
        if property.kind != "StringUtf16" {
            return Err(GenerationError::Invalid(format!(
                "UIRes is supported only for string properties, not '{}'",
                property.name
            )));
        }
        let field_name = resource_field_name(&key);
        if !is_rust_identifier(&field_name) {
            return Err(GenerationError::Invalid(format!(
                "UIRes key '{key}' does not produce a valid Rust field name"
            )));
        }
        register_binding(bindings, &field_name, "String", &format!("UIRes:{key}"))?;
        return Ok(format!("bindings.{field_name}"));
    }

    if let Some(resource) = parse_resource_extension(raw)? {
        let rust_type = property_rust_type(property, flags_enums);
        if !is_supported_binding_kind(&property.kind) {
            return Err(GenerationError::Invalid(format!(
                "{} is supported only for scalar properties, not '{}'",
                resource.extension, property.name
            )));
        }
        let field_name = resource_field_name(&resource.key);
        if !is_rust_identifier(&field_name) {
            return Err(GenerationError::Invalid(format!(
                "{} key '{}' does not produce a valid Rust field name",
                resource.extension, resource.key
            )));
        }
        register_binding(
            bindings,
            &field_name,
            &rust_type,
            &format!("Resource:{}", resource.key),
        )?;
        return Ok(if resource.extension == "StaticResource" {
            format!("bindings.{field_name}.get_untracked()")
        } else {
            format!("bindings.{field_name}")
        });
    }

    if let Some(mut parsed) = parse_binding_extension(raw)? {
        if !is_supported_binding_kind(&property.kind) {
            return Err(GenerationError::Invalid(format!(
                "Bindings to property '{}' of kind '{}' are not supported",
                property.name, property.kind
            )));
        }
        if parsed.string_format.is_some() && property.kind != "StringUtf16" {
            return Err(GenerationError::Invalid(format!(
                "StringFormat requires a string property, not '{}'",
                property.name
            )));
        }
        let supports_two_way =
            crate::emit_safe::supports_two_way_property(property, |event_name| {
                lineage
                    .iter()
                    .any(|owner| owner.events.iter().any(|event| event.name == event_name))
            });
        if !parsed.mode_explicit && supports_two_way {
            parsed.two_way = match default_mode {
                Some(AxamlBindingMode::OneWay) => false,
                Some(AxamlBindingMode::TwoWay) => true,
                None => {
                    return Err(GenerationError::Invalid(format!(
                        "Binding to property '{}' requires an explicit Mode=OneWay or Mode=TwoWay",
                        property.name
                    )))
                }
            };
        }
        if parsed.two_way && !supports_two_way {
            return Err(GenerationError::Invalid(format!(
                "TwoWay binding to property '{}' is not supported by its builder",
                property.name
            )));
        }
        if parsed.two_way && parsed.string_format.is_some() {
            return Err(GenerationError::Invalid(
                "StringFormat is not supported with TwoWay bindings".to_string(),
            ));
        }
        let rust_type = property_rust_type(property, flags_enums);
        return format_binding_expr(&parsed, &rust_type, bindings);
    }

    match property.kind.as_str() {
        "StringUtf16" => {
            let literal = raw.strip_prefix("{}").unwrap_or(raw);
            Ok(format!("{literal:?}"))
        }
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

fn format_item_property_value(
    property: &ProjectedProperty,
    lineage: &[&ProjectedType],
    raw: &str,
    context: &ItemContext,
    flags_enums: &std::collections::HashSet<&str>,
    default_mode: Option<AxamlBindingMode>,
) -> Result<Option<String>, GenerationError> {
    let Some(mut binding) = parse_binding_extension(raw)? else {
        return Ok(None);
    };
    if binding.converter.is_some() {
        return Err(GenerationError::Invalid(
            "DataTemplate item converter bindings are not supported".to_string(),
        ));
    }
    if !is_supported_binding_kind(&property.kind) {
        return Err(GenerationError::Invalid(format!(
            "DataTemplate binding to property '{}' of kind '{}' is not supported",
            property.name, property.kind
        )));
    }
    if binding.string_format.is_some() && property.kind != "StringUtf16" {
        return Err(GenerationError::Invalid(format!(
            "StringFormat requires a string property, not '{}'",
            property.name
        )));
    }
    if !binding.mode_explicit
        && crate::emit_safe::supports_two_way_property(property, |event_name| {
            lineage
                .iter()
                .any(|owner| owner.events.iter().any(|event| event.name == event_name))
        })
    {
        binding.two_way = match default_mode {
            Some(AxamlBindingMode::OneWay) => false,
            Some(AxamlBindingMode::TwoWay) => true,
            None => {
                return Err(GenerationError::Invalid(format!(
                    "DataTemplate binding to property '{}' requires an explicit Mode=OneWay",
                    property.name
                )))
            }
        };
    }
    if property.kind == "I32"
        && property
            .managed_type_name
            .as_deref()
            .is_some_and(|managed_type| flags_enums.contains(managed_type))
    {
        return Err(GenerationError::Invalid(format!(
            "DataTemplate binding to flags enum property '{}' is not supported",
            property.name
        )));
    }
    Ok(Some(format_item_binding_expression(&binding, context)?))
}

fn format_item_binding_expression(
    binding: &ParsedBinding,
    context: &ItemContext,
) -> Result<String, GenerationError> {
    if binding.converter.is_some() {
        return Err(GenerationError::Invalid(
            "DataTemplate item converter bindings are not supported".to_string(),
        ));
    }
    if binding.path.contains('.') {
        return Err(GenerationError::Invalid(
            "Nested paths in DataTemplate item bindings are not supported".to_string(),
        ));
    }
    let field_name = to_snake(&binding.path);
    if !is_rust_identifier(&field_name) {
        return Err(GenerationError::Invalid(format!(
            "DataTemplate binding path '{}' is not a valid Rust field name",
            binding.path
        )));
    }
    let item_access = format!("{}.get().{}.clone()", context.signal_name, field_name);
    if binding.two_way {
        return Ok(format!("{item_access}.two_way()"));
    }
    if let Some(fmt_raw) = &binding.string_format {
        let unescaped = fmt_raw.strip_prefix("{}").unwrap_or(fmt_raw);
        let rust_fmt = unescaped.replace("{0}", "{}");
        Ok(format!(
            "rustolonia::Memo::new_scoped(move || format!({rust_fmt:?}, {item_access}))"
        ))
    } else {
        Ok(format!(
            "rustolonia::Memo::new_scoped(move || {item_access})"
        ))
    }
}

fn is_supported_binding_kind(kind: &str) -> bool {
    matches!(
        kind,
        "StringUtf16"
            | "Brush"
            | "Thickness"
            | "CornerRadius"
            | "Size"
            | "Point"
            | "Vector"
            | "Rect"
            | "Color"
            | "I32"
            | "I64"
            | "TimeSpanI64"
            | "F32"
            | "F64"
            | "Bool"
            | "NullableBool"
            | "CharUtf16"
    )
}

fn property_rust_type(
    property: &ProjectedProperty,
    flags_enums: &std::collections::HashSet<&str>,
) -> String {
    match property.kind.as_str() {
        "StringUtf16" => "String".to_string(),
        "Brush" => "Option<rustolonia::Brush>".to_string(),
        "Thickness" | "CornerRadius" | "Size" | "Point" | "Vector" | "Rect" | "Color" => {
            let name = format!("rustolonia::{}", property.kind);
            if property.is_nullable {
                format!("Option<{name}>")
            } else {
                name
            }
        }
        "TimeSpanI64" => "std::time::Duration".to_string(),
        "I32" if is_enum_property(property, flags_enums) => {
            format!(
                "rustolonia::{}",
                simple_name(property.managed_type_name.as_deref().unwrap())
            )
        }
        kind => rust_scalar_kind(kind).to_string(),
    }
}

fn register_binding(
    bindings: &mut Vec<BindingField>,
    field_name: &str,
    rust_type: &str,
    source: &str,
) -> Result<(), GenerationError> {
    if let Some(binding) = bindings.iter().find(|b| b.field_name == field_name) {
        if binding.source != source {
            return Err(GenerationError::Invalid(format!(
                "AXAML sources '{}' and '{source}' conflict on Rust binding field '{field_name}'",
                binding.source
            )));
        }
        if binding.rust_type != rust_type || !binding.is_signal {
            return Err(GenerationError::Invalid(format!(
                "Binding '{field_name}' is used with incompatible Rust types '{}' and '{rust_type}'",
                binding.rust_type
            )));
        }
    } else {
        bindings.push(BindingField {
            field_name: field_name.to_string(),
            rust_type: rust_type.to_string(),
            source: source.to_string(),
            is_signal: true,
            is_copy: true,
        });
    }
    Ok(())
}

fn register_control_slot_binding(
    bindings: &mut Vec<BindingField>,
    field_name: &str,
    source: &str,
) -> Result<(), GenerationError> {
    if bindings.iter().any(|binding| {
        binding.field_name == field_name
            && binding.source == source
            && binding.rust_type == "rustolonia::ControlSlot"
    }) {
        return Err(GenerationError::Invalid(format!(
            "Content binding '{source}' is used more than once; each ControlSlot needs its own host"
        )));
    }
    register_value_binding(
        bindings,
        field_name,
        "rustolonia::ControlSlot",
        source,
        true,
    )
}

fn register_value_binding(
    bindings: &mut Vec<BindingField>,
    field_name: &str,
    rust_type: &str,
    source: &str,
    is_copy: bool,
) -> Result<(), GenerationError> {
    if let Some(binding) = bindings
        .iter()
        .find(|binding| binding.field_name == field_name)
    {
        if binding.source != source || binding.rust_type != rust_type || binding.is_signal {
            return Err(GenerationError::Invalid(format!(
                "AXAML sources '{}' and '{source}' conflict on Rust binding field '{field_name}'",
                binding.source
            )));
        }
    } else {
        bindings.push(BindingField {
            field_name: field_name.to_string(),
            rust_type: rust_type.to_string(),
            source: source.to_string(),
            is_signal: false,
            is_copy,
        });
    }
    Ok(())
}

fn format_binding_expr(
    parsed: &ParsedBinding,
    rust_type: &str,
    bindings: &mut Vec<BindingField>,
) -> Result<String, GenerationError> {
    if let Some(reference) = &parsed.converter {
        if !matches!(
            rust_type,
            "String" | "bool" | "Option<bool>" | "i32" | "i64" | "f64"
        ) {
            return Err(GenerationError::Invalid(format!(
                "Converter binding to Rust type '{rust_type}' is not supported"
            )));
        }
        let source_field = format!("{}_converter_source", binding_field_name(&parsed.path)?);
        register_value_binding(
            bindings,
            &source_field,
            "rustolonia::ScalarPath",
            &format!("ConverterSource:{}", parsed.path),
            false,
        )?;
        let converter_field = converter_field_name(reference);
        register_value_binding(
            bindings,
            &converter_field,
            "rustolonia::RegisteredValueConverter",
            &format!("Converter:{reference}"),
            false,
        )?;
        let parameter = parsed.converter_parameter.as_ref().map_or_else(
            || "rustolonia::ScalarValue::Null".to_string(),
            |value| format!("rustolonia::ScalarValue::String({value:?}.to_string())"),
        );
        let culture = parsed
            .converter_culture
            .as_ref()
            .map_or_else(|| "None".to_string(), |value| format!("Some({value:?})"));
        let mode = if parsed.two_way { ".two_way()" } else { "" };
        return Ok(format!(
            "{{ let __source = bindings.{source_field}.clone(); let __converter = bindings.{converter_field}.clone(); rustolonia::ConvertedBinding::<{rust_type}>::new(__source, __converter, {reference:?}, {parameter}, {culture}){mode} }}"
        ));
    }
    let field_name = binding_field_name(&parsed.path)?;
    if parsed.path.contains('.') {
        register_value_binding(
            bindings,
            &field_name,
            &format!("rustolonia::PropertyPath<{rust_type}>"),
            &parsed.path,
            false,
        )?;
        return Ok(if let Some(fmt_raw) = &parsed.string_format {
            let unescaped = fmt_raw.strip_prefix("{}").unwrap_or(fmt_raw);
            let rust_fmt = unescaped.replace("{0}", "{}");
            format!(
                "{{ let __path = bindings.{field_name}.clone(); rustolonia::PropertyPath::read_only(move || Ok(format!({rust_fmt:?}, __path.get()?))) }}"
            )
        } else if parsed.two_way {
            format!("bindings.{field_name}.two_way()")
        } else {
            format!("bindings.{field_name}.clone()")
        });
    }
    register_binding(bindings, &field_name, rust_type, &parsed.path)?;
    let expr = if let Some(fmt_raw) = &parsed.string_format {
        let unescaped = fmt_raw.strip_prefix("{}").unwrap_or(fmt_raw);
        let rust_fmt = unescaped.replace("{0}", "{}");
        format!(
            "{{ let __sig = bindings.{field_name}; move || format!({rust_fmt:?}, __sig.get()) }}"
        )
    } else if parsed.two_way {
        format!("bindings.{field_name}.two_way()")
    } else {
        format!("bindings.{field_name}")
    };
    Ok(expr)
}

fn parse_ui_res_extension(raw: &str) -> Result<Option<String>, GenerationError> {
    let trimmed = raw.trim();
    let Some(suffix) = trimmed.strip_prefix("{UIRes") else {
        return Ok(None);
    };
    if !suffix.starts_with('}') && !suffix.chars().next().is_some_and(char::is_whitespace) {
        return Ok(None);
    }
    let inner = suffix;
    let inner = inner.strip_suffix('}').ok_or_else(|| {
        GenerationError::Invalid(format!("Unterminated AXAML markup extension '{trimmed}'"))
    })?;
    let key = inner.trim();
    let key = key.strip_prefix("Key=").unwrap_or(key).trim();
    if key.is_empty() || key.contains('{') || key.contains('}') || key.contains(',') {
        return Err(GenerationError::Invalid(format!(
            "UIRes requires one literal resource key, found '{key}'"
        )));
    }
    Ok(Some(key.to_string()))
}

fn parse_resource_extension(raw: &str) -> Result<Option<ParsedResource>, GenerationError> {
    let trimmed = raw.trim();
    for extension in ["StaticResource", "DynamicResource"] {
        let Some(suffix) = trimmed.strip_prefix(&format!("{{{extension}")) else {
            continue;
        };
        if !suffix.starts_with('}') && !suffix.chars().next().is_some_and(char::is_whitespace) {
            continue;
        }
        let inner = suffix.strip_suffix('}').ok_or_else(|| {
            GenerationError::Invalid(format!("Unterminated AXAML markup extension '{trimmed}'"))
        })?;
        let key = inner.trim();
        let key = key.strip_prefix("Key=").unwrap_or(key).trim();
        if key.is_empty() || key.contains('{') || key.contains('}') || key.contains(',') {
            return Err(GenerationError::Invalid(format!(
                "{extension} requires one literal resource key, found '{key}'"
            )));
        }
        return Ok(Some(ParsedResource {
            key: key.to_string(),
            extension,
        }));
    }
    Ok(None)
}

fn parse_binding_extension(raw: &str) -> Result<Option<ParsedBinding>, GenerationError> {
    let trimmed = raw.trim();
    if trimmed.starts_with("{}") {
        return Ok(None);
    }
    if !trimmed.starts_with('{') {
        reject_markup_in_nonbinding_value(trimmed)?;
        return Ok(None);
    }
    let inner = trimmed.strip_suffix('}').ok_or_else(|| {
        GenerationError::Invalid(format!("Unterminated AXAML markup extension '{trimmed}'"))
    })?;
    let inner = inner
        .strip_prefix("{Binding")
        .or_else(|| inner.strip_prefix("{CompiledBinding"))
        .filter(|suffix| suffix.is_empty() || suffix.starts_with(char::is_whitespace))
        .ok_or_else(|| {
            let extension = trimmed
                .strip_prefix('{')
                .and_then(|value| value.split([' ', '}']).next())
                .unwrap_or(trimmed);
            GenerationError::Invalid(format!("Unsupported AXAML markup extension '{extension}'"))
        })?
        .trim();
    if inner.is_empty() {
        return Err(GenerationError::Invalid(
            "Binding markup extension requires a property path".to_string(),
        ));
    }
    let mut path: Option<String> = None;
    let mut two_way = false;
    let mut mode_seen = false;
    let mut string_format: Option<String> = None;
    let mut converter: Option<String> = None;
    let mut converter_parameter: Option<String> = None;
    let mut converter_culture: Option<String> = None;
    for part in split_binding_clauses(inner)? {
        let part = part.trim();
        if let Some(p) = part.strip_prefix("Path=") {
            if path.is_some() {
                return Err(GenerationError::Invalid(
                    "Binding path is specified more than once".to_string(),
                ));
            }
            path = Some(p.trim().to_string());
        } else if let Some(mode) = part.strip_prefix("Mode=") {
            if mode_seen {
                return Err(GenerationError::Invalid(
                    "Binding mode is specified more than once".to_string(),
                ));
            }
            mode_seen = true;
            if mode.trim().eq_ignore_ascii_case("TwoWay") {
                two_way = true;
            } else if mode.trim().eq_ignore_ascii_case("OneWay") {
                two_way = false;
            } else {
                return Err(GenerationError::Invalid(format!(
                    "Unsupported binding mode '{}'",
                    mode.trim()
                )));
            }
        } else if let Some(fmt) = part.strip_prefix("StringFormat=") {
            if string_format.is_some() {
                return Err(GenerationError::Invalid(
                    "Binding StringFormat is specified more than once".to_string(),
                ));
            }
            let fmt = fmt.trim();
            let fmt = if fmt.starts_with('\'') || fmt.starts_with('"') {
                let quote = fmt.chars().next().unwrap();
                if fmt.len() < 2 || !fmt.ends_with(quote) {
                    return Err(GenerationError::Invalid(
                        "Binding StringFormat has an unterminated quote".to_string(),
                    ));
                }
                &fmt[1..fmt.len() - 1]
            } else {
                fmt
            };
            let unescaped = fmt.strip_prefix("{}").unwrap_or(fmt);
            let Some((before, after)) = unescaped.split_once("{0}") else {
                return Err(GenerationError::Invalid(
                    "StringFormat requires exactly one {0} placeholder".to_string(),
                ));
            };
            if before.contains(['{', '}']) || after.contains(['{', '}']) {
                return Err(GenerationError::Invalid(
                    "StringFormat supports only one {0} placeholder and no other braces"
                        .to_string(),
                ));
            }
            string_format = Some(fmt.to_string());
        } else if let Some(value) = part.strip_prefix("Converter=") {
            if converter.is_some() {
                return Err(GenerationError::Invalid(
                    "Binding converter is specified more than once".to_string(),
                ));
            }
            converter = Some(parse_converter_reference(value.trim())?);
        } else if let Some(value) = part.strip_prefix("ConverterParameter=") {
            if converter_parameter.is_some() {
                return Err(GenerationError::Invalid(
                    "Binding converter parameter is specified more than once".to_string(),
                ));
            }
            converter_parameter = Some(parse_binding_literal(value, "ConverterParameter")?);
        } else if let Some(value) = part.strip_prefix("ConverterCulture=") {
            if converter_culture.is_some() {
                return Err(GenerationError::Invalid(
                    "Binding converter culture is specified more than once".to_string(),
                ));
            }
            let culture = parse_binding_literal(value, "ConverterCulture")?;
            if culture.is_empty() {
                return Err(GenerationError::Invalid(
                    "ConverterCulture must not be empty".to_string(),
                ));
            }
            converter_culture = Some(culture);
        } else if path.is_none() && !part.contains('=') {
            path = Some(part.to_string());
        } else {
            return Err(GenerationError::Invalid(format!(
                "Unsupported binding clause '{part}'"
            )));
        }
    }
    let path = path.ok_or_else(|| {
        GenerationError::Invalid("Binding markup extension requires a property path".to_string())
    })?;
    if two_way && string_format.is_some() {
        return Err(GenerationError::Invalid(
            "StringFormat is not supported with TwoWay bindings".to_string(),
        ));
    }
    if converter.is_none() && (converter_parameter.is_some() || converter_culture.is_some()) {
        return Err(GenerationError::Invalid(
            "ConverterParameter and ConverterCulture require a Converter".to_string(),
        ));
    }
    if converter.is_some() && string_format.is_some() {
        return Err(GenerationError::Invalid(
            "StringFormat combined with Converter is not supported".to_string(),
        ));
    }
    binding_field_name(&path)?;
    Ok(Some(ParsedBinding {
        path,
        two_way,
        mode_explicit: mode_seen,
        string_format,
        converter,
        converter_parameter,
        converter_culture,
    }))
}

fn parse_binding_literal(raw: &str, clause: &str) -> Result<String, GenerationError> {
    let raw = raw.trim();
    let value = if raw.starts_with('\'') || raw.starts_with('"') {
        let quote = raw.chars().next().unwrap();
        if raw.len() < 2 || !raw.ends_with(quote) {
            return Err(GenerationError::Invalid(format!(
                "{clause} has an unterminated quote"
            )));
        }
        &raw[1..raw.len() - 1]
    } else {
        raw
    };
    if value.contains(['{', '}']) {
        return Err(GenerationError::Invalid(format!(
            "{clause} supports only a literal string"
        )));
    }
    Ok(value.to_string())
}

fn parse_converter_reference(raw: &str) -> Result<String, GenerationError> {
    let inner = raw
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| {
            GenerationError::Invalid(
                "Converter must be an x:Static or StaticResource reference".to_string(),
            )
        })?;
    let mut parts = inner.split_whitespace();
    let extension = parts.next().unwrap_or_default();
    let value = parts.next().unwrap_or_default();
    if parts.next().is_some() || value.is_empty() {
        return Err(GenerationError::Invalid(
            "Converter requires exactly one static member or resource key".to_string(),
        ));
    }
    match extension {
        "x:Static" => {
            let (owner, member) = value.rsplit_once('.').ok_or_else(|| {
                GenerationError::Invalid(
                    "x:Static converter must name a type and static member".to_string(),
                )
            })?;
            let valid = |part: &str| {
                !part.is_empty()
                    && part
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            };
            if !owner.split([':', '.']).all(valid) || !valid(member) {
                return Err(GenerationError::Invalid(format!(
                    "Unsupported x:Static converter member '{value}'"
                )));
            }
            Ok(format!("x:Static {value}"))
        }
        "StaticResource" if !value.contains(['{', '}', ',']) => {
            Ok(format!("StaticResource {value}"))
        }
        _ => Err(GenerationError::Invalid(format!(
            "Unsupported converter reference '{raw}'; use x:Static or StaticResource"
        ))),
    }
}

fn binding_field_name(path: &str) -> Result<String, GenerationError> {
    let segments: Vec<_> = path.split('.').collect();
    let valid_segment = |segment: &&str| {
        let mut chars = segment.chars();
        chars
            .next()
            .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
            && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    };
    let field_name = segments
        .iter()
        .map(|segment| to_snake(segment))
        .collect::<Vec<_>>()
        .join("_");
    if !segments.iter().all(valid_segment) || !is_rust_identifier(&field_name) {
        return Err(GenerationError::Invalid(format!(
            "Binding path '{path}' is not a supported Rust property path"
        )));
    }
    Ok(field_name)
}

fn reject_markup_in_nonbinding_value(raw: &str) -> Result<(), GenerationError> {
    if raw.contains('{') || raw.contains('}') {
        return Err(GenerationError::Invalid(format!(
            "Mixed or escaped AXAML markup expression '{raw}' is not supported"
        )));
    }
    Ok(())
}

fn split_binding_clauses(input: &str) -> Result<Vec<&str>, GenerationError> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut depth = 0usize;
    for (idx, ch) in input.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '{' if !in_single && !in_double => depth += 1,
            '}' if !in_single && !in_double => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    GenerationError::Invalid("Unexpected '}' in Binding expression".to_string())
                })?;
            }
            ',' if !in_single && !in_double && depth == 0 => {
                result.push(&input[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }
    if in_single || in_double || depth != 0 {
        return Err(GenerationError::Invalid(
            "Unterminated quote or nested markup in Binding expression".to_string(),
        ));
    }
    result.push(&input[start..]);
    Ok(result)
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

fn resolve_node_type<'a>(
    ir: &'a ProjectionIr,
    node: &XmlNode,
    options: &AxamlCompileOptions,
    is_root: bool,
) -> Result<&'a ProjectedType, GenerationError> {
    let matches = |namespace_uri: &str, name: &str| {
        node.namespace.as_deref() == Some(namespace_uri) && node.tag == name
    };
    if let Some(component) = options
        .components
        .iter()
        .find(|component| matches(&component.namespace_uri, &component.name))
    {
        return resolve_control_type(ir, &component.projected_control);
    }
    if is_root {
        if let Some(alias) = options
            .root_aliases
            .iter()
            .find(|alias| matches(&alias.namespace_uri, &alias.name))
        {
            return resolve_control_type(ir, &alias.projected_control);
        }
    }
    if node.namespace.as_deref() != Some(AVALONIA_XML_NAMESPACE) {
        return Err(GenerationError::Invalid(format!(
            "Unsupported AXAML element namespace on <{}>",
            node.tag
        )));
    }
    resolve_control_type(ir, &node.tag)
}

fn validate_options(
    ir: &ProjectionIr,
    options: &AxamlCompileOptions,
) -> Result<(), GenerationError> {
    let mut elements = HashSet::new();
    for (namespace, name, projected) in options
        .root_aliases
        .iter()
        .map(|alias| (&alias.namespace_uri, &alias.name, &alias.projected_control))
        .chain(options.components.iter().map(|component| {
            (
                &component.namespace_uri,
                &component.name,
                &component.projected_control,
            )
        }))
    {
        if namespace.is_empty() || !is_xml_name(name) {
            return Err(GenerationError::Invalid(format!(
                "Invalid registered AXAML element '{{{namespace}}}{name}'"
            )));
        }
        if !elements.insert((namespace, name)) {
            return Err(GenerationError::Invalid(format!(
                "Duplicate AXAML element registration '{{{namespace}}}{name}'"
            )));
        }
        if namespace == AVALONIA_XML_NAMESPACE && resolve_control_type(ir, name).is_ok() {
            return Err(GenerationError::Invalid(format!(
                "AXAML element registration <{name}> shadows a projected control"
            )));
        }
        resolve_control_type(ir, projected)?;
    }
    for component in &options.components {
        if !is_rust_type_path(&component.factory)
            || component
                .after_children
                .as_ref()
                .is_some_and(|path| !is_rust_type_path(path))
            || component
                .finish_method
                .as_ref()
                .is_some_and(|method| !is_rust_identifier(method))
            || (!component.properties.is_empty() && component.finish_method.is_none())
        {
            return Err(GenerationError::Invalid(format!(
                "Invalid Rust builder registration for <{}>",
                component.name
            )));
        }
        let mut properties = HashSet::new();
        for property in &component.properties {
            let type_valid = match &property.value_type {
                AxamlComponentValueType::RustType(path) => is_rust_type_path(path),
                AxamlComponentValueType::TypeArgument(index)
                | AxamlComponentValueType::VecOfTypeArgument(index) => {
                    *index < component.type_argument_count
                }
            };
            let method_valid = |method: &Option<String>| {
                method
                    .as_ref()
                    .is_none_or(|value| is_rust_identifier(value))
            };
            let has_default_method = match property.default_mode {
                AxamlBindingMode::OneWay => property.one_way_method.is_some(),
                AxamlBindingMode::TwoWay => property.two_way_method.is_some(),
            };
            if !is_xml_name(&property.name)
                || !properties.insert(&property.name)
                || !type_valid
                || !method_valid(&property.one_way_method)
                || !method_valid(&property.two_way_method)
                || !has_default_method
            {
                return Err(GenerationError::Invalid(format!(
                    "Invalid binding property registration '{}.{}'",
                    component.name, property.name
                )));
            }
        }
    }
    let mut types = HashSet::new();
    for mapping in &options.type_mappings {
        if mapping.namespace_uri.is_empty()
            || !is_xml_name(&mapping.name)
            || !is_rust_type_path(&mapping.rust_type)
            || !types.insert((&mapping.namespace_uri, &mapping.name))
        {
            return Err(GenerationError::Invalid(format!(
                "Invalid or duplicate AXAML type mapping '{{{}}}{}'",
                mapping.namespace_uri, mapping.name
            )));
        }
    }
    let mut defaults = HashSet::new();
    for default in &options.binding_defaults {
        if !defaults.insert((&default.projected_control, &default.property)) {
            return Err(GenerationError::Invalid(format!(
                "Duplicate binding default '{}.{}'",
                default.projected_control, default.property
            )));
        }
        let ty = resolve_control_type(ir, &default.projected_control)?;
        let owners = lineage(ir, ty);
        let property = owners
            .iter()
            .flat_map(|owner| owner.properties.iter())
            .find(|property| property.name == default.property && property.can_write)
            .ok_or_else(|| {
                GenerationError::Invalid(format!(
                    "Unknown writable property in binding default '{}.{}'",
                    default.projected_control, default.property
                ))
            })?;
        if default.mode == AxamlBindingMode::TwoWay
            && !crate::emit_safe::supports_two_way_property(property, |event_name| {
                owners
                    .iter()
                    .any(|owner| owner.events.iter().any(|event| event.name == event_name))
            })
        {
            return Err(GenerationError::Invalid(format!(
                "TwoWay binding default is not supported for '{}.{}'",
                default.projected_control, default.property
            )));
        }
    }
    Ok(())
}

fn is_xml_name(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
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

fn is_rust_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic())
        || !chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        return false;
    }

    !matches!(
        value,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
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

fn resource_field_name(key: &str) -> String {
    let identifier: String = to_snake(key)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    format!("resource_{identifier}")
}

fn converter_field_name(reference: &str) -> String {
    let (kind, name) = reference
        .strip_prefix("x:Static ")
        .map(|name| ("static", name))
        .or_else(|| {
            reference
                .strip_prefix("StaticResource ")
                .map(|name| ("resource", name))
        })
        .expect("validated converter reference");
    let field = resource_field_name(name);
    let segments = field["resource_".len()..]
        .split('_')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    let name = if segments.is_empty() { "_" } else { &segments };
    format!("converter_{kind}_{name}")
}

fn parse_axaml_document(
    input: &str,
    options: &AxamlCompileOptions,
) -> Result<XmlNode, GenerationError> {
    let document = roxmltree::Document::parse(input).map_err(|error| {
        GenerationError::Invalid(format!("Invalid AXAML XML at {}: {error}", error.pos()))
    })?;
    let understood = options
        .root_aliases
        .iter()
        .map(|alias| alias.namespace_uri.clone())
        .chain(
            options
                .components
                .iter()
                .map(|component| component.namespace_uri.clone()),
        )
        .chain(
            options
                .type_mappings
                .iter()
                .map(|mapping| mapping.namespace_uri.clone()),
        )
        .collect();
    convert_xml_element(document.root_element(), &HashSet::new(), &understood)
}

fn convert_xml_element(
    element: roxmltree::Node<'_, '_>,
    inherited_ignorable: &HashSet<String>,
    understood: &HashSet<String>,
) -> Result<XmlNode, GenerationError> {
    let line = element.document().text_pos_at(element.range().start).row;
    let mut ignorable = inherited_ignorable.clone();
    for attr in element.attributes().filter(|attr| {
        attr.namespace() == Some(MARKUP_COMPATIBILITY_XML_NAMESPACE) && attr.name() == "Ignorable"
    }) {
        for prefix in attr.value().split_whitespace() {
            let namespace = element.lookup_namespace_uri(Some(prefix)).ok_or_else(|| {
                GenerationError::Invalid(format!(
                    "mc:Ignorable references undeclared prefix '{prefix}' (line {line})"
                ))
            })?;
            ignorable.insert(namespace.to_string());
        }
    }

    let attributes = element
        .attributes()
        .filter_map(|attr| {
            let namespace = attr.namespace().map(str::to_string);
            if is_ignorable_namespace(namespace.as_deref(), &ignorable, understood)
                || (namespace.as_deref() == Some(MARKUP_COMPATIBILITY_XML_NAMESPACE)
                    && attr.name() == "Ignorable")
            {
                return None;
            }
            Some((attr, namespace))
        })
        .map(|(attr, namespace)| {
            let name = match namespace.as_deref() {
                Some(XAML_XML_NAMESPACE) => format!("x:{}", attr.name()),
                _ => attr.name().to_string(),
            };
            Ok(XmlAttribute {
                name,
                value: normalize_binding_namespaces(element, attr.value())
                    .map_err(|error| GenerationError::Invalid(format!("{error} (line {line})")))?,
                namespace,
                type_arguments: if attr.namespace() == Some(XAML_XML_NAMESPACE)
                    && attr.name() == "TypeArguments"
                {
                    Some(parse_type_arguments(element, attr.value())?)
                } else {
                    None
                },
            })
        })
        .collect::<Result<Vec<_>, GenerationError>>()?;

    let mut children = Vec::new();
    let mut text = String::new();
    for child in element.children() {
        match child.node_type() {
            roxmltree::NodeType::Element => {
                if is_ignorable_namespace(child.tag_name().namespace(), &ignorable, understood)
                    || (child.tag_name().namespace() == Some(AVALONIA_XML_NAMESPACE)
                        && child.tag_name().name() == "Design.DataContext")
                {
                    continue;
                }
                children.push(convert_xml_element(child, &ignorable, understood)?);
            }
            roxmltree::NodeType::Text => {
                if let Some(value) = child.text() {
                    text.push_str(value);
                }
            }
            _ => {}
        }
    }

    fn parse_type_arguments(
        element: roxmltree::Node<'_, '_>,
        value: &str,
    ) -> Result<Vec<XmlQualifiedName>, GenerationError> {
        let line = element.document().text_pos_at(element.range().start).row;
        value
            .split(',')
            .map(|raw| {
                let raw = raw.trim();
                let (prefix, name) = match raw.split_once(':') {
                    Some((prefix, name)) if is_xml_name(prefix) && is_xml_name(name) => {
                        (Some(prefix), name)
                    }
                    None if is_xml_name(raw) => (None, raw),
                    _ => {
                        return Err(GenerationError::Invalid(format!(
                            "Invalid x:TypeArguments entry '{raw}' (line {line})"
                        )))
                    }
                };
                let namespace = element.lookup_namespace_uri(prefix).ok_or_else(|| {
                    GenerationError::Invalid(format!(
                        "Undeclared x:TypeArguments namespace for '{raw}' (line {line})"
                    ))
                })?;
                Ok(XmlQualifiedName {
                    namespace: namespace.to_string(),
                    name: name.to_string(),
                })
            })
            .collect()
    }

    Ok(XmlNode {
        tag: element.tag_name().name().to_string(),
        namespace: element.tag_name().namespace().map(str::to_string),
        line,
        attributes,
        children,
        text,
    })
}

fn normalize_binding_namespaces(
    element: roxmltree::Node<'_, '_>,
    value: &str,
) -> Result<String, GenerationError> {
    if !value.contains("Converter=") {
        return Ok(value.to_string());
    }
    let trimmed = value.trim();
    let Some(inner) = trimmed.strip_suffix('}') else {
        return Ok(value.to_string());
    };
    let Some((keyword, body)) = ["Binding", "CompiledBinding"]
        .into_iter()
        .find_map(|keyword| {
            inner
                .strip_prefix(&format!("{{{keyword}"))
                .filter(|suffix| suffix.starts_with(char::is_whitespace))
                .map(|body| (keyword, body.trim()))
        })
    else {
        return Ok(value.to_string());
    };
    let mut clauses = Vec::new();
    let mut changed = false;
    for clause in split_binding_clauses(body)? {
        let trimmed_clause = clause.trim();
        let Some(converter) = trimmed_clause.strip_prefix("Converter=").map(str::trim) else {
            clauses.push(trimmed_clause.to_string());
            continue;
        };
        let Some(inner) = converter
            .strip_prefix('{')
            .and_then(|inner| inner.strip_suffix('}'))
        else {
            clauses.push(trimmed_clause.to_string());
            continue;
        };
        let mut words = inner.split_whitespace();
        let extension = words.next().unwrap_or_default();
        let Some((prefix, "Static")) = extension.split_once(':') else {
            clauses.push(trimmed_clause.to_string());
            continue;
        };
        if element.lookup_namespace_uri(Some(prefix)) != Some(XAML_XML_NAMESPACE) {
            let line = element.document().text_pos_at(element.range().start).row;
            return Err(GenerationError::Invalid(format!(
                "Converter prefix '{prefix}' is not bound to the XAML namespace (line {line})"
            )));
        }
        let member = words.next().unwrap_or_default();
        if let Some((type_prefix, _)) = member.split_once(':') {
            if element.lookup_namespace_uri(Some(type_prefix)).is_none() {
                let line = element.document().text_pos_at(element.range().start).row;
                return Err(GenerationError::Invalid(format!(
                    "Converter type prefix '{type_prefix}' is not declared (line {line})"
                )));
            }
        }
        let rest = words.collect::<Vec<_>>().join(" ");
        let trailing = if rest.is_empty() {
            String::new()
        } else {
            format!(" {rest}")
        };
        clauses.push(format!("Converter={{x:Static {member}{trailing}}}"));
        changed = true;
    }
    if changed {
        Ok(format!("{{{keyword} {}}}", clauses.join(", ")))
    } else {
        Ok(value.to_string())
    }
}

fn is_ignorable_namespace(
    namespace: Option<&str>,
    ignorable: &HashSet<String>,
    understood: &HashSet<String>,
) -> bool {
    namespace.is_some_and(|uri| {
        ignorable.contains(uri)
            && !understood.contains(uri)
            && !matches!(
                uri,
                AVALONIA_XML_NAMESPACE | XAML_XML_NAMESPACE | MARKUP_COMPATIBILITY_XML_NAMESPACE
            )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile_test_axaml(axaml: &str) -> Result<String, GenerationError> {
        compile_axaml_from_json(include_str!("../../projection.ir.json"), axaml)
    }

    fn component_options() -> AxamlCompileOptions {
        AxamlCompileOptions {
            root_aliases: vec![AxamlRootAlias {
                namespace_uri: "urn:views".into(),
                name: "FilterEditor".into(),
                projected_control: "UserControl".into(),
            }],
            components: vec![
                AxamlComponent {
                    namespace_uri: AVALONIA_XML_NAMESPACE.into(),
                    name: "FormGrid".into(),
                    projected_control: "Grid".into(),
                    factory: "crate::components::form_grid".into(),
                    type_argument_count: 0,
                    finish_method: None,
                    after_children: Some("crate::components::finish_grid".into()),
                    properties: vec![],
                },
                AxamlComponent {
                    namespace_uri: AVALONIA_XML_NAMESPACE.into(),
                    name: "EnumPicker".into(),
                    projected_control: "ComboBox".into(),
                    factory: "crate::components::enum_picker".into(),
                    type_argument_count: 1,
                    finish_method: Some("into_el".into()),
                    after_children: None,
                    properties: vec![
                        AxamlComponentProperty {
                            name: "SelectedValue".into(),
                            value_type: AxamlComponentValueType::TypeArgument(0),
                            default_mode: AxamlBindingMode::TwoWay,
                            one_way_method: None,
                            two_way_method: Some("selected_value".into()),
                        },
                        AxamlComponentProperty {
                            name: "IncludedValues".into(),
                            value_type: AxamlComponentValueType::VecOfTypeArgument(0),
                            default_mode: AxamlBindingMode::OneWay,
                            one_way_method: Some("included_values".into()),
                            two_way_method: None,
                        },
                    ],
                },
            ],
            type_mappings: vec![AxamlTypeMapping {
                namespace_uri: "urn:models".into(),
                name: "FilterKind".into(),
                rust_type: "crate::models::FilterKind".into(),
            }],
            binding_defaults: vec![AxamlBindingDefault {
                projected_control: "TextBox".into(),
                property: "Text".into(),
                mode: AxamlBindingMode::TwoWay,
            }],
        }
    }

    fn compile_components(
        axaml: &str,
        options: &AxamlCompileOptions,
    ) -> Result<String, GenerationError> {
        compile_axaml_from_json_with_options(
            include_str!("../../projection.ir.json"),
            axaml,
            options,
        )
    }

    const CUSTOM_EDITOR: &str = r#"
<v:FilterEditor xmlns="https://github.com/avaloniaui"
                xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
                xmlns:v="urn:views" xmlns:t="urn:models"
                xmlns:d="urn:design"
                xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
                mc:Ignorable="d" d:DesignWidth="600"
                x:Class="Example.FilterEditor" x:DataType="v:Model">
  <Design.DataContext><v:DesignModel Unknown="ignored" /></Design.DataContext>
  <FormGrid>
    <FormGrid.ColumnDefinitions>
      <ColumnDefinition Width="Auto" SharedSizeGroup="Field" />
      <ColumnDefinition Width="*" />
    </FormGrid.ColumnDefinitions>
    <EnumPicker x:TypeArguments="t:FilterKind"
                SelectedValue="{Binding Mode}" IncludedValues="{Binding Allowed}"
                Grid.Column="0" />
    <TextBox Grid.Column="1" Text="{Binding Text}"
             ScrollViewer.HorizontalScrollBarVisibility="Hidden" />
  </FormGrid>
</v:FilterEditor>"#;

    #[test]
    fn compiles_registered_namespaced_generic_components_and_grid_definitions() {
        let source = compile_components(CUSTOM_EDITOR, &component_options()).unwrap();
        assert!(source.contains("-> (rustolonia::El<rustolonia::UserControl>, FilterEditorRefs)"));
        assert!(source.contains("pub mode: rustolonia::Signal<crate::models::FilterKind>"));
        assert!(source.contains("pub allowed: rustolonia::Signal<Vec<crate::models::FilterKind>>"));
        assert!(source.contains("crate::components::enum_picker::<crate::models::FilterKind>()"));
        assert!(source.contains(
            ".selected_value(bindings.mode).included_values(bindings.allowed).into_el()"
        ));
        assert!(source.contains("crate::components::finish_grid("));
        assert!(source.contains(".column_definitions(\"Auto@Field,*\")"));
        assert!(source.contains(".text(bindings.text.two_way())"));
        assert!(source.contains(
            ".scroll_viewer_horizontal_scroll_bar_visibility(rustolonia::ScrollBarVisibility::Hidden)"
        ));
        assert!(!source.contains("DesignModel"));
        assert!(!source.contains("DesignWidth"));
        assert!(compile_test_axaml(CUSTOM_EDITOR).is_err());
    }

    #[test]
    fn rejects_unknown_or_ambiguous_component_contracts() {
        let options = component_options();
        for (markup, diagnostic) in [
            (
                CUSTOM_EDITOR.replace("t:FilterKind", "t:Missing"),
                "Unregistered x:TypeArguments",
            ),
            (
                CUSTOM_EDITOR.replace("t:FilterKind", "bad:FilterKind"),
                "Undeclared x:TypeArguments",
            ),
            (
                CUSTOM_EDITOR.replace("t:FilterKind", "t:FilterKind,t:FilterKind"),
                "requires 1 x:TypeArguments",
            ),
            (
                CUSTOM_EDITOR.replace(
                    "SelectedValue=\"{Binding Mode}\"",
                    "SelectedValue=\"{Binding Mode, Mode=OneWay}\"",
                ),
                "OneWay binding is not supported",
            ),
            (
                CUSTOM_EDITOR.replace(
                    "SelectedValue=\"{Binding Mode}\"",
                    "SelectedValue=\"{Binding Model.Mode}\"",
                ),
                "direct signal binding",
            ),
            (
                CUSTOM_EDITOR.replace(
                    "IncludedValues=\"{Binding Allowed}\"",
                    "IncludedValues=\"{Binding Allowed, Mode=TwoWay}\"",
                ),
                "TwoWay binding is not supported",
            ),
            (
                CUSTOM_EDITOR.replace("Grid.Column=\"0\"", "Unknown=\"value\""),
                "Unknown or read-only property",
            ),
        ] {
            let error = compile_components(&markup, &options)
                .unwrap_err()
                .to_string();
            assert!(error.contains(diagnostic), "{diagnostic}: {error}");
        }

        let mut duplicate = options.clone();
        duplicate.components.push(duplicate.components[0].clone());
        assert!(compile_components(CUSTOM_EDITOR, &duplicate)
            .unwrap_err()
            .to_string()
            .contains("Duplicate AXAML element"));
        let mut invalid_default = options.clone();
        invalid_default.binding_defaults[0].property = "NoSuchProperty".into();
        assert!(compile_components(CUSTOM_EDITOR, &invalid_default)
            .unwrap_err()
            .to_string()
            .contains("Unknown writable property"));
        let mut mismatched_namespace = options.clone();
        mismatched_namespace.type_mappings[0].namespace_uri = "urn:other".into();
        assert!(compile_components(CUSTOM_EDITOR, &mismatched_namespace)
            .unwrap_err()
            .to_string()
            .contains("Unregistered x:TypeArguments"));
    }

    #[test]
    fn registered_namespaces_are_not_discarded_by_markup_compatibility() {
        let markup = r#"
<StackPanel xmlns="https://github.com/avaloniaui" xmlns:v="urn:views"
            xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
            mc:Ignorable="v"><v:Unknown /></StackPanel>"#;
        assert!(compile_test_axaml(markup).is_ok());
        assert!(compile_components(markup, &component_options())
            .unwrap_err()
            .to_string()
            .contains("Unsupported AXAML element namespace"));
    }

    #[test]
    fn grid_definition_elements_reject_unsupported_or_invalid_data() {
        let options = component_options();
        for (original, replacement) in [
            ("SharedSizeGroup=\"Field\"", "SharedSizeGroup=\"1bad\""),
            ("SharedSizeGroup=\"Field\"", "SharedSizeGroup=\"Bad-Name\""),
            ("Width=\"Auto\"", "Width=\"{Binding Width}\""),
            ("Width=\"Auto\"", "Width=\"NaN\""),
            ("Width=\"Auto\"", "MinWidth=\"5\""),
            ("<ColumnDefinition Width=\"*\" />", "<TextBlock />"),
        ] {
            let markup = CUSTOM_EDITOR.replace(original, replacement);
            assert!(
                compile_components(&markup, &options).is_err(),
                "{replacement}"
            );
        }
        let repeated =
            CUSTOM_EDITOR.replace("<FormGrid>", "<FormGrid ColumnDefinitions=\"Auto,*\">");
        assert!(compile_components(&repeated, &options)
            .unwrap_err()
            .to_string()
            .contains("specified more than once"));
    }

    #[test]
    fn explicit_modes_override_registered_defaults_without_losing_formatting() {
        let options = component_options();
        let one_way = CUSTOM_EDITOR.replace(
            "Text=\"{Binding Text}\"",
            "Text=\"{Binding Text, Mode=OneWay}\"",
        );
        assert!(compile_components(&one_way, &options)
            .unwrap()
            .contains(".text(bindings.text)"));
        let formatted = CUSTOM_EDITOR.replace(
            "Text=\"{Binding Text}\"",
            "Text=\"{Binding Text, StringFormat='{}{0}'}\"",
        );
        assert!(compile_components(&formatted, &options)
            .unwrap_err()
            .to_string()
            .contains("StringFormat is not supported with TwoWay"));
    }

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

        let rust_code = compile_test_axaml(axaml).expect("AXAML should compile to Rust");
        assert!(rust_code.contains("pub struct LoginWindowRefs"));
        assert!(rust_code.contains("pub header_text: rustolonia::NodeRef<rustolonia::TextBlock>"));
        assert!(rust_code.contains("pub username_input: rustolonia::NodeRef<rustolonia::TextBox>"));
        assert!(rust_code.contains("pub submit_button: rustolonia::NodeRef<rustolonia::Button>"));
        assert!(rust_code.contains("pub struct LoginWindowBindings"));
        assert!(rust_code.contains("pub username: rustolonia::Signal<String>"));
        assert!(rust_code.contains("pub remember_me: rustolonia::Signal<Option<bool>>"));
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
            compile_test_axaml(axaml).expect("AXAML with events and DataTemplate should compile");
        assert!(rust_code.contains("pub struct TaskWindowHandlers"));
        assert!(
            rust_code.contains("pub type TaskWindowHandler1 = Box<dyn FnMut(()) + Send + 'static>")
        );
        assert!(rust_code.contains("pub on_add_task: TaskWindowHandler1"));
        assert!(rust_code.contains(".on_click(handlers.on_add_task)"));
        assert!(rust_code.contains("format!(\"{} items\", __sig.get())"));
        assert!(rust_code.contains("rustolonia::data_template_reactive"));
        assert!(rust_code.contains("control.set_item_template(Some(&tpl))"));
    }

    #[test]
    fn rejects_unknown_control_or_property_in_axaml() {
        assert!(compile_test_axaml("<NonExistentControl />").is_err());
        assert!(compile_test_axaml("<Button NoSuchProperty=\"123\" />").is_err());
        assert!(compile_test_axaml("<Button Grid.NoSuchAttached=\"1\" />").is_err());
    }

    #[test]
    fn resolves_aliased_namespaces_and_ignores_declared_design_attributes() {
        let axaml = r#"
<a:Window xmlns:a="https://github.com/avaloniaui"
          xmlns:markup="http://schemas.microsoft.com/winfx/2006/xaml"
          xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
          xmlns:design="http://schemas.microsoft.com/expression/blend/2008"
          mc:Ignorable="design"
          design:DesignWidth="400"
          markup:Class="Example.Views.MainWindow">
  <a:TextBlock Text="Ready &amp; waiting" />
</a:Window>
"#;

        let output = compile_test_axaml(axaml).expect("namespace aliases should resolve");
        assert!(output.contains("pub struct MainWindowRefs"));
        assert!(output.contains(".text(\"Ready & waiting\")"));
    }

    #[test]
    fn rejects_markup_extensions_that_are_not_implemented() {
        let axaml = r#"
<Window xmlns="https://github.com/avaloniaui">
  <TextBlock Text="{Binding Status, Converter={DynamicResource StatusConverter}}" />
</Window>
"#;
        let error = compile_test_axaml(axaml).expect_err("converter must not be discarded");
        assert!(error
            .to_string()
            .contains("Unsupported converter reference"));
    }

    #[test]
    fn rejects_object_resources_without_a_runtime_type_mapping() {
        let axaml = r#"
<ContentControl xmlns="https://github.com/avaloniaui" ContentTemplate="{DynamicResource StatusTemplate}" />
"#;
        let error = compile_test_axaml(axaml).expect_err("object resources need a type mapping");
        assert!(error
            .to_string()
            .contains("DynamicResource is supported only for scalar properties"));
    }

    #[test]
    fn compiles_scalar_resources_to_typed_signals() {
        let axaml = r#"
<Border xmlns="https://github.com/avaloniaui"
        Background="{DynamicResource WindowBackground}" />
"#;
        let rust_code = compile_test_axaml(axaml).expect("scalar resource slots are supported");
        assert!(rust_code.contains("Signal<Option<rustolonia::Brush>>"));
        assert!(rust_code.contains(".background(bindings.resource_window_background)"));
    }

    #[test]
    fn resource_keys_can_contain_hyphens_without_losing_collision_checks() {
        let axaml = r#"
<Border xmlns="https://github.com/avaloniaui"
        Background="{DynamicResource text-brand}" />
"#;
        let rust_code = compile_test_axaml(axaml).expect("hyphenated resource key");
        assert!(rust_code.contains(".background(bindings.resource_text_brand)"));

        let conflicting = r#"
<StackPanel xmlns="https://github.com/avaloniaui">
  <Border Background="{DynamicResource text-brand}" />
  <Border Background="{DynamicResource text_brand}" />
</StackPanel>
"#;
        let error = compile_test_axaml(conflicting).expect_err("field collisions must be rejected");
        assert!(error.to_string().contains("conflict on Rust binding field"));
    }

    #[test]
    fn static_resources_are_snapshots_even_when_the_same_key_is_dynamic_elsewhere() {
        let axaml = r#"
<StackPanel xmlns="https://github.com/avaloniaui">
  <Border Background="{StaticResource Accent}" />
  <Border Background="{DynamicResource Accent}" />
</StackPanel>
"#;
        let rust_code = compile_test_axaml(axaml).expect("mixed static and dynamic resource");
        assert_eq!(rust_code.matches("pub resource_accent:").count(), 1);
        assert!(rust_code.contains(".background(bindings.resource_accent.get_untracked())"));
        assert!(rust_code.contains(".background(bindings.resource_accent)"));
    }

    #[test]
    fn compiles_typed_collection_bindings_and_item_context() {
        let axaml = r#"
<ListBox xmlns="https://github.com/avaloniaui"
         xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
         ItemsSource="{Binding Contacts}">
  <ListBox.ItemTemplate>
    <DataTemplate x:DataType="crate::Contact" x:Key="Id">
      <TextBlock Text="{Binding Name}" />
    </DataTemplate>
  </ListBox.ItemTemplate>
</ListBox>
"#;
        let rust_code = compile_test_axaml(axaml).expect("typed item templates should compile");
        assert!(rust_code.contains("Signal<Vec<crate::Contact>>"));
        assert!(rust_code
            .contains(".items_for(bindings.contacts, |__item: &crate::Contact| std::clone::Clone::clone(&__item.id)"));
        assert!(rust_code.contains(
            "move |__item_signal: rustolonia::Signal<crate::Contact>| rustolonia::TextBlock::el()"
        ));
        assert!(rust_code.contains(
            ".text(rustolonia::Memo::new_scoped(move || __item_signal.get().name.clone()))"
        ));
    }

    #[test]
    fn scopes_named_references_to_each_typed_item_template() {
        let axaml = r#"
<StackPanel xmlns="https://github.com/avaloniaui"
            xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
            x:Class="Views.ContactList" x:Name="Title">
  <ListBox ItemsSource="{Binding Contacts}">
    <ListBox.ItemTemplate>
      <DataTemplate x:DataType="crate::Contact" x:Key="Id">
        <Border x:Name="Container">
          <Border.Child>
            <TextBlock x:Name="Title" Text="{Binding Name}" />
          </Border.Child>
        </Border>
      </DataTemplate>
    </ListBox.ItemTemplate>
  </ListBox>
  <ListBox ItemsSource="{Binding Recent}">
    <ListBox.ItemTemplate>
      <DataTemplate x:DataType="crate::Contact" x:Key="Id">
        <TextBlock x:Name="Title" Text="{Binding Name}" />
      </DataTemplate>
    </ListBox.ItemTemplate>
  </ListBox>
</StackPanel>
"#;
        let output = compile_test_axaml(axaml).expect("item names should be isolated");
        assert!(output.contains("pub struct ContactListItemTemplate1Refs"));
        assert!(output.contains("pub struct ContactListItemTemplate2Refs"));
        assert!(output.contains(
            "pub contacts_item_refs_1: rustolonia::ItemNameScope<crate::Contact, ContactListItemTemplate1Refs>"
        ));
        assert!(output.contains(
            "pub recent_item_refs_2: rustolonia::ItemNameScope<crate::Contact, ContactListItemTemplate2Refs>"
        ));
        assert!(output.contains(".node_ref(refs.title)"));
        assert!(output.contains(".node_ref(__item_refs.title)"));
        assert!(output.contains("rustolonia::NodeRef::new_scoped()"));
        assert_eq!(
            output
                .matches("__names.register(__item_signal, __item_refs)")
                .count(),
            2
        );
    }

    #[test]
    fn rejects_duplicate_names_within_one_item_template() {
        let axaml = r#"
<ListBox xmlns="https://github.com/avaloniaui"
         xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
         ItemsSource="{Binding Contacts}">
  <ListBox.ItemTemplate>
    <DataTemplate x:DataType="crate::Contact" x:Key="Id">
      <StackPanel>
        <TextBlock x:Name="Title" />
        <TextBlock x:Name="Title" />
      </StackPanel>
    </DataTemplate>
  </ListBox.ItemTemplate>
</ListBox>
"#;
        let error = compile_test_axaml(axaml).expect_err("duplicate item names must be rejected");
        assert!(error.to_string().contains("Duplicate AXAML name"));
    }

    #[test]
    fn rejects_typed_collection_templates_without_explicit_model_keys() {
        let axaml = r#"
<ListBox xmlns="https://github.com/avaloniaui"
         xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
         ItemsSource="{Binding Contacts}">
  <ListBox.ItemTemplate>
    <DataTemplate x:DataType="crate::Contact">
      <TextBlock Text="{Binding Name}" />
    </DataTemplate>
  </ListBox.ItemTemplate>
</ListBox>
"#;
        let error = compile_test_axaml(axaml).expect_err("stable key fields are required");
        assert!(error
            .to_string()
            .contains("DataTemplate requires x:Key naming a stable Rust key field"));
    }

    #[test]
    fn lowers_item_context_bindings_in_text_content() {
        let axaml = r#"
<ListBox xmlns="https://github.com/avaloniaui"
         xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
         ItemsSource="{Binding Contacts}">
  <ListBox.ItemTemplate>
    <DataTemplate x:DataType="crate::Contact" x:Key="Id">
      <TextBlock>{Binding Name}</TextBlock>
    </DataTemplate>
  </ListBox.ItemTemplate>
</ListBox>
"#;
        let rust_code =
            compile_test_axaml(axaml).expect("item-context text bindings should compile");
        assert!(rust_code.contains(
            ".rsx_child(rustolonia::Memo::new_scoped(move || __item_signal.get().name.clone()))"
        ));
    }

    #[test]
    fn compiles_ui_res_to_a_typed_string_signal() {
        let axaml = r#"
<TextBlock xmlns="https://github.com/avaloniaui" Text="{UIRes Key=ConnectWhenOnline}" />
"#;
        let rust_code = compile_test_axaml(axaml).expect("UIRes string properties are supported");
        assert!(rust_code.contains("pub resource_connect_when_online: rustolonia::Signal<String>"));
        assert!(rust_code.contains(".text(bindings.resource_connect_when_online)"));
    }

    #[test]
    fn rejects_ui_res_on_non_string_properties_and_key_collisions() {
        let brush = r#"
<Border xmlns="https://github.com/avaloniaui" Background="{UIRes WindowBackground}" />
"#;
        let error = compile_test_axaml(brush).expect_err("UIRes returns strings");
        assert!(error
            .to_string()
            .contains("UIRes is supported only for string properties"));

        let collision = r#"
<Window xmlns="https://github.com/avaloniaui">
  <TextBlock Text="{UIRes aB}" />
  <TextBlock Text="{UIRes a_b}" />
</Window>
"#;
        let error = compile_test_axaml(collision).expect_err("resource keys must not alias");
        assert!(error.to_string().contains("conflict on Rust binding field"));
    }

    #[test]
    fn compiles_nested_paths_to_explicit_rust_paths() {
        let axaml = r##"
<StackPanel xmlns="https://github.com/avaloniaui">
  <TextBlock Text="{Binding User.Name}" />
  <TextBox Text="{Binding User.Name, Mode=TwoWay}" />
  <TextBlock Text="{Binding User.Name, StringFormat='{}{0}!'}" />
</StackPanel>
"##;
        let output = compile_test_axaml(axaml).expect("nested paths should compile");
        assert_eq!(output.matches("pub user_name:").count(), 1);
        assert!(output.contains("pub user_name: rustolonia::PropertyPath<String>"));
        assert!(output.contains(".text(bindings.user_name.clone())"));
        assert!(output.contains(".text(bindings.user_name.two_way())"));
        assert!(output.contains("format!(\"{}!\", __path.get()?)"));
    }

    #[test]
    fn rejects_invalid_and_ambiguous_nested_paths() {
        for path in ["User..Name", ".Name", "User.", "User[0].Name", "!User.Name"] {
            let axaml = format!(
                r#"<TextBlock xmlns="https://github.com/avaloniaui" Text="{{Binding {path}}}" />"#
            );
            assert!(compile_test_axaml(&axaml)
                .expect_err(path)
                .to_string()
                .contains("not a supported Rust property path"));
        }
        let conflicting = r#"
<StackPanel xmlns="https://github.com/avaloniaui">
  <TextBlock Text="{Binding User.Name}" />
  <TextBlock Text="{Binding UserName}" />
</StackPanel>"#;
        assert!(compile_test_axaml(conflicting)
            .expect_err("different model paths cannot share a Rust field")
            .to_string()
            .contains("conflict on Rust binding field"));
    }

    #[test]
    fn compiles_commands_and_preserves_string_parameters() {
        let axaml = r#"
<StackPanel xmlns="https://github.com/avaloniaui">
  <Button Command="{Binding SetPortsCommand}" CommandParameter="3389" />
  <Button Command="{Binding SetPortsCommand}" CommandParameter="{Binding Target.Value}" />
  <TextBlock Text="{Binding Target.Value}" />
</StackPanel>"#;
        let output = compile_test_axaml(axaml).expect("command bindings should compile");
        assert_eq!(output.matches("pub set_ports_command:").count(), 1);
        assert!(output
            .contains("pub set_ports_command: rustolonia::Signal<Option<rustolonia::Command>>"));
        assert!(output.contains("__control.set_command(command.as_ref().map"));
        assert!(output.contains(".command_parameter(\"3389\")"));
        assert!(output.contains(
            "pub target_value_command_parameter: rustolonia::PropertyPath<rustolonia::Variant>"
        ));
        assert!(output.contains("pub target_value: rustolonia::PropertyPath<String>"));
    }

    #[test]
    fn rejects_two_way_command_and_parameter_bindings() {
        for axaml in [
            r#"<Button xmlns="https://github.com/avaloniaui" Command="{Binding Save, Mode=TwoWay}" />"#,
            r#"<Button xmlns="https://github.com/avaloniaui" CommandParameter="{Binding Value, Mode=TwoWay}" />"#,
        ] {
            assert!(compile_test_axaml(axaml)
                .expect_err("commands and parameters are one-way")
                .to_string()
                .contains("one-way"));
        }
    }

    #[test]
    fn compiles_registered_static_and_resource_converters() {
        let axaml = r#"
<StackPanel xmlns="https://github.com/avaloniaui"
            xmlns:markup="http://schemas.microsoft.com/winfx/2006/xaml">
  <Border IsVisible="{Binding Notification.Content, Converter={markup:Static StringConverters.IsNotNullOrEmpty}}">
    <TextBlock Text="{Binding Notification.Content}" />
  </Border>
  <TextBox Text="{Binding Price, Mode=TwoWay, Converter={StaticResource DecimalConverter}, ConverterParameter='0.00,units', ConverterCulture=fr-FR}" />
</StackPanel>"#;
        let output = compile_test_axaml(axaml).expect("registered converters should compile");
        assert!(
            output.contains("pub notification_content_converter_source: rustolonia::ScalarPath")
        );
        assert!(output.contains("pub notification_content: rustolonia::PropertyPath<String>"));
        assert!(output.contains(
            "pub converter_static_string_converters_is_not_null_or_empty: rustolonia::RegisteredValueConverter"
        ));
        assert!(output.contains("rustolonia::ConvertedBinding::<bool>::new"));
        assert!(output.contains("\"x:Static StringConverters.IsNotNullOrEmpty\""));
        assert!(output.contains(
            "pub converter_resource_decimal_converter: rustolonia::RegisteredValueConverter"
        ));
        assert!(output.contains("rustolonia::ScalarValue::String(\"0.00,units\".to_string())"));
        assert!(output.contains("Some(\"fr-FR\")).two_way()"));
    }

    #[test]
    fn rejects_converter_clauses_that_cannot_be_preserved() {
        let cases = [
            (
                r#"<Border xmlns="https://github.com/avaloniaui" IsVisible="{Binding X, Converter={x:Static C.Foo}}" />"#,
                "not bound to the XAML namespace",
            ),
            (
                r#"<Border xmlns="https://github.com/avaloniaui" xmlns:x="clr-namespace:Wrong" IsVisible="{Binding X, Converter={x:Static C.Foo}}" />"#,
                "not bound to the XAML namespace",
            ),
            (
                r#"<Border xmlns="https://github.com/avaloniaui" IsVisible="{Binding X, ConverterParameter=42}" />"#,
                "require a Converter",
            ),
            (
                r#"<Border xmlns="https://github.com/avaloniaui" IsVisible="{Binding X, Converter={StaticResource C}, ConverterParameter={Binding Y}}" />"#,
                "only a literal string",
            ),
            (
                r#"<TextBlock xmlns="https://github.com/avaloniaui" Text="{Binding X, Converter={StaticResource C}, StringFormat='{}{0}'}" />"#,
                "StringFormat combined with Converter",
            ),
            (
                r#"<Border xmlns="https://github.com/avaloniaui" Margin="{Binding X, Converter={StaticResource C}}" />"#,
                "Converter binding to Rust type",
            ),
            (
                r#"<Button xmlns="https://github.com/avaloniaui" Command="{Binding X, Converter={StaticResource C}}" />"#,
                "only a one-way binding",
            ),
        ];
        for (axaml, diagnostic) in cases {
            let error = compile_test_axaml(axaml).expect_err(axaml);
            assert!(error.to_string().contains(diagnostic), "{error}");
        }
    }

    #[test]
    fn compiles_content_bindings_to_control_slots() {
        let content = r#"
<ContentControl xmlns="https://github.com/avaloniaui" Content="{Binding Child}" />
"#;
        let output = compile_test_axaml(content).expect("Content uses an explicit Rust view slot");
        assert!(output.contains("pub child: rustolonia::ControlSlot"));
        assert!(output.contains(".content(bindings.child.host())"));
    }

    #[test]
    fn distinct_content_bindings_create_independent_slots() {
        let axaml = r#"
<StackPanel xmlns="https://github.com/avaloniaui">
  <ContentControl Content="{Binding Header}" />
  <ContentControl Content="{Binding Details}" />
  <ContentControl Content="{Binding Footer}" />
</StackPanel>"#;
        let output = compile_test_axaml(axaml).expect("each model has its own view slot");
        for field in ["header", "details", "footer"] {
            assert!(output.contains(&format!("pub {field}: rustolonia::ControlSlot")));
            assert!(output.contains(&format!(".content(bindings.{field}.host())")));
        }

        let items = r#"
<ItemsControl xmlns="https://github.com/avaloniaui" ItemsSource="{Binding Items}" />
"#;
        let output = compile_test_axaml(items).expect("ItemsSource can use a Rust view registry");
        assert!(output.contains("pub items: rustolonia::ItemsView"));
        assert!(output.contains("let __items = bindings.items.clone()"));
        assert!(output.contains("__items.attach(scope, control)"));
        assert!(output.contains("#[derive(Clone, Debug)]"));
    }

    #[test]
    fn rejects_data_template_bindings_without_an_items_source() {
        let axaml = r#"
<ListBox xmlns="https://github.com/avaloniaui">
  <ListBox.ItemTemplate>
    <DataTemplate>
      <TextBlock Text="{Binding Name}" />
    </DataTemplate>
  </ListBox.ItemTemplate>
</ListBox>
"#;
        let error = compile_test_axaml(axaml)
            .expect_err("template item binding must not use outer context");
        assert!(error
            .to_string()
            .contains("DataTemplate names, bindings, and events require ItemsSource with an explicit Rust item contract"));
    }

    #[test]
    fn reports_markup_failures_with_source_lines() {
        let axaml = r#"<Window xmlns="https://github.com/avaloniaui">
  <TextBlock Text="{Binding Status, Converter=StatusConverter}" />
</Window>"#;
        let error = compile_test_axaml(axaml).expect_err("converter must be diagnosed");
        assert!(error.to_string().contains("line 2"));
    }

    #[test]
    fn rejects_non_avalonia_control_namespaces() {
        let axaml = r#"
<Window xmlns="https://github.com/avaloniaui" xmlns:custom="clr-namespace:Example">
  <custom:SpecialControl />
</Window>
"#;
        let error = compile_test_axaml(axaml).expect_err("custom CLR types need registration");
        assert!(error
            .to_string()
            .contains("Unsupported AXAML element namespace"));
    }

    #[test]
    fn rejects_unimplemented_markup_compatibility_attributes() {
        let axaml = r#"
<Window xmlns="https://github.com/avaloniaui"
        xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
        mc:MustUnderstand="custom">
</Window>
"#;
        let error = compile_test_axaml(axaml).expect_err("unknown mc behavior must not be ignored");
        assert!(error
            .to_string()
            .contains("Unsupported namespaced attribute 'MustUnderstand'"));
    }

    #[test]
    fn rejects_namespaced_attributes_that_resemble_builtin_properties() {
        for attribute in ["Name", "Classes"] {
            let axaml = format!(
                r#"<TextBlock xmlns="https://github.com/avaloniaui"
                    xmlns:custom="clr-namespace:Example" custom:{attribute}="value" />"#
            );
            let error = compile_test_axaml(&axaml).expect_err(attribute);
            assert!(
                error
                    .to_string()
                    .contains(&format!("Unsupported namespaced attribute '{attribute}'")),
                "{error}"
            );
        }
    }

    #[test]
    fn rejects_nested_xaml_metadata_that_would_change_binding_context() {
        for attribute in ["Class", "DataType", "CompileBindings"] {
            let axaml = format!(
                r#"<StackPanel xmlns="https://github.com/avaloniaui"
                    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
                    <TextBlock x:{attribute}="Example" />
                </StackPanel>"#
            );
            let error = compile_test_axaml(&axaml).expect_err(attribute);
            assert!(
                error
                    .to_string()
                    .contains("supported only on the root element"),
                "{error}"
            );
        }
    }

    #[test]
    fn rejects_reused_event_handlers_instead_of_emitting_moved_values() {
        let axaml = r#"
<StackPanel xmlns="https://github.com/avaloniaui">
  <Button Click="Handle">One</Button>
  <Button Click="Handle">Two</Button>
</StackPanel>"#;
        assert!(compile_test_axaml(axaml)
            .expect_err("a boxed handler cannot be moved into two controls")
            .to_string()
            .contains("used more than once"));
    }

    #[test]
    fn event_args_use_the_public_rustolonia_api() {
        let axaml = r#"
<ListBox xmlns="https://github.com/avaloniaui" SelectionChanged="OnSelection" />"#;
        let source = compile_test_axaml(axaml).expect("selection event");
        assert!(source.contains(
            "rustolonia::ComPtr<rustolonia::IAvnSelectingItemsControlSelectionChangedArgs>"
        ));
        assert!(!source.contains("rustolonia_sys::"));
    }

    #[test]
    fn two_way_bindings_require_a_matching_builder_and_change_event() {
        for axaml in [
            r#"<TextBlock xmlns="https://github.com/avaloniaui" Text="{Binding Name, Mode=TwoWay}" />"#,
            r#"<Button xmlns="https://github.com/avaloniaui" Grid.Row="{Binding Row, Mode=TwoWay}" />"#,
        ] {
            assert!(compile_test_axaml(axaml)
                .expect_err(axaml)
                .to_string()
                .contains("TwoWay binding"));
        }
    }

    #[test]
    fn editable_binding_modes_must_be_explicit() {
        let implicit = r#"<TextBox xmlns="https://github.com/avaloniaui" Text="{Binding Name}" />"#;
        assert!(compile_test_axaml(implicit)
            .expect_err("the projected builder cannot infer Avalonia's default binding mode")
            .to_string()
            .contains("requires an explicit Mode"));
        let explicit = implicit.replace("{Binding Name}", "{Binding Name, Mode=OneWay}");
        assert!(compile_test_axaml(&explicit).is_ok());

        let template = r#"
<ItemsControl xmlns="https://github.com/avaloniaui"
              xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
              ItemsSource="{Binding Items}">
  <ItemsControl.ItemTemplate>
    <DataTemplate x:DataType="crate::Item" x:Key="Id">
      <TextBox Text="{Binding Name}" />
    </DataTemplate>
  </ItemsControl.ItemTemplate>
</ItemsControl>"#;
        assert!(compile_test_axaml(template)
            .expect_err("item bindings must not silently assume one-way mode")
            .to_string()
            .contains("requires an explicit Mode=OneWay"));
        let explicit = template.replace("{Binding Name}", "{Binding Name, Mode=OneWay}");
        assert!(compile_test_axaml(&explicit).is_ok());

        let two_way = template.replace(
            r#"<TextBox Text="{Binding Name}" />"#,
            r#"<TextBox Text="{Binding Name, Mode=TwoWay}" />"#,
        );
        let source = compile_test_axaml(&two_way)
            .expect("an explicitly two-way template binding uses a signal-valued item field");
        assert!(source.contains(".text(__item_signal.get().name.clone().two_way())"));
    }

    #[test]
    fn understood_namespaces_are_not_ignored() {
        let axaml = r#"
<StackPanel xmlns="https://github.com/avaloniaui"
            xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
            xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
            mc:Ignorable="x">
  <TextBlock x:Name="title" />
</StackPanel>"#;
        let source = compile_test_axaml(axaml).expect("known namespaces are processed");
        assert!(source.contains("pub title: rustolonia::NodeRef<rustolonia::TextBlock>"));
        assert!(source.contains(".name(\"title\")"));
    }

    #[test]
    fn items_source_rejects_inline_items() {
        let axaml = r#"
<ItemsControl xmlns="https://github.com/avaloniaui" ItemsSource="{Binding Items}">
  <TextBlock Text="unexpected static item" />
</ItemsControl>"#;
        assert!(compile_test_axaml(axaml)
            .expect_err("static items conflict with the bound item source")
            .to_string()
            .contains("cannot be combined with inline items"));
    }

    #[test]
    fn duration_and_nullable_geometry_bindings_use_builder_types() {
        let axaml = r#"
<StackPanel xmlns="https://github.com/avaloniaui">
  <TextBox CaretBlinkInterval="{Binding Delay}" />
  <Popup PlacementRect="{Binding Area}" />
</StackPanel>"#;
        let source = compile_test_axaml(axaml).expect("typed projected properties");
        assert!(source.contains("pub delay: rustolonia::Signal<std::time::Duration>"));
        assert!(source.contains("pub area: rustolonia::Signal<Option<rustolonia::Rect>>"));
    }

    #[test]
    fn rejects_mixed_markup_text_instead_of_emitting_it_literally() {
        let axaml = r#"
<Window xmlns="https://github.com/avaloniaui">
  <TextBlock>Hello {Binding Name}</TextBlock>
</Window>
"#;
        let error = compile_test_axaml(axaml).expect_err("mixed text binding is unsupported");
        assert!(error
            .to_string()
            .contains("Mixed or escaped AXAML markup expression"));
    }

    #[test]
    fn rejects_rust_keyword_view_names() {
        let axaml = r#"
<Window xmlns="https://github.com/avaloniaui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        x:Class="Example.type" />
"#;
        let error = compile_test_axaml(axaml).expect_err("keyword cannot name a Rust type");
        assert!(error
            .to_string()
            .contains("does not produce a valid Rust type name"));
    }

    #[test]
    fn rejects_incompatible_binding_types_for_the_same_model_property() {
        let axaml = r#"
<Window xmlns="https://github.com/avaloniaui">
  <StackPanel>
    <TextBlock Text="{Binding Value}" />
    <TextBlock FontSize="{Binding Value}" />
  </StackPanel>
</Window>
"#;
        let error = compile_test_axaml(axaml).expect_err("one field cannot have two Rust types");
        assert!(error
            .to_string()
            .contains("used with incompatible Rust types"));
    }

    #[test]
    fn rejects_bindings_that_would_drop_or_misinterpret_clauses() {
        for (binding, diagnostic) in [
            ("{BindingStatus}", "Unsupported AXAML markup extension"),
            ("{Binding Name, Path=Other}", "specified more than once"),
            (
                "{Binding Name, Mode=OneWay, Mode=TwoWay}",
                "specified more than once",
            ),
            (
                "{Binding Name, Mode=TwoWay, StringFormat='{}{0}'}",
                "StringFormat is not supported with TwoWay",
            ),
            (
                "{Binding Name, StringFormat='{}{0:N2}'}",
                "requires exactly one {0}",
            ),
            (
                "{Binding Name, StringFormat='{}{0} {0}'}",
                "supports only one {0}",
            ),
        ] {
            let axaml =
                format!(r#"<TextBlock xmlns="https://github.com/avaloniaui" Text="{binding}" />"#);
            let error = compile_test_axaml(&axaml).expect_err(binding);
            assert!(error.to_string().contains(diagnostic), "{error}");
        }

        let axaml = r#"<TextBlock xmlns="https://github.com/avaloniaui"
            FontSize="{Binding Name, StringFormat='{}{0}'}" />"#;
        assert!(compile_test_axaml(axaml)
            .expect_err("formatted numeric property")
            .to_string()
            .contains("StringFormat requires a string property"));
    }

    #[test]
    fn rejects_template_metadata_that_would_otherwise_be_ignored() {
        let static_template = r#"
<ListBox xmlns="https://github.com/avaloniaui">
  <ListBox.ItemTemplate>
    <DataTemplate DataType="SomeModel"><TextBlock Text="item" /></DataTemplate>
  </ListBox.ItemTemplate>
</ListBox>"#;
        assert!(compile_test_axaml(static_template)
            .expect_err("static template metadata")
            .to_string()
            .contains("Static DataTemplate requires"));

        let foreign_template = r#"
<ListBox xmlns="https://github.com/avaloniaui"
         xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
         xmlns:other="clr-namespace:Example" ItemsSource="{Binding Items}">
  <ListBox.ItemTemplate>
    <other:DataTemplate x:DataType="crate::Item" x:Key="Id">
      <TextBlock Text="{Binding Name}" />
    </other:DataTemplate>
  </ListBox.ItemTemplate>
</ListBox>"#;
        assert!(compile_test_axaml(foreign_template)
            .expect_err("foreign DataTemplate")
            .to_string()
            .contains("ItemTemplate must contain exactly one DataTemplate"));

        let property_attribute = r#"
<ListBox xmlns="https://github.com/avaloniaui">
  <ListBox.ItemTemplate Unrecognized="value">
    <DataTemplate><TextBlock Text="item" /></DataTemplate>
  </ListBox.ItemTemplate>
</ListBox>"#;
        assert!(compile_test_axaml(property_attribute)
            .expect_err("property element attributes")
            .to_string()
            .contains("Attributes on property element"));
    }

    #[test]
    fn rejects_one_content_slot_attached_to_multiple_hosts() {
        let axaml = r#"
<StackPanel xmlns="https://github.com/avaloniaui">
  <ContentControl Content="{Binding Child}" />
  <ContentControl Content="{Binding Child}" />
</StackPanel>"#;
        assert!(compile_test_axaml(axaml)
            .expect_err("one ControlSlot cannot refer to two hosts")
            .to_string()
            .contains("each ControlSlot needs its own host"));
    }
}
