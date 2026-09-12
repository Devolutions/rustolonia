use crate::error::GenerationError;
use crate::ir::{
    ProjectedAttachedProperty, ProjectedEvent, ProjectedMethod, ProjectedParameter, ProjectedType,
    ProjectionIr,
};

pub const MINIMUM_VERSION: i32 = 1;
pub const CURRENT_VERSION: i32 = 17;

const SUPPORTED_MARSHALLING_KINDS: &[&str] = &[
    "Void",
    "I32",
    "I64",
    "F32",
    "F64",
    "Bool",
    "NullableBool",
    "StringUtf16",
    "ComInterface",
    "ComCollection",
    "Thickness",
    "CornerRadius",
    "Size",
    "Point",
    "Rect",
    "Color",
    "Brush",
    "Vector",
    "CharUtf16",
    "Command",
    "Variant",
    "TimeSpanI64",
    "DateTimeI64",
    "PixelPointI32",
    "DataTemplate",
    "ItemFilter",
    "TextFilter",
    "Notification",
    "ItemSelector",
    "TextSelector",
    "PopupPlacement",
    "AsyncPopulator",
    "DialogCompletion",
];

const SUPPORTED_DIRECTIONS: &[&str] = &["In", "Out", "InOut"];
const SUPPORTED_TYPE_KINDS: &[&str] = &["Interface", "Class", "Struct", "Enum"];
const SUPPORTED_PAYLOAD_KINDS: &[&str] = &["None", "Fields", "Args"];
const SUPPORTED_COLLECTION_ELEMENT_KINDS: &[&str] =
    &["ComInterface", "StringUtf16", "Variant", "DateTimeI64"];

pub fn validate(ir: &ProjectionIr) -> Result<(), GenerationError> {
    if ir.version < MINIMUM_VERSION || ir.version > CURRENT_VERSION {
        return Err(GenerationError::invalid(format!(
            "Unsupported projection IR version {} (supported {MINIMUM_VERSION}..={CURRENT_VERSION}).",
            ir.version
        )));
    }

    let mut types_by_full_name = std::collections::HashMap::new();
    for ty in &ir.types {
        if ty.full_name.trim().is_empty() {
            return Err(GenerationError::invalid(
                "Projected type is missing fullName.",
            ));
        }
        if types_by_full_name
            .insert(ty.full_name.as_str(), ty)
            .is_some()
        {
            return Err(GenerationError::invalid(format!(
                "Duplicate projected type '{}'.",
                ty.full_name
            )));
        }
        validate_named(
            &ty.kind,
            SUPPORTED_TYPE_KINDS,
            &format!(
                "Unknown type kind '{}' at type '{}'.",
                ty.kind, ty.full_name
            ),
        )?;
        for method in &ty.methods {
            validate_method(ty, method)?;
        }
        for property in &ty.properties {
            validate_property(ty, property)?;
        }
        for event in &ty.events {
            validate_event(ty, event)?;
        }
    }

    for attached in &ir.attached_properties {
        validate_attached(attached)?;
    }

    for ty in &ir.types {
        validate_inheritance(ty, &types_by_full_name)?;
    }

    Ok(())
}

fn validate_method(ty: &ProjectedType, method: &ProjectedMethod) -> Result<(), GenerationError> {
    let location = format!("type '{}' method '{}'", ty.full_name, method.name);
    validate_kind(&method.return_kind, &location)?;
    for parameter in &method.parameters {
        validate_parameter(parameter, &location)?;
        if parameter.direction == "InOut" {
            return Err(GenerationError::invalid(format!(
                "InOut method parameters are not supported at {location} parameter '{}'.",
                parameter.name
            )));
        }
    }
    if method.return_kind != "I32" || !method.preserve_sig {
        return Err(GenerationError::invalid(format!(
            "Only PreserveSig I32 HRESULT returns are supported at {location}."
        )));
    }
    Ok(())
}

fn validate_property(
    ty: &ProjectedType,
    property: &crate::ir::ProjectedProperty,
) -> Result<(), GenerationError> {
    let location = format!("type '{}' property '{}'", ty.full_name, property.name);
    validate_kind(&property.kind, &location)?;
    if property.kind != "ComCollection" {
        if let Some(element_kind) = &property.element_kind {
            validate_kind(element_kind, &format!("{location} element"))?;
        }
        return Ok(());
    }

    require_present(
        property.interface_name.as_deref(),
        "interfaceName",
        &location,
    )?;
    require_present(property.interface_iid.as_deref(), "interfaceIid", &location)?;
    let Some(element_kind) = property
        .element_kind
        .as_deref()
        .filter(|kind| !kind.trim().is_empty())
    else {
        return Err(GenerationError::invalid(format!(
            "Missing elementKind at {location}."
        )));
    };
    if !SUPPORTED_COLLECTION_ELEMENT_KINDS.contains(&element_kind) {
        return Err(GenerationError::invalid(format!(
            "Unsupported collection element kind '{element_kind}' at {location}."
        )));
    }
    if element_kind == "ComInterface" {
        require_present(
            property.element_interface_name.as_deref(),
            "elementInterfaceName",
            &location,
        )?;
    }
    Ok(())
}

