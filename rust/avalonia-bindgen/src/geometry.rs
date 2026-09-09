//! Blittable Avalonia value types carried across the nano-COM ABI by value.
//!
//! Mirrors `Avalonia.Projection.Ir.GeometryMarshalling` on the managed side. Every
//! struct is `#[repr(C)]` and sequential so the C, C#, and Rust declarations agree.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    F64,
    U32,
}

impl FieldKind {
    pub fn rust_type(self) -> &'static str {
        match self {
            FieldKind::F64 => "f64",
            FieldKind::U32 => "u32",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conversion {
    /// Safe fields map one-to-one onto the ABI fields.
    Components,
    /// A packed ARGB integer matching `Avalonia.Media.Color.ToUInt32()`.
    PackedColor,
}

#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub name: &'static str,
    pub kind: FieldKind,
}

/// Extra constructors emitted onto a safe geometry struct so callers can write
/// `Thickness::uniform(8.0)` instead of repeating the same scalar four times.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Helper {
    /// `uniform(value)`: every field takes the same scalar.
    Uniform,
    /// `symmetric(horizontal, vertical)`: the listed fields take `horizontal`, the rest `vertical`.
    Symmetric(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug)]
pub struct GeometryStruct {
    /// The `MarshallingKind` name used by the projection IR.
    pub kind: &'static str,
    /// The ABI struct name shared with the C header and the C# COM layer.
    pub abi_name: &'static str,
    /// The ergonomic name exposed by the safe `avalonia` crate.
    pub safe_name: &'static str,
    pub managed_type_name: &'static str,
    pub conversion: Conversion,
    pub fields: &'static [Field],
    /// Ergonomic constructors emitted in addition to `new`.
    pub helpers: &'static [Helper],
}

const fn f64_field(name: &'static str) -> Field {
    Field {
        name,
        kind: FieldKind::F64,
    }
}

pub const GEOMETRY: &[GeometryStruct] = &[
    GeometryStruct {
        kind: "Thickness",
        abi_name: "AvnThickness",
        safe_name: "Thickness",
        managed_type_name: "Avalonia.Thickness",
        conversion: Conversion::Components,
        fields: &[
            f64_field("left"),
            f64_field("top"),
            f64_field("right"),
            f64_field("bottom"),
        ],
        helpers: &[Helper::Uniform, Helper::Symmetric(&["left", "right"])],
    },
    GeometryStruct {
        kind: "CornerRadius",
        abi_name: "AvnCornerRadius",
        safe_name: "CornerRadius",
        managed_type_name: "Avalonia.CornerRadius",
        conversion: Conversion::Components,
        fields: &[
            f64_field("top_left"),
            f64_field("top_right"),
            f64_field("bottom_right"),
            f64_field("bottom_left"),
        ],
        helpers: &[Helper::Uniform],
    },
    GeometryStruct {
        kind: "Size",
        abi_name: "AvnSize",
        safe_name: "Size",
        managed_type_name: "Avalonia.Size",
        conversion: Conversion::Components,
        fields: &[f64_field("width"), f64_field("height")],
        helpers: &[],
    },
    GeometryStruct {
        kind: "Point",
        abi_name: "AvnPoint",
        safe_name: "Point",
        managed_type_name: "Avalonia.Point",
        conversion: Conversion::Components,
        fields: &[f64_field("x"), f64_field("y")],
        helpers: &[],
    },
    GeometryStruct {
        kind: "Rect",
        abi_name: "AvnRect",
        safe_name: "Rect",
        managed_type_name: "Avalonia.Rect",
        conversion: Conversion::Components,
        fields: &[
            f64_field("x"),
            f64_field("y"),
            f64_field("width"),
            f64_field("height"),
        ],
        helpers: &[],
    },
    GeometryStruct {
        kind: "Color",
        abi_name: "AvnColor",
        safe_name: "Color",
        managed_type_name: "Avalonia.Media.Color",
        conversion: Conversion::PackedColor,
        fields: &[Field {
            name: "argb",
            kind: FieldKind::U32,
        }],
        helpers: &[],
    },
    GeometryStruct {
        kind: "Vector",
        abi_name: "AvnVector",
        safe_name: "Vector",
        managed_type_name: "Avalonia.Vector",
        conversion: Conversion::Components,
        fields: &[f64_field("x"), f64_field("y")],
        helpers: &[],
    },
];

