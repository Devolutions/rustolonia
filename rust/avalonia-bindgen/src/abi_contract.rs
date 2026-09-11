use crate::ir::ProjectionIr;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AbiBaseline {
    pub schema_version: i32,
    pub kind: String,
    pub projection_ir_version: i32,
    pub producer_pin: String,
    pub abi_definitions: AbiDefinitions,
    #[serde(default)]
    pub retired_identities: Vec<RetiredIdentity>,
    #[serde(default)]
    pub value_types: Vec<AbiValueType>,
    #[serde(default)]
    pub slot_catalog: Vec<AbiSlot>,
    #[serde(default)]
    pub ir_member_catalog: Vec<String>,
    pub interfaces: Vec<AbiInterface>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AbiDefinitions {
    pub hresult: String,
    pub call_win32: String,
    pub call_other: String,
    pub struct_packing: String,
    pub pointer_model: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetiredIdentity {
    pub iid: String,
    pub name: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AbiValueType {
    pub name: String,
    pub fields: Vec<AbiField>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AbiField {
    #[serde(rename = "type")]
    pub type_name: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AbiInterface {
    pub name: String,
    pub iid: String,
    pub iid_constant: String,
    pub abi_version: i32,
    pub slots: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ir_bases: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ir_members: Vec<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AbiSlot {
    pub name: String,
    pub calling_convention: String,
    pub return_type: String,
    pub parameters: Vec<AbiParameter>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AbiParameter {
    #[serde(rename = "type")]
    pub type_name: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbiInventory {
    pub definitions: AbiDefinitions,
    pub value_types: Vec<AbiValueType>,
    pub slot_catalog: Vec<AbiSlot>,
    pub ir_member_catalog: Vec<String>,
    pub interfaces: Vec<AbiInterface>,
}

pub fn parse_header(header: &str) -> Result<AbiInventory, String> {
    if header.contains("#pragma pack") {
        return Err(
            "ABI header contains #pragma pack; packing must be recorded explicitly.".into(),
        );
    }
    let definitions = parse_definitions(header)?;
    let value_types = parse_value_types(header)?;
    let mut intern = SlotInterner::default();
    let mut interfaces = Vec::new();
    let mut seen_iids = HashSet::new();
    let mut seen_consts = HashSet::new();
    let mut rest = header;
    while let Some(start) = rest.find("static const AvnGuid ") {
        rest = &rest[start + "static const AvnGuid ".len()..];
        let Some(iid_marker) = rest.find("_IID") else {
            return Err("ABI header IID constant is missing a _IID suffix.".into());
        };
        let constant = format!("{}IID", &rest[..iid_marker + 1]);
        let Some(brace) = rest.find('{') else {
            return Err(format!("IID constant {constant} is missing a GUID body."));
        };
        let Some(end_guid) = rest.find("};") else {
            return Err(format!("IID constant {constant} is unterminated."));
        };
        if brace > end_guid {
            return Err(format!(
                "IID constant {constant} has a malformed GUID body."
            ));
        }
        let iid = parse_guid_block(&rest[brace + 1..end_guid])?;
        if !seen_iids.insert(iid.to_ascii_uppercase()) {
            return Err(format!("Duplicate IID {iid} in ABI header."));
        }
        if !seen_consts.insert(constant.clone()) {
            return Err(format!("Duplicate IID constant {constant} in ABI header."));
        }
        rest = &rest[end_guid + 2..];
        let expected_version =
            format!("#define {}_ABI_VERSION ", constant.trim_end_matches("_IID"));
        let trimmed = rest.trim_start();
        if !trimmed.starts_with(&expected_version) {
            return Err(format!(
                "IID constant {constant} is not followed by {expected_version}."
            ));
        }
        rest = trimmed[expected_version.len()..].trim_start();
        let version_end = rest
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(rest.len());
        let abi_version: i32 = rest[..version_end]
            .parse()
            .map_err(|_| format!("{constant} has a non-integer ABI version."))?;
        rest = rest[version_end..].trim_start();
        if !rest.starts_with("struct ") {
            return Err(format!("{constant} is not followed by a vtable struct."));
        }
        rest = &rest["struct ".len()..];
        let Some(vtbl) = rest.find("Vtbl {") else {
            return Err(format!("{constant} is missing a Vtbl body."));
        };
        let name = rest[..vtbl].trim().to_string();
        let expected_constant = iid_constant_name(&name)?;
        if constant != expected_constant {
            return Err(format!(
                "IID constant {constant} does not belong to vtable {name} (expected {expected_constant})."
            ));
        }
        rest = &rest[vtbl + "Vtbl {".len()..];
        let Some(body_end) = rest.find("\n};") else {
            return Err(format!("Interface {name} has an unterminated vtable."));
        };
        let slots = parse_slots(&name, &rest[..body_end], &mut intern)?;
        rest = &rest[body_end + 3..];
        let _ = iid_marker;
        interfaces.push(AbiInterface {
            name,
            iid,
            iid_constant: constant,
            abi_version,
            slots,
            ir_bases: Vec::new(),
            ir_members: Vec::new(),
        });
    }
    interfaces.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(AbiInventory {
        definitions,
        value_types,
        slot_catalog: intern.slots,
        ir_member_catalog: Vec::new(),
        interfaces,
    })
}

pub fn attach_ir(inventory: &mut AbiInventory, ir: &ProjectionIr) -> Result<(), String> {
    ir.validate().map_err(|error| error.to_string())?;
    let by_name: HashMap<_, _> = ir.types.iter().map(|ty| (ty.name.as_str(), ty)).collect();
    let by_full: HashMap<_, _> = ir
        .types
        .iter()
        .map(|ty| (ty.full_name.as_str(), ty))
        .collect();
    let mut intern = MemberInterner::default();
    for interface in &mut inventory.interfaces {
        let Some(ty) = by_name.get(interface.name.as_str()) else {
            continue;
        };
        let (bases, members) = flatten_lineage(ty, &by_full, &mut intern)?;
        interface.ir_bases = bases;
        interface.ir_members = members;
    }
    inventory.ir_member_catalog = intern.members;
    Ok(())
}

/// Rebuilds the frozen released-ABI snapshot from the current native header and
/// projection IR. The previous snapshot supplies the producer pin and the retired
/// identity list; both carry forward unchanged. This is the intentional, reviewed
/// way to advance the ABI: change the projection, regenerate, then rewrite the
/// baseline so `identity_breaks` compares future builds against the new release.
pub fn regenerate_baseline(ir_json: &str, header: &str, existing: &str) -> Result<String, String> {
    let ir: ProjectionIr = serde_json::from_str(ir_json)
        .map_err(|error| format!("Failed to parse projection IR: {error}"))?;
    let previous: AbiBaseline = serde_json::from_str(existing)
        .map_err(|error| format!("Failed to parse existing ABI baseline: {error}"))?;
    if previous.kind != "released" {
        return Err(format!(
            "Existing ABI snapshot kind must be 'released', found '{}'.",
            previous.kind
        ));
    }
    let mut inventory = parse_header(header)?;
    attach_ir(&mut inventory, &ir)?;
    let baseline = AbiBaseline {
        schema_version: previous.schema_version,
        kind: previous.kind,
        projection_ir_version: ir.version,
        producer_pin: previous.producer_pin,
        abi_definitions: inventory.definitions,
        retired_identities: previous.retired_identities,
        value_types: inventory.value_types,
        slot_catalog: inventory.slot_catalog,
        ir_member_catalog: inventory.ir_member_catalog,
        interfaces: inventory.interfaces,
    };
    let mut json = serde_json::to_string_pretty(&baseline)
        .map_err(|error| format!("Failed to serialize ABI baseline: {error}"))?;
    json.push('\n');
    Ok(json)
}

pub fn identity_breaks(
    released: &AbiBaseline,
    current: &AbiInventory,
) -> Result<Vec<String>, String> {
    if released.kind != "released" {
        return Err(format!(
            "Frozen ABI snapshot kind must be 'released', found '{}'.",
            released.kind
        ));
    }
    if released.abi_definitions != current.definitions {
        return Err(
            "Released ABI typedef/macro/packing definitions changed under the frozen snapshot."
                .into(),
        );
    }
    let current_by_iid = index_unique_iids(&current.interfaces)?;
    let released_by_iid = index_unique_iids(&released.interfaces)?;
    let retired: HashSet<_> = released
        .retired_identities
        .iter()
        .map(|retired| retired.iid.to_ascii_uppercase())
        .collect();
    for iid in &retired {
        if let Some(reused) = current_by_iid.get(iid) {
            return Err(format!(
                "Retired IID {} ({}) was reused by {}.",
                reused.iid, reused.name, reused.name
            ));
        }
        if released_by_iid.contains_key(iid) {
            return Err(format!(
                "Retired IID {iid} is still listed as a released identity."
            ));
        }
    }

    let mut breaks = Vec::new();
    let current_values: BTreeMap<_, _> = current
        .value_types
        .iter()
        .map(|value| (value.name.as_str(), value))
        .collect();
    if current_values.len() != current.value_types.len() {
        return Err("Duplicate value-type records in the current ABI inventory.".into());
    }
    for published in &released.value_types {
        match current_values.get(published.name.as_str()) {
            None => breaks.push(format!(
                "Released value type {} is missing from the current ABI.",
                published.name
            )),
            Some(actual) if *actual != published => breaks.push(format!(
                "Released value type {} changed layout under an unchanged ABI generation.",
                published.name
            )),
            Some(_) => {}
        }
    }

    for published in &released.interfaces {
        let key = published.iid.to_ascii_uppercase();
        let Some(actual) = current_by_iid.get(&key) else {
            breaks.push(format!(
                "Published IID {} ({}) is missing from the current ABI.",
                published.iid, published.name
            ));
            continue;
        };
        if actual.name != published.name || actual.iid_constant != published.iid_constant {
            breaks.push(format!(
                "IID {} changed name/constant from {}/{} to {}/{}.",
                published.iid,
                published.name,
                published.iid_constant,
                actual.name,
                actual.iid_constant
            ));
        }
        if actual.abi_version != published.abi_version {
            breaks.push(format!(
                "IID {} ({}) changed abiVersion from {} to {}.",
                published.iid, published.name, published.abi_version, actual.abi_version
            ));
        }
        let published_slots = expand_slots(&released.slot_catalog, &published.slots)?;
        let actual_slots = expand_slots(&current.slot_catalog, &actual.slots)?;
        if published_slots != actual_slots {
            breaks.push(format!(
                "IID {} ({}) changed callable signature or calling-convention binding under an unchanged identity.",
                published.iid, published.name
            ));
        }
        let published_members = expand_members(&released.ir_member_catalog, &published.ir_members)?;
        let actual_members = expand_members(&current.ir_member_catalog, &actual.ir_members)?;
        if published.ir_bases != actual.ir_bases || published_members != actual_members {
            breaks.push(format!(
                "IID {} ({}) changed flattened IR lineage semantics (kind/direction/nullability) under an unchanged identity.",
                published.iid, published.name
            ));
        }
    }
    Ok(breaks)
}

fn expand_slots<'a>(catalog: &'a [AbiSlot], indices: &[usize]) -> Result<Vec<&'a AbiSlot>, String> {
    indices
        .iter()
        .map(|index| {
            catalog
                .get(*index)
                .ok_or_else(|| format!("Slot catalog is missing index {index}."))
        })
        .collect()
}

fn expand_members(catalog: &[String], indices: &[usize]) -> Result<Vec<String>, String> {
    indices
        .iter()
        .map(|index| {
            catalog
                .get(*index)
                .cloned()
                .ok_or_else(|| format!("IR member catalog is missing index {index}."))
        })
        .collect()
}

fn flatten_lineage(
    ty: &crate::ir::ProjectedType,
    by_full: &HashMap<&str, &crate::ir::ProjectedType>,
    intern: &mut MemberInterner,
) -> Result<(Vec<String>, Vec<usize>), String> {
    let mut chain = Vec::new();
    let mut current = Some(ty);
    let mut seen = HashSet::new();
    while let Some(item) = current {
        if !seen.insert(item.full_name.as_str()) {
            return Err(format!(
                "IR lineage for {} contains a cycle at {}.",
                ty.name, item.full_name
            ));
        }
        chain.push(item);
        current = item
            .base_full_name
            .as_deref()
            .filter(|name| !name.is_empty())
            .and_then(|name| by_full.get(name).copied());
    }
    chain.reverse();
    let bases = chain.iter().map(|item| item.name.clone()).collect();
    let mut members = Vec::new();
    for item in chain {
        for method in &item.methods {
            let params = method
                .parameters
                .iter()
                .map(|parameter| {
                    format!(
                        "{}:{}:{}:{}",
                        parameter.name,
                        parameter.kind,
                        parameter.direction,
                        u8::from(parameter.is_nullable)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            members.push(intern.intern(&format!(
                "{}.M.{}:{}:{params}",
                item.name, method.name, method.return_kind
            )));
        }
        for property in &item.properties {
            members.push(intern.intern(&format!(
                "{}.P.{}:{}:{}:{}:{}:{}",
                item.name,
                property.name,
                property.kind,
                u8::from(property.is_nullable),
                u8::from(property.can_read),
                u8::from(property.can_write),
                property.element_kind.as_deref().unwrap_or("")
            )));
        }
        for event in &item.events {
            let params = event
                .parameters
                .iter()
                .map(|parameter| {
                    format!(
                        "{}:{}:{}:{}",
                        parameter.name,
                        parameter.kind,
                        parameter.direction,
                        u8::from(parameter.is_nullable)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            members.push(intern.intern(&format!(
                "{}.E.{}:{}:{params}",
                item.name, event.name, event.payload_kind
            )));
        }
    }
    Ok((bases, members))
}

fn index_unique_iids(
    interfaces: &[AbiInterface],
) -> Result<HashMap<String, &AbiInterface>, String> {
    let mut map = HashMap::new();
    for interface in interfaces {
        let key = interface.iid.to_ascii_uppercase();
        if map.insert(key, interface).is_some() {
            return Err(format!(
                "Duplicate IID {} ({}) in ABI inventory.",
                interface.iid, interface.name
            ));
        }
    }
    Ok(map)
}

fn parse_definitions(header: &str) -> Result<AbiDefinitions, String> {
    let win32 = capture_define(header, "#if defined(_WIN32)", "#define AVN_CALL ")?;
    let other = {
        let else_marker = header
            .find("#else")
            .ok_or("ABI header is missing the non-Windows AVN_CALL branch.")?;
        capture_define(&header[else_marker..], "#else", "#define AVN_CALL")?
    };
    let hresult = header
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("typedef ")?
                .strip_suffix(" AvnHResult;")
        })
        .ok_or("ABI header is missing typedef AvnHResult.")?
        .trim()
        .to_string();
    Ok(AbiDefinitions {
        hresult,
        call_win32: win32,
        call_other: other,
        struct_packing: "translation-unit-default".into(),
        pointer_model: "const-and-indirection-only".into(),
    })
}

fn capture_define(header: &str, section: &str, prefix: &str) -> Result<String, String> {
    let start = header
        .find(section)
        .ok_or_else(|| format!("ABI header is missing {section}."))?;
    let slice = &header[start..];
    let line = slice
        .lines()
        .find(|candidate| candidate.trim_start().starts_with("#define AVN_CALL"))
        .ok_or_else(|| format!("ABI header is missing {prefix} in {section}."))?;
    Ok(line
        .trim()
        .trim_start_matches("#define AVN_CALL")
        .trim()
        .to_string())
}

fn parse_value_types(header: &str) -> Result<Vec<AbiValueType>, String> {
    let mut values = Vec::new();
    let mut seen = HashSet::new();
    let mut rest = header;
    while let Some(start) = rest.find("typedef struct Avn") {
        rest = &rest[start + "typedef struct ".len()..];
        let Some(brace) = rest.find(" {") else {
            return Err("Value type is missing a struct body.".into());
        };
        let name = rest[..brace].trim().to_string();
        if !seen.insert(name.clone()) {
            return Err(format!("Duplicate value type {name} in ABI header."));
        }
        rest = &rest[brace + 2..];
        let Some(end) = rest.find("\n} ") else {
            return Err(format!("Value type {name} is unterminated."));
        };
        let closing = rest[end + 3..].trim_start();
        if !closing.starts_with(&name) {
            return Err(format!(
                "Value type {name} closing tag does not match the opening name."
            ));
        }
        let mut fields = Vec::new();
        for line in rest[..end].lines() {
            let line = line.trim().trim_end_matches(';');
            if line.is_empty() {
                continue;
            }
            let Some((type_name, field)) = line.rsplit_once(' ') else {
                return Err(format!("Value type {name} has an unparsed field '{line}'."));
            };
            fields.push(AbiField {
                type_name: type_name.to_string(),
                name: field.to_string(),
            });
        }
        rest = &rest[end + 3..];
        values.push(AbiValueType { name, fields });
    }
    values.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(values)
}

fn parse_guid_block(block: &str) -> Result<String, String> {
    let hex: Vec<u32> = block
        .split("0x")
        .skip(1)
        .map(|part| {
            let digits: String = part
                .chars()
                .take_while(|character| character.is_ascii_hexdigit())
                .collect();
            u32::from_str_radix(&digits, 16).map_err(|_| format!("Malformed GUID hex '{digits}'."))
        })
        .collect::<Result<_, _>>()?;
    if hex.len() != 11 {
        return Err(format!(
            "GUID body must contain 11 hex components, found {}.",
            hex.len()
        ));
    }
    if hex[1..3].iter().any(|value| *value > u16::MAX.into())
        || hex[3..].iter().any(|value| *value > u8::MAX.into())
    {
        return Err("GUID components exceed their declared field widths.".into());
    }
    Ok(format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        hex[0], hex[1], hex[2], hex[3], hex[4], hex[5], hex[6], hex[7], hex[8], hex[9], hex[10]
    ))
}

fn iid_constant_name(interface: &str) -> Result<String, String> {
    let rest = interface
        .strip_prefix("IAvn")
        .ok_or_else(|| format!("Interface {interface} is not an IAvn* name."))?;
    let mut out = String::from("I_AVN");
    for character in rest.chars() {
        if character.is_uppercase() {
            out.push('_');
        }
        out.push(character.to_ascii_uppercase());
    }
    out.push_str("_IID");
    Ok(out)
}

fn parse_slots(owner: &str, body: &str, intern: &mut SlotInterner) -> Result<Vec<usize>, String> {
    let mut slots = Vec::new();
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let slot = parse_slot(owner, line, slots.len() as i32)?;
        slots.push(intern.intern(slot));
    }
    Ok(slots)
}

fn parse_slot(owner: &str, line: &str, expected_index: i32) -> Result<AbiSlot, String> {
    let (return_type, rest) = line
        .split_once(" (AVN_CALL *")
        .ok_or_else(|| format!("Interface {owner} has an unparsed slot '{line}'."))?;
    let (name, rest) = rest
        .split_once(")(")
        .ok_or_else(|| format!("Interface {owner} slot name is unterminated: '{line}'."))?;
    let (params_body, suffix) = rest
        .split_once(");")
        .ok_or_else(|| format!("Interface {owner}::{name} is missing a parameter terminator."))?;
    let index = suffix
        .trim()
        .strip_prefix("/* slot ")
        .and_then(|value| value.strip_suffix(" */"))
        .and_then(|value| value.parse::<i32>().ok())
        .ok_or_else(|| format!("Interface {owner}::{name} is missing a slot index comment."))?;
    if index != expected_index {
        return Err(format!(
            "Interface {owner}::{name} slot index {index} is not sequential (expected {expected_index})."
        ));
    }
    let mut parameters = Vec::new();
    let params_body = params_body.trim();
    if !params_body.is_empty() && params_body != "void" {
        for part in params_body.split(',') {
            let part = part.split_whitespace().collect::<Vec<_>>().join(" ");
            let Some((type_name, param_name)) = part.rsplit_once(' ') else {
                return Err(format!(
                    "Interface {owner}::{name} has an unparsed parameter '{part}'."
                ));
            };
            let type_name = if type_name == format!("{owner}*") {
                "$self".into()
            } else {
                type_name.to_string()
            };
            parameters.push(AbiParameter {
                type_name,
                name: param_name.to_string(),
            });
        }
    }
    Ok(AbiSlot {
        name: name.to_string(),
        calling_convention: "AVN_CALL".into(),
        return_type: return_type.trim().to_string(),
        parameters,
    })
}

#[derive(Default)]
struct SlotInterner {
    index: HashMap<String, usize>,
    slots: Vec<AbiSlot>,
}

impl SlotInterner {
    fn intern(&mut self, slot: AbiSlot) -> usize {
        let key = format!(
            "{}|{}|{}|{}",
            slot.name,
            slot.calling_convention,
            slot.return_type,
            slot.parameters
                .iter()
                .map(|parameter| format!("{}:{}", parameter.type_name, parameter.name))
                .collect::<Vec<_>>()
                .join(",")
        );
        if let Some(index) = self.index.get(&key) {
            return *index;
        }
        let index = self.slots.len();
        self.index.insert(key, index);
        self.slots.push(slot);
        index
    }
}

#[derive(Default)]
struct MemberInterner {
    index: HashMap<String, usize>,
    members: Vec<String>,
}

impl MemberInterner {
    fn intern(&mut self, member: &str) -> usize {
        if let Some(index) = self.index.get(member) {
            return *index;
        }
        let index = self.members.len();
        self.index.insert(member.to_string(), index);
        self.members.push(member.to_string());
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASELINE_JSON: &str = include_str!("../../abi-baseline.json");
    const HEADER: &str = include_str!("../../avalonia-sys/include/avalonia-rust-abi.h");
    const IR_JSON: &str = include_str!("../../projection.ir.json");

    fn released() -> AbiBaseline {
        serde_json::from_str(BASELINE_JSON).unwrap()
    }

    fn current_inventory() -> AbiInventory {
        let mut inventory = parse_header(HEADER).unwrap();
        let ir: ProjectionIr = serde_json::from_str(IR_JSON).unwrap();
        attach_ir(&mut inventory, &ir).unwrap();
        inventory
    }

    #[test]
    fn released_snapshot_is_preserved_by_current_header_and_ir() {
        let released = released();
        assert_eq!(released.schema_version, 3);
        assert_eq!(released.kind, "released");
        assert_eq!(
            released.producer_pin,
            "9654332a79f637473da96054f75b2a16deaa557e"
        );
        assert_eq!(released.abi_definitions.hresult, "int32_t");
        assert_eq!(released.abi_definitions.call_win32, "__stdcall");
        assert_eq!(
            released.abi_definitions.pointer_model,
            "const-and-indirection-only"
        );
        let current = current_inventory();
        assert!(
            identity_breaks(&released, &current).unwrap().is_empty(),
            "{}",
            identity_breaks(&released, &current).unwrap().join("\n")
        );
        assert!(current.interfaces.len() >= released.interfaces.len());
    }

    #[test]
    fn additive_interface_with_new_iid_is_not_a_break() {
        let released = released();
        let mut current = current_inventory();
        current.interfaces.push(AbiInterface {
            name: "IAvnFuture".into(),
            iid: "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF".into(),
            iid_constant: "I_AVN_FUTURE_IID".into(),
            abi_version: 1,
            slots: vec![],
            ir_bases: vec![],
            ir_members: vec![],
        });
        assert!(identity_breaks(&released, &current).unwrap().is_empty());
    }

    #[test]
    fn changing_avn_call_definition_is_a_contract_break() {
        let released = released();
        let mutated = HEADER.replace("#define AVN_CALL __stdcall", "#define AVN_CALL __cdecl");
        assert_ne!(mutated, HEADER);
        let current = parse_header(&mutated).unwrap();
        let error = identity_breaks(&released, &current).unwrap_err();
        assert!(error.contains("typedef/macro/packing"), "{error}");
    }

    #[test]
    fn changing_hresult_typedef_is_a_contract_break() {
        let released = released();
        let mutated = HEADER.replace("typedef int32_t AvnHResult;", "typedef int64_t AvnHResult;");
        assert_ne!(mutated, HEADER);
        let current = parse_header(&mutated).unwrap();
        let error = identity_breaks(&released, &current).unwrap_err();
        assert!(error.contains("typedef/macro/packing"), "{error}");
    }

    #[test]
    fn same_iid_with_changed_parameter_type_is_a_contract_break() {
        let released = released();
        let mutated = HEADER.replace(
            "AvnHResult (AVN_CALL *get_name)(IAvnControl* self, uint16_t** value); /* slot 5 */",
            "AvnHResult (AVN_CALL *get_name)(IAvnControl* self, int32_t* value); /* slot 5 */",
        );
        assert_ne!(mutated, HEADER);
        let mut current = parse_header(&mutated).unwrap();
        let ir: ProjectionIr = serde_json::from_str(IR_JSON).unwrap();
        attach_ir(&mut current, &ir).unwrap();
        let control_iid = &released
            .interfaces
            .iter()
            .find(|interface| interface.name == "IAvnControl")
            .expect("the released baseline lists IAvnControl")
            .iid[..8];
        let breaks = identity_breaks(&released, &current).unwrap();
        assert!(
            breaks
                .iter()
                .any(|message| message.contains(control_iid)
                    && message.contains("callable signature")),
            "{breaks:?}"
        );
    }

    #[test]
    fn derived_identity_tracks_base_ir_nullability() {
        let released = released();
        let mut ir: ProjectionIr = serde_json::from_str(IR_JSON).unwrap();
        let base = ir
            .types
            .iter_mut()
            .find(|ty| ty.name == "IAvnStyledElement")
            .unwrap();
        base.properties[0].is_nullable = !base.properties[0].is_nullable;
        let mut current = parse_header(HEADER).unwrap();
        attach_ir(&mut current, &ir).unwrap();
        let breaks = identity_breaks(&released, &current).unwrap();
        assert!(
            breaks.iter().any(|message| message.contains("IAvnControl")
                && message.contains("flattened IR lineage")),
            "{breaks:?}"
        );
    }

    #[test]
    fn mismatched_iid_constant_and_vtable_fail_closed() {
        let mutated = HEADER.replace(
            "static const AvnGuid I_AVN_CONTROL_IID",
            "static const AvnGuid I_AVN_WRONG_IID",
        );
        let error = parse_header(&mutated).unwrap_err();
        assert!(
            error.contains("I_AVN_WRONG_IID") || error.contains("not followed"),
            "{error}"
        );
    }

    #[test]
    fn duplicate_value_type_records_fail_closed() {
        let extra = "\ntypedef struct AvnColor {\n    uint32_t argb;\n} AvnColor;\n";
        let error = parse_header(&format!("{HEADER}{extra}")).unwrap_err();
        assert!(error.contains("Duplicate value type AvnColor"), "{error}");
    }

    #[test]
    fn duplicate_iid_records_fail_closed() {
        let released = released();
        let mut current = current_inventory();
        let duplicate = current.interfaces[0].clone();
        current.interfaces.push(duplicate);
        let error = identity_breaks(&released, &current).unwrap_err();
        assert!(error.contains("Duplicate IID"), "{error}");
    }

    #[test]
    fn guid_components_must_fit_their_native_fields() {
        let error = parse_guid_block(
            "0x00000001, 0x10000, 0x0003, { 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B }",
        )
        .unwrap_err();
        assert!(error.contains("field widths"), "{error}");
    }

    #[test]
    fn missing_published_iid_is_a_contract_break() {
        let released = released();
        let mut current = current_inventory();
        let removed = current.interfaces.remove(0);
        let breaks = identity_breaks(&released, &current).unwrap();
        assert!(
            breaks
                .iter()
                .any(|message| message.contains(&removed.iid) && message.contains("missing")),
            "{breaks:?}"
        );
    }
}