fn validate_event(ty: &ProjectedType, event: &ProjectedEvent) -> Result<(), GenerationError> {
    let location = format!("type '{}' event '{}'", ty.full_name, event.name);
    validate_named(
        &event.payload_kind,
        SUPPORTED_PAYLOAD_KINDS,
        &format!(
            "Unknown event payload kind '{}' at {location}.",
            event.payload_kind
        ),
    )?;
    for parameter in &event.parameters {
        validate_parameter(parameter, &location)?;
    }
    match event.payload_kind.as_str() {
        "None" if !event.parameters.is_empty() => {
            return Err(GenerationError::invalid(format!(
                "None payload cannot have parameters at {location}."
            )));
        }
        "Fields" if event.parameters.is_empty() => {
            return Err(GenerationError::invalid(format!(
                "Fields payload requires parameters at {location}."
            )));
        }
        "Args" => {
            require_present(
                event.args_interface_name.as_deref(),
                "argsInterfaceName",
                &location,
            )?;
            require_present(
                event.args_interface_iid.as_deref(),
                "argsInterfaceIid",
                &location,
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_attached(attached: &ProjectedAttachedProperty) -> Result<(), GenerationError> {
    validate_kind(
        &attached.kind,
        &format!(
            "attached property '{}.{}'",
            attached.owner_name, attached.name
        ),
    )
}

fn validate_parameter(
    parameter: &ProjectedParameter,
    owner_location: &str,
) -> Result<(), GenerationError> {
    let location = format!("{owner_location} parameter '{}'", parameter.name);
    validate_kind(&parameter.kind, &location)?;
    validate_named(
        &parameter.direction,
        SUPPORTED_DIRECTIONS,
        &format!(
            "Unknown parameter direction '{}' at {location}.",
            parameter.direction
        ),
    )
}

fn validate_kind(kind: &str, location: &str) -> Result<(), GenerationError> {
    validate_named(
        kind,
        SUPPORTED_MARSHALLING_KINDS,
        &format!("Unknown marshalling kind '{kind}' at {location}."),
    )
}

fn validate_named(value: &str, allowed: &[&str], message: &str) -> Result<(), GenerationError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(GenerationError::invalid(message))
    }
}

fn require_present(
    value: Option<&str>,
    field: &str,
    location: &str,
) -> Result<(), GenerationError> {
    if value
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .is_some()
    {
        Ok(())
    } else {
        Err(GenerationError::invalid(format!(
            "Missing {field} at {location}."
        )))
    }
}

fn validate_inheritance(
    ty: &ProjectedType,
    types_by_full_name: &std::collections::HashMap<&str, &ProjectedType>,
) -> Result<(), GenerationError> {
    match ty.base_full_name.as_deref() {
        None => return Ok(()),
        Some(base) if base.trim().is_empty() => {
            return Err(GenerationError::invalid(format!(
                "Type '{}' has an empty baseFullName.",
                ty.full_name
            )));
        }
        Some(_) => {}
    }

    if ty.base_full_name.as_deref() == Some(ty.full_name.as_str()) {
        return Err(GenerationError::invalid(format!(
            "Type '{}' has a self base reference.",
            ty.full_name
        )));
    }

    let mut seen = vec![ty.full_name.as_str()];
    let mut current = ty;
    while let Some(base_name) = current.base_full_name.as_deref() {
        if base_name.trim().is_empty() {
            return Err(GenerationError::invalid(format!(
                "Type '{}' has an empty baseFullName.",
                current.full_name
            )));
        }
        let Some(base_type) = types_by_full_name.get(base_name) else {
            return Err(GenerationError::invalid(format!(
                "Type '{}' references missing base '{base_name}'.",
                ty.full_name
            )));
        };
        if seen.contains(&base_type.full_name.as_str()) {
            seen.push(base_type.full_name.as_str());
            return Err(GenerationError::invalid(format!(
                "Inheritance cycle: {}.",
                seen.join(" -> ")
            )));
        }
        seen.push(base_type.full_name.as_str());
        current = base_type;
    }

    Ok(())
}