pub fn find(kind: &str) -> Option<&'static GeometryStruct> {
    GEOMETRY.iter().find(|geometry| geometry.kind == kind)
}

pub fn is_geometry(kind: &str) -> bool {
    find(kind).is_some()
}

impl GeometryStruct {
    pub fn optional_abi_name(&self) -> String {
        format!("AvnOptional{}", self.kind)
    }
}

/// `#[repr(C)]` ABI structs for the `avalonia-sys` crate.
pub fn emit_sys_structs() -> String {
    let mut out = String::new();
    for geometry in GEOMETRY {
        out.push_str(&format!(
            "/// Blittable ABI mirror of `{}`.\n\
             #[repr(C)]\n\
             #[derive(Clone, Copy, Debug, Default, PartialEq)]\n\
             pub struct {} {{\n",
            geometry.managed_type_name, geometry.abi_name
        ));
        for field in geometry.fields {
            out.push_str(&format!(
                "    pub {}: {},\n",
                field.name,
                field.kind.rust_type()
            ));
        }
        out.push_str("}\n\n");
        let optional = geometry.optional_abi_name();
        out.push_str(&format!(
            "/// Nullable ABI wrapper of `{abi}`.\n\
             #[repr(C)]\n\
             #[derive(Clone, Copy, Debug, Default, PartialEq)]\n\
             pub struct {optional} {{\n\
             \x20   pub has_value: i32,\n\
             \x20   pub value: {abi},\n\
             }}\n\n",
            abi = geometry.abi_name
        ));
    }
    out
}

