use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionIr {
    pub version: i32,
    pub source_assembly: Option<String>,
    pub factory_iid: Option<String>,
    pub brush_interface_name: Option<String>,
    pub brush_interface_iid: Option<String>,
    #[serde(default)]
    pub gradient_brush_interface_name: Option<String>,
    #[serde(default)]
    pub gradient_brush_interface_iid: Option<String>,
    #[serde(default)]
    pub linear_gradient_brush_interface_name: Option<String>,
    #[serde(default)]
    pub linear_gradient_brush_interface_iid: Option<String>,
    #[serde(default)]
    pub radial_gradient_brush_interface_name: Option<String>,
    #[serde(default)]
    pub radial_gradient_brush_interface_iid: Option<String>,
    pub command_interface_name: Option<String>,
    pub command_interface_iid: Option<String>,
    #[serde(default)]
    pub command_handler_interface_name: Option<String>,
    #[serde(default)]
    pub command_handler_interface_iid: Option<String>,
    #[serde(default)]
    pub template_interface_name: Option<String>,
    #[serde(default)]
    pub template_interface_iid: Option<String>,
    #[serde(default)]
    pub item_filter_interface_name: Option<String>,
    #[serde(default)]
    pub item_filter_interface_iid: Option<String>,
    #[serde(default)]
    pub text_filter_interface_name: Option<String>,
    #[serde(default)]
    pub text_filter_interface_iid: Option<String>,
    #[serde(default)]
    pub notification_interface_name: Option<String>,
    #[serde(default)]
    pub notification_interface_iid: Option<String>,
    #[serde(default)]
    pub notification_handler_interface_iid: Option<String>,
    #[serde(default)]
    pub item_selector_interface_name: Option<String>,
    #[serde(default)]
    pub item_selector_interface_iid: Option<String>,
    #[serde(default)]
    pub text_selector_interface_name: Option<String>,
    #[serde(default)]
    pub text_selector_interface_iid: Option<String>,
    #[serde(default)]
    pub popup_placement_interface_name: Option<String>,
    #[serde(default)]
    pub popup_placement_interface_iid: Option<String>,
    #[serde(default)]
    pub async_populator_interface_name: Option<String>,
    #[serde(default)]
    pub async_populator_interface_iid: Option<String>,
    #[serde(default)]
    pub async_populator_completion_interface_name: Option<String>,
    #[serde(default)]
    pub async_populator_completion_interface_iid: Option<String>,
    #[serde(default)]
    pub types: Vec<ProjectedType>,
    #[serde(default)]
    pub enums: Vec<ProjectedEnum>,
    #[serde(default)]
    pub attached_properties: Vec<ProjectedAttachedProperty>,
    #[serde(default)]
    pub skipped: Vec<SkippedMember>,
}

impl ProjectionIr {
    pub fn validate(&self) -> Result<(), crate::GenerationError> {
        crate::validate::validate(self)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedAttachedProperty {
    pub owner_name: String,
    pub owner_managed_full_name: String,
    pub statics_interface_name: String,
    pub statics_interface_iid: String,
    pub name: String,
    pub kind: String,
    pub managed_type_name: String,
    #[serde(default)]
    pub is_nullable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedEnum {
    pub name: String,
    pub full_name: String,
    #[serde(default)]
    pub is_flags: bool,
    #[serde(default)]
    pub values: Vec<ProjectedEnumValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedEnumValue {
    pub name: String,
    pub value: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedType {
    pub name: String,
    pub full_name: String,
    pub kind: String,
    pub iid: Option<String>,
    pub base_full_name: Option<String>,
    pub managed_full_name: Option<String>,
    #[serde(default)]
    pub is_constructible: bool,
    #[serde(default)]
    pub methods: Vec<ProjectedMethod>,
    #[serde(default)]
    pub properties: Vec<ProjectedProperty>,
    #[serde(default)]
    pub events: Vec<ProjectedEvent>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedEvent {
    pub name: String,
    pub handler_interface_name: String,
    pub handler_interface_iid: String,
    pub payload_kind: String,
    pub managed_handler_type_name: Option<String>,
    #[serde(default)]
    pub args_interface_name: Option<String>,
    #[serde(default)]
    pub args_interface_iid: Option<String>,
    #[serde(default)]
    pub parameters: Vec<ProjectedParameter>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedMethod {
    pub name: String,
    pub managed_name: Option<String>,
    pub return_kind: String,
    #[serde(default)]
    pub preserve_sig: bool,
    #[serde(default)]
    pub parameters: Vec<ProjectedParameter>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedProperty {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub can_read: bool,
    #[serde(default)]
    pub can_write: bool,
    pub interface_name: Option<String>,
    pub interface_iid: Option<String>,
    pub element_interface_name: Option<String>,
    pub element_kind: Option<String>,
    pub managed_type_name: Option<String>,
    #[serde(default)]
    pub is_nullable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedParameter {
    pub name: String,
    pub kind: String,
    pub direction: String,
    pub interface_name: Option<String>,
    pub managed_type_name: Option<String>,
    #[serde(default)]
    pub is_nullable: bool,
}

#[derive(Debug, Deserialize)]
pub struct SkippedMember {
    pub owner: String,
    pub member: String,
    pub reason: String,
}
