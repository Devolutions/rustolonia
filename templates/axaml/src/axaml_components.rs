use rustolonia_bindgen::{
    AxamlBindingDefault, AxamlBindingMode, AxamlCompileOptions, AxamlComponent,
    AxamlComponentProperty, AxamlComponentValueType, AxamlRootAlias, AxamlTypeMapping,
};

pub fn options() -> AxamlCompileOptions {
    AxamlCompileOptions {
        root_aliases: vec![AxamlRootAlias {
            namespace_uri: "urn:example:views".into(),
            name: "FilterEditor".into(),
            projected_control: "UserControl".into(),
        }],
        components: vec![
            AxamlComponent {
                namespace_uri: "https://github.com/avaloniaui".into(),
                name: "FormGrid".into(),
                projected_control: "Grid".into(),
                factory: "crate::components::form_grid".into(),
                type_argument_count: 0,
                finish_method: None,
                after_children: Some("crate::components::finish_form_grid".into()),
                properties: vec![],
            },
            AxamlComponent {
                namespace_uri: "https://github.com/avaloniaui".into(),
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
            namespace_uri: "urn:example:models".into(),
            name: "FilterKind".into(),
            rust_type: "crate::components::FilterKind".into(),
        }],
        binding_defaults: vec![AxamlBindingDefault {
            projected_control: "TextBox".into(),
            property: "Text".into(),
            mode: AxamlBindingMode::TwoWay,
        }],
    }
}