/// Ergonomic structs plus `From`/`Into` bridges for the safe `avalonia` crate.
pub fn emit_safe_structs() -> String {
    let mut out = String::new();
    for geometry in GEOMETRY {
        let safe = geometry.safe_name;
        let abi = geometry.abi_name;
        let safe_fields: Vec<(&str, &str)> = match geometry.conversion {
            Conversion::Components => geometry
                .fields
                .iter()
                .map(|field| (field.name, field.kind.rust_type()))
                .collect(),
            Conversion::PackedColor => vec![("a", "u8"), ("r", "u8"), ("g", "u8"), ("b", "u8")],
        };
        out.push_str(&format!(
            "/// Safe mirror of `{}`, marshalled as `sys::{abi}`.\n\
             #[derive(Clone, Copy, Debug, Default, PartialEq)]\n\
             pub struct {safe} {{\n",
            geometry.managed_type_name
        ));
        for (name, ty) in &safe_fields {
            out.push_str(&format!("    pub {name}: {ty},\n"));
        }
        out.push_str("}\n\n");

        let parameters = safe_fields
            .iter()
            .map(|(name, ty)| format!("{name}: {ty}"))
            .collect::<Vec<_>>()
            .join(", ");
        let initializers = safe_fields
            .iter()
            .map(|(name, _)| (*name).to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let mut helpers = String::new();
        for helper in geometry.helpers {
            assert_eq!(
                geometry.conversion,
                Conversion::Components,
                "{safe}: helpers assume the safe fields mirror the ABI fields"
            );
            assert!(
                geometry
                    .fields
                    .iter()
                    .all(|field| field.kind == FieldKind::F64),
                "{safe}: helpers assume every field is an f64"
            );
            match helper {
                Helper::Uniform => {
                    let assignments = geometry
                        .fields
                        .iter()
                        .map(|field| format!("{}: value", field.name))
                        .collect::<Vec<_>>()
                        .join(", ");
                    helpers.push_str(&format!(
                        "    /// Every component takes the same value.\n\
                         \x20   pub const fn uniform(value: f64) -> Self {{\n\
                         \x20       Self {{ {assignments} }}\n\
                         \x20   }}\n"
                    ));
                }
                Helper::Symmetric(horizontal_fields) => {
                    let assignments = geometry
                        .fields
                        .iter()
                        .map(|field| {
                            let source = if horizontal_fields.contains(&field.name) {
                                "horizontal"
                            } else {
                                "vertical"
                            };
                            format!("{}: {source}", field.name)
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    helpers.push_str(&format!(
                        "    /// Applies `horizontal` to {} and `vertical` to the rest.\n\
                         \x20   pub const fn symmetric(horizontal: f64, vertical: f64) -> Self {{\n\
                         \x20       Self {{ {assignments} }}\n\
                         \x20   }}\n",
                        horizontal_fields.join("/")
                    ));
                }
            }
        }
        let optional = geometry.optional_abi_name();
        out.push_str(&format!(
            "impl {safe} {{\n\
             \x20   pub const fn new({parameters}) -> Self {{\n\
             \x20       Self {{ {initializers} }}\n\
             \x20   }}\n\
             {helpers}    pub(crate) fn from_optional_abi(value: sys::{optional}) -> Option<Self> {{\n\
             \x20       if value.has_value != 0 {{\n\
             \x20           Some(value.value.into())\n\
             \x20       }} else {{\n\
             \x20           None\n\
             \x20       }}\n\
             \x20   }}\n\
             \x20   pub(crate) fn to_optional_abi(value: Option<Self>) -> sys::{optional} {{\n\
             \x20       match value {{\n\
             \x20           Some(inner) => sys::{optional} {{ has_value: 1, value: inner.into() }},\n\
             \x20           None => sys::{optional} {{ has_value: 0, value: Default::default() }},\n\
             \x20       }}\n\
             \x20   }}\n\
             }}\n\n"
        ));

        if geometry.conversion == Conversion::PackedColor {
            // A brush caller almost always wants an opaque colour, so spell it once here
            // instead of repeating `255` at every call site.
            out.push_str(&format!(
                "impl {safe} {{\n\
                 \x20   /// A fully opaque colour.\n\
                 \x20   pub const fn rgb(r: u8, g: u8, b: u8) -> Self {{\n\
                 \x20       Self::new(255, r, g, b)\n\
                 \x20   }}\n\
                 }}\n\n"
            ));
        }

        match geometry.conversion {
            Conversion::Components => {
                let from_abi = geometry
                    .fields
                    .iter()
                    .map(|field| format!("            {}: value.{},\n", field.name, field.name))
                    .collect::<String>();
                let to_abi = geometry
                    .fields
                    .iter()
                    .map(|field| format!("            {}: value.{},\n", field.name, field.name))
                    .collect::<String>();
                out.push_str(&format!(
                    "impl From<sys::{abi}> for {safe} {{\n\
                     \x20   fn from(value: sys::{abi}) -> Self {{\n\
                     \x20       Self {{\n{from_abi}        }}\n\
                     \x20   }}\n\
                     }}\n\n\
                     impl From<{safe}> for sys::{abi} {{\n\
                     \x20   fn from(value: {safe}) -> Self {{\n\
                     \x20       Self {{\n{to_abi}        }}\n\
                     \x20   }}\n\
                     }}\n\n"
                ));
            }
            Conversion::PackedColor => out.push_str(&format!(
                "impl From<sys::{abi}> for {safe} {{\n\
                 \x20   fn from(value: sys::{abi}) -> Self {{\n\
                 \x20       Self {{\n\
                 \x20           a: (value.argb >> 24) as u8,\n\
                 \x20           r: (value.argb >> 16) as u8,\n\
                 \x20           g: (value.argb >> 8) as u8,\n\
                 \x20           b: value.argb as u8,\n\
                 \x20       }}\n\
                 \x20   }}\n\
                 }}\n\n\
                 impl From<{safe}> for sys::{abi} {{\n\
                 \x20   fn from(value: {safe}) -> Self {{\n\
                 \x20       Self {{\n\
                 \x20           argb: (u32::from(value.a) << 24)\n\
                 \x20               | (u32::from(value.r) << 16)\n\
                 \x20               | (u32::from(value.g) << 8)\n\
                 \x20               | u32::from(value.b),\n\
                 \x20       }}\n\
                 \x20   }}\n\
                 }}\n\n"
            )),
        }
    }
    out
}
