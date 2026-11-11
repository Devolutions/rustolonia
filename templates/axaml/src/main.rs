include!(concat!(env!("OUT_DIR"), "/main_window.rs"));
include!(concat!(env!("OUT_DIR"), "/filter_editor.rs"));

mod components;

use generated_filter_editor::{build_filter_editor, FilterEditorBindings};
use generated_main_window::{build_main_window, MainWindowBindings, MainWindowHandlers};
use rustolonia::{
    ConversionDirection, Error, Lens, PropertyPath, RegisteredValueConverter, ScalarKind,
    ScalarPath, ScalarValue, ValueConverterDispatch, Variant,
};
use std::sync::Arc;

#[derive(Clone, PartialEq, Eq)]
pub struct Contact {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, PartialEq)]
pub struct EditableContact {
    pub id: u32,
    pub name: rustolonia::Signal<String>,
}

#[derive(Clone, Lens)]
struct Profile {
    title: String,
    message: String,
    port: String,
}

struct SampleConverters;

impl ValueConverterDispatch for SampleConverters {
    fn convert(
        &self,
        converter_id: i32,
        direction: ConversionDirection,
        value: ScalarValue,
        _parameter: ScalarValue,
        _target_kind: ScalarKind,
        _culture: &str,
    ) -> rustolonia::Result<ScalarValue> {
        match (converter_id, direction, value) {
            (1, ConversionDirection::Convert, ScalarValue::Int64(value)) => {
                Ok(ScalarValue::String(value.to_string()))
            }
            (1, ConversionDirection::ConvertBack, ScalarValue::String(value)) => value
                .parse::<i64>()
                .map(ScalarValue::Int64)
                .map_err(|_| Error::Binding("enter a whole number".to_string())),
            _ => Err(Error::Binding("unsupported sample conversion".to_string())),
        }
    }
}

fn main() -> rustolonia::Result<()> {
    rustolonia::App::load_from_env()?.run(build)
}

fn build(scope: &rustolonia::AppScope) -> rustolonia::Result<()> {
    let name = scope.signal(String::new());
    let greeting = scope.signal("Type your name, then say hello.".to_string());
    let theme_resources = rustolonia::ResourceScope::new();
    let theme_dictionary = theme_resources.dictionary()?;
    theme_dictionary.insert_for_theme(
        rustolonia::ThemeVariant::Default,
        "WindowBackground",
        Some(rustolonia::Brush::solid(rustolonia::Color::rgb(
            255, 255, 255,
        ))),
    )?;
    theme_dictionary.insert_for_theme(
        rustolonia::ThemeVariant::Dark,
        "WindowBackground",
        Some(rustolonia::Brush::solid(rustolonia::Color::rgb(32, 32, 40))),
    )?;
    theme_resources.merge(&theme_dictionary)?;
    let window_background = theme_resources.dynamic("WindowBackground")?;
    let contacts = scope.signal(vec![
        Contact {
            id: 1,
            name: "Ada".to_string(),
        },
        Contact {
            id: 2,
            name: "Grace".to_string(),
        },
    ]);
    let editable_contacts = scope.signal(vec![EditableContact {
        id: 1,
        name: scope.signal("Editable contact".to_string()),
    }]);
    let child_view = rustolonia::ControlSlot::new();
    let selected_child = scope.signal(Some(String::from("This child view is selected in Rust.")));
    let child_views = rustolonia::ViewRegistry::new(|_: &String| ());
    child_views.register((), |text| rustolonia::TextBlock::el().text(text))?;
    let contact_views = rustolonia::ViewRegistry::new(|_: &Contact| ());
    contact_views.register((), |contact| {
        rustolonia::TextBlock::el().text(move || contact.get().name)
    })?;
    let recent_contacts =
        rustolonia::ItemsView::from_registry(contacts, |contact| contact.id, contact_views);
    let resources = rustolonia::LocalizationResources::new("en-US")?;
    resources.insert(
        "en-US",
        "GreetingInstruction",
        "Enter your name, then say hello.",
    )?;
    resources.insert(
        "fr-FR",
        "GreetingInstruction",
        "Saisissez votre nom, puis dites bonjour.",
    )?;
    let greeting_instruction = resources.string_signal("GreetingInstruction")?;
    scope.retain_localization_resources(resources);

    let profile = scope.signal(Profile {
        title: "Connection presets".to_string(),
        message: "Choose a port to run the Rust command.".to_string(),
        port: "2222".to_string(),
    });
    let port_spec = scope.signal(String::new());
    let command = rustolonia::Command::new(move |parameter| {
        let Variant::Utf16(value) = parameter else {
            return Err(Error::Binding(
                "port parameter must be a string".to_string(),
            ));
        };
        port_spec.set(value);
        Ok(())
    });
    let number = scope.signal(12_i64);
    let culture = scope.signal("en-US".to_string());
    let converters: Arc<dyn ValueConverterDispatch> = Arc::new(SampleConverters);

    let bindings = MainWindowBindings {
        name,
        greeting,
        resource_greeting_instruction: greeting_instruction,
        resource_window_background: window_background,
        optional_flag: scope.signal(None::<bool>),
        contacts,
        editable_contacts,
        child_view,
        recent_contacts,
        contacts_item_refs_1: rustolonia::ItemNameScope::default(),
        profile_title: Profile::lens_title(profile),
        profile_message: Profile::lens_message(profile),
        set_ports_command: scope.signal(Some(command)),
        profile_port_command_parameter: PropertyPath::read_only(move || {
            Ok(Variant::Utf16(profile.get().port))
        }),
        port_spec,
        number_converter_source: ScalarPath::new(
            ScalarKind::Int64,
            PropertyPath::writable(
                move || Ok(ScalarValue::Int64(number.get())),
                move |value| {
                    let ScalarValue::Int64(value) = value else {
                        return Err(Error::Binding("number must be an integer".to_string()));
                    };
                    number.set_if_changed(value);
                    Ok(())
                },
            ),
        )?,
        converter_resource_integer_converter: RegisteredValueConverter::new(
            "StaticResource IntegerConverter",
            1,
            converters,
            culture,
        )?,
    };
    let handlers = MainWindowHandlers {
        on_greet: Box::new(move |_| {
            let name = name.get();
            let message = if name.trim().is_empty() {
                "Please enter your name.".to_string()
            } else {
                format!("Hello, {name}!")
            };
            greeting.set(message);
        }),
        on_contact_selection_changed: Box::new(|_| {}),
    };

    let (window, refs) = build_main_window(&bindings, handlers);
    let window = window.build(scope)?;
    let editor_bindings = FilterEditorBindings {
        kind: scope.signal(components::FilterKind::Name),
        allowed_kinds: scope.signal(Vec::new()),
        search_text: scope.signal(String::new()),
    };
    let (editor, _) = build_filter_editor(&editor_bindings);
    let editor = editor
        .provide_context(components::ComponentContext {
            culture,
            compact_height: 32.0,
            right_margin: 1.0,
        })
        .build(scope)?;
    refs.custom_editor
        .get()
        .ok_or_else(|| Error::Binding("custom editor host was not built".into()))?
        .set_content(&editor)?;
    child_view.bind(scope, selected_child, child_views)?;
    let theme = scope.theme_variant_signal(&window)?;
    let resources_for_theme = theme_resources.clone();
    scope.effect(move || resources_for_theme.set_theme(theme.get()))?;
    scope.retain_resource_scope(theme_resources);
    scope.mount(window)
}
