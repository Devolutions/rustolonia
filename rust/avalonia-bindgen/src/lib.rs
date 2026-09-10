mod abi_contract;
mod emit;
mod emit_safe;
mod error;
mod geometry;
mod ir;
mod owned;
mod validate;
mod variant;
pub use abi_contract::regenerate_baseline;
pub use error::GenerationError;
pub use ir::ProjectionIr;
pub use owned::{check_outputs, write_outputs, BINDGEN_GENERATOR_ID};

pub fn generate_from_ir(ir: &ProjectionIr) -> Result<String, GenerationError> {
    ir.validate()?;
    Ok(emit::emit_sys_module(ir))
}

pub fn generate_safe_from_ir(ir: &ProjectionIr) -> Result<String, GenerationError> {
    ir.validate()?;
    Ok(emit_safe::emit_safe_module(ir))
}

pub fn generate_from_json(json: &str) -> Result<String, GenerationError> {
    let ir: ProjectionIr = serde_json::from_str(json)?;
    generate_from_ir(&ir)
}

pub fn generate_safe_from_json(json: &str) -> Result<String, GenerationError> {
    let ir: ProjectionIr = serde_json::from_str(json)?;
    generate_safe_from_ir(&ir)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
    {
      "version": 1,
      "sourceAssembly": "Avalonia.Host",
      "types": [
        {
          "name": "IAvnEcho",
          "fullName": "Avalonia.Host.Com.IAvnEcho",
          "kind": "Interface",
          "iid": "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D11",
          "methods": [
            {
              "name": "Ping",
              "returnKind": "I32",
              "preserveSig": true,
              "parameters": [
                { "name": "value", "kind": "I32", "direction": "In" },
                { "name": "result", "kind": "I32", "direction": "Out" }
              ]
            },
            {
              "name": "EchoString",
              "returnKind": "I32",
              "preserveSig": true,
              "parameters": [
                { "name": "input", "kind": "StringUtf16", "direction": "In", "isNullable": true },
                { "name": "output", "kind": "StringUtf16", "direction": "Out", "isNullable": true }
              ]
            },
            {
              "name": "Fail",
              "returnKind": "I32",
              "preserveSig": true,
              "parameters": []
            }
          ]
        }
      ]
    }
    "#;

    #[test]
    fn rejects_unsupported_method_abi_before_emitting_either_surface() {
        for (from, to, message) in [
            (
                "\"direction\": \"In\"",
                "\"direction\": \"InOut\"",
                "InOut method",
            ),
            (
                "\"returnKind\": \"I32\"",
                "\"returnKind\": \"Void\"",
                "PreserveSig I32",
            ),
            (
                "\"preserveSig\": true",
                "\"preserveSig\": false",
                "PreserveSig I32",
            ),
        ] {
            let json = FIXTURE.replace(from, to);
            assert!(generate_from_json(&json)
                .unwrap_err()
                .to_string()
                .contains(message));
            assert!(generate_safe_from_json(&json)
                .unwrap_err()
                .to_string()
                .contains(message));
        }
    }

    #[test]
    fn generated_utf16_inputs_keep_terminated_buffers_alive() {
        let fixture = generate_from_json(FIXTURE).unwrap();
        assert!(fixture.contains("let input = input.map(crate::terminated_utf16);"));
        assert!(
            fixture.contains("input.as_ref().map_or(ptr::null_mut(), |v| v.as_ptr().cast_mut())")
        );
        let generated = generate_from_json(include_str!("../../projection.ir.json")).unwrap();
        for method in generated.split("    pub fn ").skip(1) {
            let Some((signature, body)) = method.split_once(" {\n") else {
                continue;
            };
            if !signature.contains("&[u16]") {
                continue;
            }
            assert!(
                body.contains("crate::terminated_utf16"),
                "Missing terminated input buffer for {signature}"
            );
            assert!(
                !body.contains(".map_or(ptr::null_mut(), |v| v.as_ptr().cast_mut())")
                    || body
                        .contains(".as_ref().map_or(ptr::null_mut(), |v| v.as_ptr().cast_mut())"),
                "Nullable buffer must stay borrowed during {signature}"
            );
        }
    }

    #[test]
    fn emits_fixture_echo_surface() {
        let src = generate_from_json(FIXTURE).unwrap();
        assert!(src.contains("pub struct IAvnEcho"));
        assert!(src.contains("data1: 0x6B2E8F10"));
        assert!(src.contains("pub fn ping"));
        assert!(src.contains("pub fn echo_string"));
        assert!(src.contains("pub fn fail"));
        assert!(src.contains("unsafe impl ComInterface for IAvnEcho"));
        assert!(!src.contains("unsupported"));
    }

    #[test]
    fn checked_in_sys_bindings_match_shared_ir() {
        let ir = include_str!("../../projection.ir.json");
        let expected = include_str!("../../avalonia-sys/src/generated.rs");
        assert_eq!(generate_from_json(ir).unwrap(), expected);
    }

    #[test]
    fn checked_in_safe_bindings_match_shared_ir() {
        let ir = include_str!("../../projection.ir.json");
        let expected = include_str!("../../avalonia/src/generated.rs");
        assert_eq!(generate_safe_from_json(ir).unwrap(), expected);
    }

    #[test]
    fn factory_slots_are_sorted_by_interface_name() {
        let ir: ProjectionIr =
            serde_json::from_str(include_str!("../../projection.ir.json")).unwrap();
        let generated = generate_from_ir(&ir).unwrap();
        let mut slots: Vec<_> = ir
            .types
            .iter()
            .filter(|ty| ty.is_constructible)
            .map(|ty| {
                let suffix = ty.name.strip_prefix("IAvn").unwrap();
                (
                    &ty.full_name,
                    generated
                        .find(&format!("    create_{}:", to_snake(suffix)))
                        .unwrap(),
                )
            })
            .collect();
        slots.sort_by_key(|(_, position)| *position);

        assert!(slots.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn checked_in_native_header_matches_rust_vtable_slots() {
        let ir: ProjectionIr =
            serde_json::from_str(include_str!("../../projection.ir.json")).unwrap();
        let rust = include_str!("../../avalonia-sys/src/generated.rs");
        let header = include_str!("../../avalonia-sys/include/avalonia-rust-abi.h");
        let mut names: Vec<String> = ir
            .types
            .iter()
            .filter(|ty| ty.kind == "Class" || ty.kind == "Interface")
            .map(|ty| ty.name.clone())
            .collect();

        for event in ir.types.iter().flat_map(|ty| ty.events.iter()) {
            push_unique(&mut names, simple_name(&event.handler_interface_name));
        }
        for property in ir
            .types
            .iter()
            .flat_map(|ty| ty.properties.iter())
            .filter(|property| property.kind == "ComCollection")
        {
            push_unique(
                &mut names,
                simple_name(property.interface_name.as_deref().unwrap()),
            );
        }
        for property in &ir.attached_properties {
            push_unique(&mut names, simple_name(&property.statics_interface_name));
        }
        push_unique(&mut names, "IAvnControlFactory");

        for name in names {
            assert_eq!(
                rust_vtable_slots(rust, &name),
                header_vtable_slots(header, &name),
                "vtable drift for {name}"
            );
        }
    }

    #[test]
    fn checked_in_ir_validates() {
        let ir: ProjectionIr =
            serde_json::from_str(include_str!("../../projection.ir.json")).unwrap();
        ir.validate().unwrap();
    }

    #[test]
    fn version_one_void_return_is_rejected() {
        let src = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnVoid",
                  "fullName": "Tests.IAvnVoid",
                  "kind": "Interface",
                  "methods": [
                    { "name": "Noop", "returnKind": "Void", "parameters": [] }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap_err();
        assert!(src.to_string().contains("Only PreserveSig I32 HRESULT"));
    }

    #[test]
    fn future_schema_version_is_rejected_by_public_entrypoint() {
        let error = generate_from_json(r#"{ "version": 17, "types": [] }"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Unsupported projection IR version 17"));
        assert!(error.contains("1..=16"));
    }

    #[test]
    fn invalid_schema_version_is_rejected_by_public_entrypoint() {
        let error = generate_from_json(r#"{ "version": 0, "types": [] }"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Unsupported projection IR version 0"));
    }

    #[test]
    fn unknown_marshalling_kind_is_rejected_instead_of_c_void_fallback() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnBad",
                  "fullName": "Tests.IAvnBad",
                  "kind": "Interface",
                  "methods": [
                    { "name": "Ping", "returnKind": "FutureKind", "parameters": [] }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Unknown marshalling kind 'FutureKind'"));
        assert!(error.contains("type 'Tests.IAvnBad' method 'Ping'"));
        assert!(!error.to_lowercase().contains("c_void"));
    }

    #[test]
    fn unknown_parameter_direction_is_rejected_by_public_entrypoint() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnBad",
                  "fullName": "Tests.IAvnBad",
                  "kind": "Interface",
                  "methods": [
                    {
                      "name": "Ping",
                      "returnKind": "Void",
                      "parameters": [
                        { "name": "value", "kind": "I32", "direction": "Sideways" }
                      ]
                    }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Unknown parameter direction 'Sideways'"));
        assert!(error.contains("parameter 'value'"));
    }

    #[test]
    fn missing_base_reference_is_rejected_by_public_entrypoint() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnChild",
                  "fullName": "Tests.IAvnChild",
                  "kind": "Interface",
                  "baseFullName": "Tests.IAvnMissing"
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("Type 'Tests.IAvnChild' references missing base 'Tests.IAvnMissing'")
        );
    }

    #[test]
    fn self_base_reference_is_rejected_by_public_entrypoint() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnSelf",
                  "fullName": "Tests.IAvnSelf",
                  "kind": "Interface",
                  "baseFullName": "Tests.IAvnSelf"
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Type 'Tests.IAvnSelf' has a self base reference."));
    }

    #[test]
    fn inheritance_cycle_is_rejected_by_public_entrypoint() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "baseFullName": "Tests.IAvnB"
                },
                {
                  "name": "IAvnB",
                  "fullName": "Tests.IAvnB",
                  "kind": "Interface",
                  "baseFullName": "Tests.IAvnA"
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Inheritance cycle: Tests.IAvnA -> Tests.IAvnB -> Tests.IAvnA"));
    }

    #[test]
    fn unknown_type_kind_is_rejected_by_public_entrypoint() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnBad",
                  "fullName": "Tests.IAvnBad",
                  "kind": "Trait"
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Unknown type kind 'Trait' at type 'Tests.IAvnBad'"));
    }

    #[test]
    fn generate_safe_from_json_uses_the_same_validation() {
        let error = generate_safe_from_json(r#"{ "version": 99, "types": [] }"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Unsupported projection IR version 99"));
    }

    #[test]
    fn generate_from_ir_rejects_future_version() {
        let ir: ProjectionIr = serde_json::from_str(r#"{ "version": 17, "types": [] }"#).unwrap();
        let error = generate_from_ir(&ir).unwrap_err().to_string();
        assert!(error.contains("Unsupported projection IR version 17"));
    }

    #[test]
    fn generate_safe_from_ir_rejects_inheritance_cycle() {
        let ir: ProjectionIr = serde_json::from_str(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "baseFullName": "Tests.IAvnB"
                },
                {
                  "name": "IAvnB",
                  "fullName": "Tests.IAvnB",
                  "kind": "Interface",
                  "baseFullName": "Tests.IAvnA"
                }
              ]
            }
            "#,
        )
        .unwrap();
        let error = generate_safe_from_ir(&ir).unwrap_err().to_string();
        assert!(error.contains("Inheritance cycle: Tests.IAvnA -> Tests.IAvnB -> Tests.IAvnA"));
    }

    #[test]
    fn empty_base_full_name_is_rejected_by_public_entrypoint() {
        let error = generate_from_json(
            r#"{"version":1,"types":[{"name":"IAvnA","fullName":"Tests.IAvnA","kind":"Interface","baseFullName":""}]}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Type 'Tests.IAvnA' has an empty baseFullName."));
    }

    #[test]
    fn whitespace_base_full_name_is_rejected_by_public_entrypoint() {
        let error = generate_from_json(
            r#"{"version":1,"types":[{"name":"IAvnA","fullName":"Tests.IAvnA","kind":"Interface","baseFullName":"   "}]}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Type 'Tests.IAvnA' has an empty baseFullName."));
    }

    #[test]
    fn collection_i32_element_kind_is_rejected_before_emission() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "properties": [
                    {
                      "name": "Items",
                      "kind": "ComCollection",
                      "interfaceName": "Tests.IAvnList",
                      "interfaceIid": "00000000-0000-0000-0000-000000000001",
                      "elementKind": "I32"
                    }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Unsupported collection element kind 'I32'"));
        assert!(error.contains("type 'Tests.IAvnA' property 'Items'"));
    }

    #[test]
    fn collection_missing_interface_name_is_rejected_before_emission() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "properties": [
                    {
                      "name": "Items",
                      "kind": "ComCollection",
                      "elementKind": "Variant"
                    }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Missing interfaceName at type 'Tests.IAvnA' property 'Items'."));
    }

    #[test]
    fn collection_missing_element_kind_is_rejected_before_emission() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "properties": [
                    {
                      "name": "Items",
                      "kind": "ComCollection",
                      "interfaceName": "Tests.IAvnList",
                      "interfaceIid": "00000000-0000-0000-0000-000000000001"
                    }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Missing elementKind at type 'Tests.IAvnA' property 'Items'."));
    }

    #[test]
    fn collection_com_interface_missing_element_interface_is_rejected() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "properties": [
                    {
                      "name": "Items",
                      "kind": "ComCollection",
                      "interfaceName": "Tests.IAvnList",
                      "interfaceIid": "00000000-0000-0000-0000-000000000001",
                      "elementKind": "ComInterface"
                    }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("Missing elementInterfaceName at type 'Tests.IAvnA' property 'Items'.")
        );
    }

    #[test]
    fn fields_payload_without_parameters_is_rejected_before_emission() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "events": [
                    {
                      "name": "Changed",
                      "handlerInterfaceName": "Tests.IAvnChangedHandler",
                      "handlerInterfaceIid": "00000000-0000-0000-0000-000000000002",
                      "payloadKind": "Fields",
                      "parameters": []
                    }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(error
            .contains("Fields payload requires parameters at type 'Tests.IAvnA' event 'Changed'."));
    }

    #[test]
    fn args_payload_without_interface_name_is_rejected_before_emission() {
        let error = generate_from_json(
            r#"
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "events": [
                    {
                      "name": "Changed",
                      "handlerInterfaceName": "Tests.IAvnChangedHandler",
                      "handlerInterfaceIid": "00000000-0000-0000-0000-000000000002",
                      "payloadKind": "Args"
                    }
                  ]
                }
              ]
            }
            "#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("Missing argsInterfaceName at type 'Tests.IAvnA' event 'Changed'."));
    }

    fn to_snake(value: &str) -> String {
        let mut output = String::new();
        for (index, character) in value.chars().enumerate() {
            if character.is_uppercase() && index > 0 {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        }
        output
    }

    fn simple_name(value: &str) -> &str {
        value.rsplit('.').next().unwrap()
    }

    fn push_unique(values: &mut Vec<String>, value: &str) {
        if !values.iter().any(|existing| existing == value) {
            values.push(value.to_owned());
        }
    }

    fn rust_vtable_slots<'a>(source: &'a str, name: &str) -> Vec<&'a str> {
        let marker = format!("struct {name}Vtbl {{");
        let section = source
            .split_once(&marker)
            .unwrap_or_else(|| panic!("missing Rust vtable {name}"))
            .1
            .split_once("\n}")
            .unwrap()
            .0;
        section
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let (field, _) = line.split_once(':')?;
                (!field.contains(' ')).then_some(field)
            })
            .collect()
    }

    fn header_vtable_slots<'a>(source: &'a str, name: &str) -> Vec<&'a str> {
        let marker = format!("struct {name}Vtbl {{");
        let section = source
            .split_once(&marker)
            .unwrap_or_else(|| panic!("missing native vtable {name}"))
            .1
            .split_once("\n};")
            .unwrap()
            .0;
        section
            .lines()
            .filter_map(|line| {
                let (_, after_pointer) = line.split_once('*')?;
                after_pointer.split_once(')').map(|(field, _)| field)
            })
            .collect()
    }
}
