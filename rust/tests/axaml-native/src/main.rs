include!(concat!(env!("OUT_DIR"), "/main_window.rs"));
include!(concat!(env!("OUT_DIR"), "/filter_editor.rs"));

mod components;

use generated_filter_editor::{build_filter_editor, FilterEditorBindings};
use generated_main_window::{
    build_main_window, MainWindowBindings, MainWindowHandlers, MainWindowItemTemplate1Refs,
    MainWindowRefs,
};
use rustolonia::{
    App, AppScope, Border, Brush, Button, CheckBox, Color, Command, Control, ControlSlot,
    ConversionDirection, Downcast, Error, EventSubscription, Grid, ItemNameScope, ItemsControl,
    ItemsView, Label, LocalizationResources, NodeRef, PropertyPath, ReactiveScope,
    RegisteredValueConverter, ResourceScope, ScalarKind, ScalarPath, ScalarValue, ScrollViewer,
    Signal, StackPanel, TextBlock, TextBox, ThemeVariant, Thickness, Upcast,
    ValueConverterDispatch, Variant, ViewRegistry, Window,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

type CheckResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type DeferredChecks = Box<dyn FnOnce() -> CheckResult<(Window, TextBlock)>>;
type ConversionCall = (i32, ConversionDirection, ScalarKind, ScalarValue, String);

thread_local! {
    static DEFERRED_CHECKS: RefCell<Option<DeferredChecks>> = RefCell::new(None);
    static DEFERRED_INPUT: RefCell<Option<(TextBox, Signal<i64>)>> = const { RefCell::new(None) };
}

#[derive(Clone, PartialEq)]
pub struct Contact {
    pub id: u32,
    pub name: String,
    kind: u8,
    marker: Rc<()>,
}

#[derive(Clone, PartialEq)]
pub struct EditableContact {
    pub id: u32,
    pub name: Signal<String>,
}

#[derive(Clone, PartialEq)]
struct ChildModel {
    kind: u8,
    label: String,
}

#[derive(Clone, PartialEq)]
struct InteractiveProfile {
    title: Signal<String>,
    message: Signal<String>,
    port: Signal<String>,
}

#[derive(Default)]
struct NativeConverters {
    calls: Mutex<Vec<ConversionCall>>,
}

impl ValueConverterDispatch for NativeConverters {
    fn convert(
        &self,
        id: i32,
        direction: ConversionDirection,
        value: ScalarValue,
        parameter: ScalarValue,
        target: ScalarKind,
        culture: &str,
    ) -> rustolonia::Result<ScalarValue> {
        self.calls.lock().unwrap().push((
            id,
            direction,
            target,
            parameter.clone(),
            culture.to_string(),
        ));
        match (id, direction, value, target) {
            (1, ConversionDirection::Convert, ScalarValue::String(value), ScalarKind::Boolean) => {
                Ok(ScalarValue::Boolean(!value.is_empty()))
            }
            (2, ConversionDirection::Convert, ScalarValue::Int64(value), ScalarKind::String)
                if parameter == ScalarValue::String("roundtrip".to_string())
                    && culture == "fr-FR" =>
            {
                Ok(ScalarValue::String(value.to_string()))
            }
            (
                2,
                ConversionDirection::ConvertBack,
                ScalarValue::String(value),
                ScalarKind::Int64,
            ) if parameter == ScalarValue::String("roundtrip".to_string())
                && culture == "fr-FR" =>
            {
                value
                    .parse::<i64>()
                    .map(ScalarValue::Int64)
                    .map_err(|_| Error::Binding("number is invalid".to_string()))
            }
            _ => Err(Error::Binding("unexpected native conversion".to_string())),
        }
    }
}

struct InteractiveState {
    profile: Signal<Option<InteractiveProfile>>,
    first: InteractiveProfile,
    second: InteractiveProfile,
    command: Signal<Option<Command>>,
    original: Command,
    replacement: Command,
    enabled: Signal<bool>,
    port_spec: Signal<String>,
    number: Signal<i64>,
    culture: Signal<String>,
    converters: Arc<NativeConverters>,
    queries: Arc<Mutex<Vec<Variant>>>,
}

impl InteractiveState {
    fn new() -> rustolonia::Result<Self> {
        let first = InteractiveProfile {
            title: Signal::new("First".to_string()),
            message: Signal::new("First message".to_string()),
            port: Signal::new("2222".to_string()),
        };
        let second = InteractiveProfile {
            title: Signal::new("Second".to_string()),
            message: Signal::new("Second message".to_string()),
            port: Signal::new("2200".to_string()),
        };
        let profile = Signal::new(Some(first.clone()));
        let port_spec = Signal::new(String::new());
        let enabled = Signal::new(true);
        let queries = Arc::new(Mutex::new(Vec::new()));
        let recorded_queries = queries.clone();
        let original = Command::with_can_execute_parameter(
            enabled,
            move |parameter| {
                recorded_queries.lock().unwrap().push(parameter.clone());
                Ok(matches!(parameter, Variant::Utf16(value) if value != "blocked"))
            },
            move |parameter| {
                let Variant::Utf16(value) = parameter else {
                    return Err(Error::Binding("port must be a string".to_string()));
                };
                port_spec.set(value);
                Ok(())
            },
        )?;
        let replacement = Command::new(move |parameter| {
            let Variant::Utf16(value) = parameter else {
                return Err(Error::Binding("port must be a string".to_string()));
            };
            port_spec.set(format!("replacement:{value}"));
            Ok(())
        });
        Ok(Self {
            profile,
            first,
            second,
            command: Signal::new(Some(original.clone())),
            original,
            replacement,
            enabled,
            port_spec,
            number: Signal::new(12),
            culture: Signal::new("en-US".to_string()),
            converters: Arc::new(NativeConverters::default()),
            queries,
        })
    }

    fn current(
        profile: Signal<Option<InteractiveProfile>>,
    ) -> rustolonia::Result<InteractiveProfile> {
        profile
            .get()
            .ok_or_else(|| Error::Binding("profile is missing".to_string()))
    }
}

struct InteractiveControls {
    title: TextBlock,
    detail: Border,
    message: TextBlock,
    number: TextBox,
    text_changed_count: Arc<AtomicUsize>,
    text_changed_subscription: EventSubscription,
    convert_back_calls: usize,
}

fn check(condition: bool, message: &str) -> CheckResult<()> {
    if condition {
        Ok(())
    } else {
        Err(std::io::Error::other(message).into())
    }
}

fn cast<T>(control: Control, description: &str) -> CheckResult<T>
where
    Control: Downcast<T>,
{
    control
        .downcast()
        .ok_or_else(|| std::io::Error::other(description).into())
}

fn text_value(text: &TextBlock) -> CheckResult<String> {
    text.text()?
        .ok_or_else(|| std::io::Error::other("TextBlock has no text").into())
}

fn item(owner: &impl Upcast<ItemsControl>, index: usize) -> CheckResult<Control> {
    Ok(owner.upcast_ref().items()?.get(index)?)
}

fn direct_item(owner: &impl Upcast<ItemsControl>, index: usize) -> CheckResult<TextBlock> {
    cast(item(owner, index)?, "inline item is not a TextBlock")
}

fn named_item(
    names: &ItemNameScope<Contact, MainWindowItemTemplate1Refs>,
    id: u32,
) -> CheckResult<NodeRef<TextBlock>> {
    names
        .entries()
        .into_iter()
        .find(|(item, _)| item.get_untracked().id == id)
        .map(|(_, refs)| refs.contact_name)
        .ok_or_else(|| std::io::Error::other("missing named item").into())
}

fn named_item_id(node: NodeRef<TextBlock>) -> CheckResult<i64> {
    let control = node
        .get()
        .ok_or_else(|| std::io::Error::other("named item has no control"))?;
    Ok(control.object_id()?)
}

fn child_text(border: &Border) -> CheckResult<TextBlock> {
    cast(
        border
            .child()?
            .ok_or_else(|| std::io::Error::other("view slot has no child"))?,
        "view slot child is not a TextBlock",
    )
}

fn registry_item(
    owner: &impl Upcast<ItemsControl>,
    index: usize,
) -> CheckResult<(Border, TextBlock)> {
    let border = cast(item(owner, index)?, "registry item is not a Border")?;
    let text = child_text(&border)?;
    Ok((border, text))
}

fn exercise_custom_editor(
    scope: &AppScope,
    refs: &MainWindowRefs,
    driver: &Grid,
) -> CheckResult<()> {
    use components::FilterKind;

    let view_scope = ReactiveScope::new();
    let bindings = FilterEditorBindings {
        kind: Signal::new(FilterKind::Name),
        allowed_kinds: Signal::new(Vec::new()),
        search_text: Signal::new("initial".to_string()),
    };
    let culture = Signal::new("en-US".to_string());
    let (editor, editor_refs) = build_filter_editor(&bindings);
    let editor = view_scope.run(|| {
        editor
            .provide_context(components::ComponentContext {
                culture,
                compact_height: 24.0,
                right_margin: 1.0,
            })
            .build(scope)
    })?;
    let host = refs
        .custom_editor
        .get()
        .ok_or_else(|| std::io::Error::other("missing custom editor host"))?;
    host.set_content(&editor)?;
    driver.update_layout()?;
    let grid = editor_refs
        .form_grid
        .get()
        .ok_or_else(|| std::io::Error::other("missing custom grid"))?;
    let picker = editor_refs
        .filter_picker
        .get()
        .ok_or_else(|| std::io::Error::other("missing enum picker"))?;
    let text = editor_refs
        .filter_text
        .get()
        .ok_or_else(|| std::io::Error::other("missing filter text"))?;
    check(
        grid.column_definitions()? == "Auto@FilterColumn,1*".parse()?,
        "shared column metadata did not reach the host",
    )?;
    check(
        text.scroll_viewer_horizontal_scroll_bar_visibility()?
            == rustolonia::ScrollBarVisibility::Hidden,
        "attached scroll visibility did not reach the text box",
    )?;
    check(
        picker.height()? == 24.0
            && picker.margin()?.right == 1.0
            && text.height()? == 24.0
            && text.margin()? == rustolonia::Thickness::uniform(0.0)
            && picker.classes()?.contains("form-grid-item")?
            && picker.classes()?.contains("last-row-item")?
            && text.classes()?.contains("last-column-item")?
            && text.classes()?.contains("preceded-by-control")?,
        "custom grid metrics or classes were not applied",
    )?;
    text.set_visible(false)?;
    grid.update_layout()?;
    let picker_last_column = picker.classes()?.contains("last-column-item")?;
    let picker_right = picker.margin()?.right;
    let text_is_item = text.classes()?.contains("form-grid-item")?;
    check(
        picker_last_column && picker_right == 0.0 && !text_is_item,
        &format!(
            "hiding the text box did not update grid classes and margins \
             (loaded={}, last_column={picker_last_column}, right={picker_right}, text_item={text_is_item})",
            grid.is_loaded()?
        ),
    )?;
    text.set_visible(true)?;
    picker.set_visible(false)?;
    grid.update_layout()?;
    check(
        !text.classes()?.contains("preceded-by-control")?
            && !picker.classes()?.contains("form-grid-item")?,
        "hiding the picker did not clear classes or the preceding-control flag",
    )?;
    picker.set_visible(true)?;
    grid.update_layout()?;
    check(
        picker.margin()?.right == 1.0 && text.classes()?.contains("preceded-by-control")?,
        "restoring visible controls did not restore grid classes",
    )?;
    host.set_visible(false)?;
    grid.update_layout()?;
    check(
        !grid.is_effectively_visible()?
            && !picker.classes()?.contains("form-grid-item")?
            && !text.classes()?.contains("form-grid-item")?,
        "a hidden ancestor did not clear the direct children's classes",
    )?;
    host.set_visible(true)?;
    grid.update_layout()?;
    check(
        grid.is_effectively_visible()?
            && picker.classes()?.contains("form-grid-item")?
            && text.classes()?.contains("form-grid-item")?,
        "restoring an ancestor did not restore grid classes",
    )?;
    check(
        picker.items()?.len()? == 3
            && picker.selected_index()? == 0
            && text_value(&direct_item(&picker, 0)?)? == "Name",
        "initial picker choices or selection",
    )?;
    picker.set_selected_index(1)?;
    check(
        bindings.kind.get() == FilterKind::Owner,
        "selection did not update the model",
    )?;
    bindings.kind.set(FilterKind::Category);
    check(
        picker.selected_index()? == 2,
        "model did not update selection",
    )?;
    bindings.allowed_kinds.set(vec![FilterKind::Name]);
    check(
        picker.items()?.len()? == 1
            && picker.selected_index()? == -1
            && bindings.kind.get() == FilterKind::Category,
        "restricting choices changed an excluded model value",
    )?;
    picker.set_selected_index(0)?;
    check(
        bindings.kind.get() == FilterKind::Name,
        "filtered selection did not update the model",
    )?;
    bindings
        .allowed_kinds
        .set(vec![FilterKind::Category, FilterKind::Name]);
    bindings.kind.set(FilterKind::Category);
    culture.set("fr-FR".into());
    check(
        picker.items()?.len()? == 2
            && text_value(&direct_item(&picker, 0)?)? == "Nom"
            && text_value(&direct_item(&picker, 1)?)? == "Catégorie"
            && picker.selected_index()? == 1
            && bindings.kind.get() == FilterKind::Category,
        "localized choices, ordering, or selection were not preserved",
    )?;
    bindings.allowed_kinds.set(Vec::new());
    check(
        picker.items()?.len()? == 3 && picker.selected_index()? == 2,
        "empty filter did not restore all choices",
    )?;
    bindings.allowed_kinds.update(|choices| {
        choices.clear();
        choices.push(FilterKind::Name);
    });
    check(
        picker.items()?.len()? == 1
            && picker.selected_index()? == -1
            && bindings.kind.get() == FilterKind::Category,
        "an in-place filter update did not restrict the picker",
    )?;
    bindings.allowed_kinds.update(Vec::clear);
    check(
        picker.items()?.len()? == 3
            && picker.selected_index()? == 2
            && text_value(&direct_item(&picker, 0)?)? == "Nom"
            && text_value(&direct_item(&picker, 2)?)? == "Catégorie",
        "clearing the filter in place did not restore ordered localized choices",
    )?;
    check(
        text.text()?.as_deref() == Some("initial"),
        "initial filter text",
    )?;
    text.set_text("edited")?;
    check(
        bindings.search_text.get() == "edited",
        "filter text did not update the model",
    )?;
    bindings.search_text.set("model update".into());
    check(
        text.text()?.as_deref() == Some("model update"),
        "model did not update filter text",
    )?;

    view_scope.dispose();
    check(
        editor_refs.form_grid.get().is_none()
            && editor_refs.filter_picker.get().is_none()
            && editor_refs.filter_text.get().is_none(),
        "disposed custom view retained named controls",
    )?;
    text.set_visible(false)?;
    grid.update_layout()?;
    check(
        !picker.classes()?.contains("last-column-item")?
            && text.classes()?.contains("form-grid-item")?,
        "disposed grid still reacted to layout changes",
    )?;
    picker.set_selected_index(0)?;
    text.set_text("after disposal")?;
    check(
        bindings.kind.get() == FilterKind::Category && bindings.search_text.get() == "model update",
        "disposed custom view still wrote to the model",
    )?;
    bindings.kind.set(FilterKind::Owner);
    bindings.search_text.set("after model disposal".into());
    check(
        picker.selected_index()? == 0 && text.text()?.as_deref() == Some("after disposal"),
        "disposed custom view still reacted to the model",
    )?;
    host.set_content(None)?;
    Ok(())
}

fn exercise_generalized_form_grid(
    scope: &AppScope,
    refs: &MainWindowRefs,
    driver: &Grid,
) -> CheckResult<()> {
    let host = refs
        .custom_editor
        .get()
        .ok_or_else(|| std::io::Error::other("missing form grid host"))?;
    let lifetime = ReactiveScope::new();
    let context = components::ComponentContext {
        culture: Signal::new("en-US".to_string()),
        compact_height: 24.0,
        right_margin: 1.0,
    };
    let grid = lifetime.run(|| {
        components::finish_form_grid(components::form_grid())
            .provide_context(context.clone())
            .build(scope)
    })?;
    grid.set_row_definitions("Auto,Auto,Auto")?;
    grid.set_column_definitions("Auto,Auto,Auto")?;
    host.set_content(&grid)?;
    driver.update_layout()?;

    let first = TextBox::new()?;
    let label = TextBlock::new()?;
    label.set_text("Label")?;
    label.set_margin(Thickness::uniform(9.0))?;
    let original_padding = label.padding()?;
    let span = TextBox::new()?;
    let lower = TextBox::new()?;
    let last = TextBox::new()?;
    let checkbox = CheckBox::new()?;
    let marked = Button::new()?;
    marked.classes()?.add("form-grid-label-like")?;
    let shadowed = TextBlock::new()?;
    shadowed.classes()?.add("last-row-item")?;
    shadowed.classes()?.add("out-of-bounds")?;
    Grid::set_column(&label, 1)?;
    Grid::set_column(&span, 2)?;
    Grid::set_row_span(&span, 2)?;
    Grid::set_row(&lower, 1)?;
    Grid::set_column(&lower, 1)?;
    Grid::set_row(&last, 2)?;
    Grid::set_column(&last, 2)?;
    Grid::set_row(&checkbox, 2)?;
    Grid::set_row(&marked, 1)?;
    let controls: [&Control; 8] = [
        first.upcast_ref(),
        label.upcast_ref(),
        span.upcast_ref(),
        lower.upcast_ref(),
        last.upcast_ref(),
        checkbox.upcast_ref(),
        marked.upcast_ref(),
        shadowed.upcast_ref(),
    ];
    for control in controls {
        grid.children()?.add(control)?;
    }
    grid.update_layout()?;
    check(
        first.classes()?.contains("form-grid-item")?
            && !first.classes()?.contains("last-row-item")?
            && first.margin()? == Thickness::new(0.0, 0.0, 1.0, 5.0)
            && label.classes()?.contains("label-like-item")?
            && label.classes()?.contains("preceded-by-control")?
            && label.margin()? == Thickness::new(4.0, 0.0, 5.0, 5.0)
            && label.padding()? == Thickness::uniform(3.0),
        "multi-row form grid did not apply label/input metrics",
    )?;
    check(
        span.classes()?.contains("preceded-by-control")?
            && span.classes()?.contains("last-column-item")?
            && !span.classes()?.contains("last-row-item")?
            && last.classes()?.contains("last-row-item")?
            && checkbox.classes()?.contains("label-like-item")?
            && marked.classes()?.contains("label-like-item")?,
        "spanning controls or label-like children were classified incorrectly",
    )?;
    check(
        !shadowed.classes()?.contains("form-grid-item")?
            && shadowed.classes()?.contains("last-row-item")?
            && !shadowed.classes()?.contains("out-of-bounds")?,
        "shadowed child did not preserve its other classes",
    )?;

    grid.children()?.remove(1)?;
    check(
        !label.classes()?.contains("form-grid-item")?
            && !label.classes()?.contains("label-like-item")?
            && label.margin()? == Thickness::uniform(9.0)
            && label.padding()? == original_padding,
        "removing a child did not immediately restore its classes and sample styling",
    )?;
    grid.update_layout()?;
    Grid::set_row(&lower, 2)?;
    Grid::set_column(&lower, 0)?;
    grid.update_layout()?;
    check(
        !span.classes()?.contains("preceded-by-control")?,
        "moving a child did not update a row-spanning neighbor",
    )?;
    lower.set_visible(false)?;
    last.set_visible(false)?;
    checkbox.set_visible(false)?;
    grid.update_layout()?;
    check(
        span.classes()?.contains("last-row-item")?
            && !lower.classes()?.contains("form-grid-item")?
            && !last.classes()?.contains("form-grid-item")?
            && !checkbox.classes()?.contains("form-grid-item")?,
        "hidden trailing-row controls did not change the visible extent",
    )?;

    let partial = TextBox::new()?;
    Grid::set_column(&partial, 2)?;
    Grid::set_column_span(&partial, 2)?;
    grid.children()?.add(&partial)?;
    grid.update_layout()?;
    check(
        partial.classes()?.contains("out-of-bounds")?
            && !partial.classes()?.contains("form-grid-item")?,
        "a shadowed partial span did not report its out-of-bounds cell",
    )?;
    Grid::set_row(&partial, 2)?;
    grid.update_layout()?;
    check(
        partial.classes()?.contains("form-grid-item")?
            && partial.classes()?.contains("out-of-bounds")?
            && partial.classes()?.contains("last-row-item")?
            && !partial.classes()?.contains("last-column-item")?,
        "a moved partial span was not classified using raw endpoints",
    )?;
    lifetime.dispose();
    let partial_index = grid.children()?.len()? - 1;
    grid.children()?.remove(partial_index)?;
    check(
        partial.classes()?.contains("form-grid-item")?,
        "disposed grid still handled child removal",
    )?;
    let added_after_disposal = TextBox::new()?;
    grid.children()?.add(&added_after_disposal)?;
    grid.update_layout()?;
    check(
        !added_after_disposal.classes()?.contains("form-grid-item")?,
        "disposed grid observed newly added children",
    )?;
    host.set_content(None)?;

    let options = components::form_grid::FormGridOptions {
        treat_empty_cells_as_controls: true,
        apply_sample_styles: false,
        ..Default::default()
    };
    let lifetime = ReactiveScope::new();
    let grid = lifetime.run(|| {
        components::form_grid::finish_form_grid_with_options(components::form_grid(), options)
            .provide_context(context.clone())
            .build(scope)
    })?;
    grid.set_column_definitions("Auto,Auto,Auto")?;
    let label = Label::new()?;
    label.set_margin(Thickness::uniform(7.0))?;
    Grid::set_column(&label, 2)?;
    grid.children()?.add(&label)?;
    host.set_content(&grid)?;
    driver.update_layout()?;
    grid.update_layout()?;
    check(
        label.classes()?.contains("label-like-item")?
            && label.classes()?.contains("preceded-by-control")?
            && label.margin()? == Thickness::uniform(7.0),
        "empty-cell option or disabled sample styling was not respected",
    )?;
    grid.set_column_definitions("Auto,Auto")?;
    grid.update_layout()?;
    check(
        label.classes()?.contains("out-of-bounds")?
            && label.classes()?.contains("form-grid-item")?,
        "changed definitions did not retain the shadowed/out-of-bounds class behavior",
    )?;
    lifetime.dispose();
    host.set_content(None)?;

    let target_lifetime = ReactiveScope::new();
    let target = target_lifetime.run(|| {
        components::form_grid::finish_form_grid_with_options(
            components::form_grid(),
            components::form_grid::FormGridOptions {
                apply_sample_styles: false,
                ..Default::default()
            },
        )
        .provide_context(context.clone())
        .build(scope)
    })?;
    let source_lifetime = ReactiveScope::new();
    let source = source_lifetime.run(|| {
        components::finish_form_grid(components::form_grid())
            .provide_context(context)
            .build(scope)
    })?;
    let child = TextBox::new()?;
    child.set_margin(Thickness::uniform(9.0))?;
    let original_height = child.height()?;
    source.children()?.add(&child)?;
    let panel = StackPanel::new()?;
    panel.children()?.add(&source)?;
    panel.children()?.add(&target)?;
    host.set_content(&panel)?;
    driver.update_layout()?;
    check(
        child.classes()?.contains("form-grid-item")? && child.height()? == 24.0,
        "source grid did not classify and style its child",
    )?;

    source.children()?.remove(0)?;
    check(
        !child.classes()?.contains("form-grid-item")?
            && child.margin()? == Thickness::uniform(9.0)
            && (child.height()? == original_height
                || (child.height()?.is_nan() && original_height.is_nan())),
        "removing a child did not restore it before reparenting",
    )?;
    target.children()?.add(&child)?;
    driver.update_layout()?;
    source.update_layout()?;
    check(
        child.classes()?.contains("form-grid-item")?
            && child.margin()? == Thickness::uniform(9.0)
            && (child.height()? == original_height
                || (child.height()?.is_nan() && original_height.is_nan())),
        "the old grid disturbed a child after reparenting",
    )?;

    host.set_content(None)?;
    target.children()?.remove(0)?;
    check(
        !child.classes()?.contains("form-grid-item")?,
        "removing a child from an unattached grid did not clear its classes",
    )?;
    source.children()?.add(&child)?;
    host.set_content(&panel)?;
    driver.update_layout()?;
    check(
        child.classes()?.contains("form-grid-item")? && child.height()? == 24.0,
        "reparenting through an unattached grid did not reapply the new grid",
    )?;
    source_lifetime.dispose();
    target_lifetime.dispose();
    host.set_content(None)?;
    Ok(())
}

fn exercise_interactive(
    refs: &MainWindowRefs,
    state: &InteractiveState,
) -> CheckResult<InteractiveControls> {
    let title = refs
        .profile_title
        .get()
        .ok_or_else(|| std::io::Error::other("missing profile title"))?;
    let detail = refs
        .profile_detail
        .get()
        .ok_or_else(|| std::io::Error::other("missing profile detail"))?;
    let message = refs
        .profile_message
        .get()
        .ok_or_else(|| std::io::Error::other("missing profile message"))?;
    let number = refs
        .number_input
        .get()
        .ok_or_else(|| std::io::Error::other("missing number input"))?;
    let rdp = refs
        .rdp_port
        .get()
        .ok_or_else(|| std::io::Error::other("missing RDP button"))?;
    let ssh = refs
        .ssh_port
        .get()
        .ok_or_else(|| std::io::Error::other("missing SSH button"))?;
    let web = refs
        .web_ports
        .get()
        .ok_or_else(|| std::io::Error::other("missing web button"))?;
    let dynamic = refs
        .dynamic_port
        .get()
        .ok_or_else(|| std::io::Error::other("missing dynamic button"))?;
    let selected_port = refs
        .selected_port
        .get()
        .ok_or_else(|| std::io::Error::other("missing selected port"))?;

    check(text_value(&title)? == "First", "initial nested title")?;
    check(
        text_value(&message)? == "First message" && detail.is_visible()?,
        "initial converted message visibility",
    )?;
    state.first.message.set(String::new());
    check(!detail.is_visible()?, "empty message stayed visible")?;
    state.first.message.set("Restored".to_string());
    check(
        detail.is_visible()? && text_value(&message)? == "Restored",
        "message did not update reactively",
    )?;
    state.profile.set(Some(state.second.clone()));
    check(
        text_value(&title)? == "Second"
            && text_value(&message)? == "Second message"
            && detail.is_visible()?,
        "replacing an intermediate model did not update bindings",
    )?;
    let calls_before_old_changes = state.converters.calls.lock().unwrap().len();
    state.first.title.set("stale".to_string());
    state.first.message.set(String::new());
    state.first.port.set("stale".to_string());
    check(
        text_value(&title)? == "Second"
            && detail.is_visible()?
            && state.converters.calls.lock().unwrap().len() == calls_before_old_changes,
        "replaced model still had active subscribers",
    )?;
    state.second.title.set("Second updated".to_string());
    check(
        text_value(&title)? == "Second updated",
        "new intermediate model did not remain observable",
    )?;
    state.culture.set("fr-CA".to_string());
    check(
        state
            .converters
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|(id, _, _, _, culture)| *id == 1 && culture == "fr-CA"),
        "culture change did not re-evaluate the converter",
    )?;

    for (button, expected) in [
        (&rdp, "3389"),
        (&ssh, "22"),
        (&web, "80,443,8080,8443"),
        (&dynamic, "2200"),
    ] {
        let parameter = button.command_parameter()?;
        check(
            parameter == Variant::Utf16(expected.to_string()),
            "command parameter did not remain a string",
        )?;
        let command = button
            .command()?
            .ok_or_else(|| std::io::Error::other("button has no command"))?;
        check(
            command.can_execute(parameter.clone())?,
            "command rejected its button parameter",
        )?;
        command.execute(parameter)?;
        check(
            state.port_spec.get() == expected && text_value(&selected_port)? == expected,
            "button command did not receive its parameter",
        )?;
    }

    state.enabled.set(false);
    check(
        !state.original.can_execute("3389")?,
        "reactive CanExecute did not disable the command",
    )?;
    let queries_before_enabled = state.queries.lock().unwrap().len();
    state.enabled.set(true);
    check(
        state.original.can_execute("3389")?
            && state.queries.lock().unwrap()[queries_before_enabled..]
                .contains(&Variant::Utf16("2200".to_string())),
        "CanExecuteChanged did not re-query the attached buttons",
    )?;
    let queries_before_blocked = state.queries.lock().unwrap().len();
    state.second.port.set("blocked".to_string());
    let blocked_was_queried = state.queries.lock().unwrap()[queries_before_blocked..]
        .contains(&Variant::Utf16("blocked".to_string()));
    check(
        blocked_was_queried && !state.original.can_execute("blocked")?,
        "parameter change did not re-query CanExecute",
    )?;
    state.second.port.set("端口-😀".to_string());
    check(
        state.original.can_execute("端口-😀")?
            && dynamic.command_parameter()? == Variant::Utf16("端口-😀".to_string()),
        "dynamic Unicode parameter did not update",
    )?;

    let command = dynamic
        .command()?
        .ok_or_else(|| std::io::Error::other("dynamic button has no command"))?;
    for _ in 0..250 {
        check(
            command.can_execute("端口-😀")?,
            "Unicode parameter CanExecute failed",
        )?;
        command.execute("端口-😀")?;
        state.original.execute("直接-😀")?;
    }
    check(
        state.port_spec.get() == "直接-😀",
        "repeated Unicode parameter delivery failed",
    )?;
    state.command.set(Some(state.replacement.clone()));
    let replacement = rdp
        .command()?
        .ok_or_else(|| std::io::Error::other("replacement command missing"))?;
    replacement.execute(rdp.command_parameter()?)?;
    check(
        state.port_spec.get() == "replacement:3389",
        "reactive command replacement failed",
    )?;
    state.command.set(None);
    check(rdp.command()?.is_none(), "command was not cleared")?;
    state.command.set(Some(state.original.clone()));
    let failing = Command::new(|_| Err(Error::Binding("command failed".to_string())));
    check(
        failing.execute("échec-😀").is_err(),
        "command error was swallowed",
    )?;

    check(number.text()?.as_deref() == Some("12"), "initial Convert")?;
    let text_changed_count = Arc::new(AtomicUsize::new(0));
    let text_changed = text_changed_count.clone();
    let text_changed_subscription = number.subscribe_text_changed(move |_| {
        text_changed.fetch_add(1, Ordering::Relaxed);
    })?;
    number.set_text("42")?;
    check(
        state.number.get() == 42,
        "ConvertBack did not update the model",
    )?;
    number.set_text("invalid")?;
    check(
        state.number.get() == 42,
        "invalid conversion changed the model",
    )?;
    let convert_back_calls = state
        .converters
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|(id, direction, _, _, _)| {
            *id == 2 && *direction == ConversionDirection::ConvertBack
        })
        .count();
    state.number.set(17);
    check(
        number.text()?.as_deref() == Some("17")
            && state
                .converters
                .calls
                .lock()
                .unwrap()
                .iter()
                .filter(|(id, direction, _, _, _)| {
                    *id == 2 && *direction == ConversionDirection::ConvertBack
                })
                .count()
                == convert_back_calls,
        "model update fed back through ConvertBack or failed after validation",
    )?;
    check(
        state.converters.calls.lock().unwrap().iter().any(
            |(id, direction, target, parameter, culture)| {
                *id == 2
                    && *direction == ConversionDirection::ConvertBack
                    && *target == ScalarKind::Int64
                    && *parameter == ScalarValue::String("roundtrip".to_string())
                    && culture == "fr-FR"
            },
        ),
        "ConvertBack lost its source type, parameter, or culture",
    )?;

    Ok(InteractiveControls {
        title,
        detail,
        message,
        number,
        text_changed_count,
        text_changed_subscription,
        convert_back_calls,
    })
}

fn exercise(scope: &AppScope) -> CheckResult<DeferredChecks> {
    let lifetime = ReactiveScope::new();
    let (
        window,
        instruction,
        resources,
        strings,
        contacts,
        tick,
        removed,
        refs,
        item_names,
        interactive,
        interactive_controls,
    ) = lifetime.run(|| -> CheckResult<_> {
        let resources = ResourceScope::new();
        let light_brush = Some(Brush::solid(Color::rgb(20, 30, 40)));
        let dark_brush = Some(Brush::solid(Color::rgb(200, 210, 220)));
        let light = light_brush.map(rustolonia::Paint::from);
        let dark = dark_brush.map(rustolonia::Paint::from);
        let dictionary = resources.dictionary()?;
        dictionary.insert_for_theme(ThemeVariant::Default, "WindowBackground", light_brush)?;
        dictionary.insert_for_theme(ThemeVariant::Dark, "WindowBackground", dark_brush)?;
        resources.merge(&dictionary)?;

        let strings = LocalizationResources::new("en-US")?;
        strings.insert("en-US", "GreetingInstruction", "Enter your name")?;
        strings.insert("fr-FR", "GreetingInstruction", "Entrez votre nom")?;
        let child_slot = ControlSlot::new();
        let first_marker = Rc::new(());
        let removed_marker = Rc::new(());
        let removed_weak = Rc::downgrade(&removed_marker);
        let contacts = Signal::new(vec![
            Contact {
                id: 1,
                name: "One".into(),
                kind: 0,
                marker: first_marker.clone(),
            },
            Contact {
                id: 2,
                name: "Two".into(),
                kind: 0,
                marker: removed_marker.clone(),
            },
        ]);
        let editable_name = Signal::new("Editable item".to_string());
        let editable_contacts = Signal::new(vec![EditableContact {
            id: 1,
            name: editable_name,
        }]);
        let tick = Signal::new(0);
        let views = ViewRegistry::new(|contact: &Contact| contact.kind);
        views.register(0, move |contact: Signal<Contact>| {
            TextBlock::el().text(move || format!("A:{}:{}", contact.get().name, tick.get()))
        })?;
        views.register(1, move |contact: Signal<Contact>| {
            TextBlock::el().text(move || format!("B:{}:{}", contact.get().name, tick.get()))
        })?;
        let interactive = InteractiveState::new()?;
        let profile = interactive.profile;
        let number = interactive.number;
        let bindings = MainWindowBindings {
            name: Signal::new(String::new()),
            greeting: Signal::new(String::new()),
            resource_greeting_instruction: strings.string_signal("GreetingInstruction")?,
            resource_window_background: resources.dynamic("WindowBackground")?,
            optional_flag: Signal::new(None::<bool>),
            child_view: child_slot,
            contacts,
            editable_contacts,
            recent_contacts: ItemsView::from_registry(contacts, |contact| contact.id, views),
            contacts_item_refs_1: ItemNameScope::default(),
            profile_title: PropertyPath::read_only(move || {
                Ok(InteractiveState::current(profile)?.title.get())
            }),
            profile_message: PropertyPath::read_only(move || {
                Ok(InteractiveState::current(profile)?.message.get())
            }),
            set_ports_command: interactive.command,
            profile_port_command_parameter: PropertyPath::read_only(move || {
                Ok(Variant::Utf16(
                    InteractiveState::current(profile)?.port.get(),
                ))
            }),
            port_spec: interactive.port_spec,
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
                2,
                interactive.converters.clone(),
                interactive.culture,
            )?,
        };
        let handlers = MainWindowHandlers {
            on_greet: Box::new(|_| {}),
            on_contact_selection_changed: Box::new(|_| {}),
        };
        let (window, refs) = build_main_window(&bindings, handlers);
        let window = window.build(scope)?;
        let _custom_static_probe = Border::el()
            .visible(rustolonia::ConvertedBinding::<bool>::new(
                ScalarPath::new(
                    ScalarKind::String,
                    PropertyPath::read_only(move || {
                        Ok(ScalarValue::String(
                            InteractiveState::current(profile)?.message.get(),
                        ))
                    }),
                )?,
                RegisteredValueConverter::new(
                    "x:Static CustomConverters.HasText",
                    1,
                    interactive.converters.clone(),
                    interactive.culture,
                )?,
                "x:Static CustomConverters.HasText",
                ScalarValue::Null,
                None,
            ))
            .build(scope)?;
        let instruction = refs
            .instruction
            .get()
            .ok_or_else(|| std::io::Error::other("missing instruction"))?;
        let surface = refs
            .surface
            .get()
            .ok_or_else(|| std::io::Error::other("missing surface"))?;
        let static_swatch = refs
            .static_swatch
            .get()
            .ok_or_else(|| std::io::Error::other("missing static swatch"))?;
        let optional_flag = refs
            .optional_flag
            .get()
            .ok_or_else(|| std::io::Error::other("missing optional flag"))?;
        let name_input = refs
            .name_input
            .get()
            .ok_or_else(|| std::io::Error::other("missing name input"))?;
        let editable_list = refs
            .editable_contacts_list
            .get()
            .ok_or_else(|| std::io::Error::other("missing editable contacts list"))?;
        let editable_input: TextBox = cast(
            item(&editable_list, 0)?,
            "editable item template is not a TextBox",
        )?;
        check(
            editable_input.text()?.as_deref() == Some("Editable item"),
            "two-way item binding did not read its initial signal",
        )?;
        editable_input.set_text("Edited item")?;
        check(
            editable_name.get() == "Edited item",
            "two-way item binding did not update its model signal",
        )?;
        editable_name.set("Updated from Rust".to_string());
        check(
            editable_input.text()?.as_deref() == Some("Updated from Rust"),
            "two-way item binding did not observe its model signal",
        )?;
        bindings.name.set("Initial name".to_string());
        check(
            name_input.text()?.as_deref() == Some("Initial name"),
            "model did not update two-way text",
        )?;
        name_input.set_text("Edited name")?;
        check(
            bindings.name.get() == "Edited name",
            "two-way text did not update its model",
        )?;
        check(
            optional_flag.is_checked()?.is_none(),
            "initial nullable binding",
        )?;
        optional_flag.set_checked(Some(true))?;
        check(
            bindings.optional_flag.get() == Some(true),
            "nullable binding update",
        )?;
        optional_flag.set_checked(None)?;
        check(
            bindings.optional_flag.get().is_none(),
            "nullable binding lost null",
        )?;
        check(
            text_value(&instruction)? == "Enter your name",
            "initial localization",
        )?;
        check(
            instruction.name()?.as_deref() == Some("instruction"),
            "x:Name was not assigned to the control",
        )?;
        check(surface.background()? == light, "initial dynamic resource")?;
        check(
            static_swatch.background()? == light,
            "initial static resource",
        )?;
        strings.set_culture("fr-FR")?;
        resources.set_theme(ThemeVariant::Dark)?;
        check(
            text_value(&instruction)? == "Entrez votre nom",
            "culture update",
        )?;
        check(surface.background()? == dark, "theme update")?;
        check(
            static_swatch.background()? == light,
            "static resource changed",
        )?;

        let model = Signal::new(Some(ChildModel {
            kind: 0,
            label: "Initial".into(),
        }));
        let child_views = ViewRegistry::new(|child: &ChildModel| child.kind);
        child_views.register(0, |child: Signal<ChildModel>| {
            TextBlock::el().text(move || format!("first:{}", child.get().label))
        })?;
        child_views.register(1, |child: Signal<ChildModel>| {
            TextBlock::el().text(move || format!("second:{}", child.get().label))
        })?;
        child_slot.bind(scope, model, child_views)?;
        let host = child_slot
            .host_control()
            .ok_or_else(|| std::io::Error::other("content slot was not mounted"))?;
        let first = child_text(&host)?;
        let first_id = first.object_id()?;
        check(text_value(&first)? == "first:Initial", "initial content")?;
        model.set(Some(ChildModel {
            kind: 0,
            label: "Edited".into(),
        }));
        check(
            child_text(&host)?.object_id()? == first_id,
            "same content kind rebuilt",
        )?;
        check(
            text_value(&first)? == "first:Edited",
            "content model update",
        )?;
        model.set(Some(ChildModel {
            kind: 1,
            label: "Replacement".into(),
        }));
        let replacement = child_text(&host)?;
        check(
            replacement.object_id()? != first_id,
            "content kind was not replaced",
        )?;
        check(
            text_value(&replacement)? == "second:Replacement",
            "replacement content",
        )?;
        model.set(None);
        check(host.child()?.is_none(), "content was not cleared")?;

        let checked = Signal::new(None::<bool>);
        let checked_ref = NodeRef::<CheckBox>::new();
        child_slot.mount(
            scope,
            CheckBox::el()
                .node_ref(checked_ref)
                .three_state(true)
                .checked(checked.two_way()),
        )?;
        let check_box = checked_ref
            .get()
            .ok_or_else(|| std::io::Error::other("missing scoped checkbox"))?;
        check_box.set_checked(Some(true))?;
        check(checked.get() == Some(true), "view event was not attached")?;
        child_slot.clear()?;
        check(
            checked_ref.get().is_none(),
            "disposed view retained its checkbox",
        )?;
        check_box.set_checked(None)?;
        check(
            checked.get() == Some(true),
            "disposed view still handled control events",
        )?;

        model.set(Some(ChildModel {
            kind: 0,
            label: "Before clear".into(),
        }));
        child_slot.clear()?;
        model.set(Some(ChildModel {
            kind: 0,
            label: "After clear".into(),
        }));
        check(
            text_value(&child_text(&host)?)? == "first:After clear",
            "bound content did not remount after a manual clear",
        )?;
        model.set(None);

        let late_scope = ReactiveScope::new();
        let late_slot = ControlSlot::new();
        let _late_host = late_scope.run(|| late_slot.host().build(scope))?;
        check(
            matches!(
                late_scope.run(|| late_slot.host().build(scope)),
                Err(Error::ControlSlotAlreadyMounted)
            ),
            "a slot was mounted in two hosts",
        )?;
        let late_checked = Signal::new(None::<bool>);
        let late_ref = NodeRef::<CheckBox>::new();
        late_slot.mount(
            scope,
            CheckBox::el()
                .node_ref(late_ref)
                .three_state(true)
                .checked(late_checked.two_way()),
        )?;
        let late_checkbox = late_ref
            .get()
            .ok_or_else(|| std::io::Error::other("missing late-mounted checkbox"))?;
        late_checkbox.set_checked(Some(true))?;
        check(
            late_checked.get() == Some(true),
            "late event was not attached",
        )?;
        late_scope.dispose();
        check(
            late_slot.host_control().is_none() && late_ref.get().is_none(),
            "disposing the slot owner retained a late-mounted view",
        )?;
        late_checkbox.set_checked(None)?;
        check(
            late_checked.get() == Some(true),
            "disposed slot owner still handled late-mounted events",
        )?;
        let _remounted_host = late_slot.host().build(scope)?;

        let bound_scope = ReactiveScope::new();
        let bound_slot = ControlSlot::new();
        let _bound_host = bound_scope.run(|| bound_slot.host().build(scope))?;
        let selection_count = Rc::new(Cell::new(0));
        let selections = selection_count.clone();
        let bound_views = ViewRegistry::new(move |_: &ChildModel| {
            selections.set(selections.get() + 1);
            0
        });
        bound_views.register(0, |child: Signal<ChildModel>| {
            TextBlock::el().text(move || child.get().label)
        })?;
        let bound_model = Signal::new(Some(ChildModel {
            kind: 0,
            label: "Bound outside owner".into(),
        }));
        bound_slot.bind(scope, bound_model, bound_views)?;
        let selections_before_disposal = selection_count.get();
        bound_scope.dispose();
        bound_model.set(Some(ChildModel {
            kind: 0,
            label: "After owner disposal".into(),
        }));
        check(
            selection_count.get() == selections_before_disposal,
            "slot binding outlived its owner",
        )?;

        let inline = refs
            .contacts_list
            .get()
            .ok_or_else(|| std::io::Error::other("missing inline list"))?;
        let recent = refs
            .recent_list
            .get()
            .ok_or_else(|| std::io::Error::other("missing registry list"))?;
        check(inline.items()?.len()? == 2, "initial inline item count")?;
        check(recent.items()?.len()? == 2, "initial registry item count")?;
        let inline_one = direct_item(&inline, 0)?;
        let inline_two = direct_item(&inline, 1)?;
        let item_names = bindings.contacts_item_refs_1.clone();
        let named_one = named_item(&item_names, 1)?;
        let named_two = named_item(&item_names, 2)?;
        check(item_names.entries().len() == 2, "initial named item count")?;
        check(
            named_item_id(named_one)? == inline_one.object_id()?,
            "first item name resolved to the wrong control",
        )?;
        check(
            named_item_id(named_two)? == inline_two.object_id()?,
            "second item name resolved to the wrong control",
        )?;
        check(
            inline_one.name()?.as_deref() == Some("contact_name")
                && inline_two.name()?.as_deref() == Some("contact_name"),
            "item control names were not set",
        )?;
        let (outer_one, _) = registry_item(&recent, 0)?;
        let (outer_two, removed) = registry_item(&recent, 1)?;
        check(
            text_value(&inline_one)? == "One",
            "first inline item context",
        )?;
        check(
            text_value(&inline_two)? == "Two",
            "second inline item context",
        )?;
        check(
            text_value(&removed)? == "A:Two:0",
            "second registry item context",
        )?;
        let inline_one_id = inline_one.object_id()?;
        let outer_one_id = outer_one.object_id()?;
        let outer_two_id = outer_two.object_id()?;

        let third_marker = Rc::new(());
        contacts.set(vec![
            Contact {
                id: 2,
                name: "Two edited".into(),
                kind: 0,
                marker: removed_marker.clone(),
            },
            Contact {
                id: 1,
                name: "One".into(),
                kind: 0,
                marker: first_marker.clone(),
            },
            Contact {
                id: 3,
                name: "Three".into(),
                kind: 1,
                marker: third_marker.clone(),
            },
        ]);
        check(inline.items()?.len()? == 3, "inline item addition")?;
        check(recent.items()?.len()? == 3, "registry item addition")?;
        check(item_names.entries().len() == 3, "named item addition")?;
        check(
            direct_item(&inline, 1)?.object_id()? == inline_one_id,
            "inline item identity",
        )?;
        check(
            named_item_id(named_item(&item_names, 1)?)? == inline_one_id,
            "named item identity changed after reordering",
        )?;
        check(
            text_value(&direct_item(&inline, 0)?)? == "Two edited",
            "inline item update",
        )?;
        let (reordered_two, updated_two) = registry_item(&recent, 0)?;
        let (reordered_one, _) = registry_item(&recent, 1)?;
        check(
            reordered_two.object_id()? == outer_two_id,
            "registry reorder for item two",
        )?;
        check(
            reordered_one.object_id()? == outer_one_id,
            "registry reorder for item one",
        )?;
        check(
            text_value(&updated_two)? == "A:Two edited:0",
            "registry item update",
        )?;
        check(
            text_value(&registry_item(&recent, 2)?.1)? == "B:Three:0",
            "registry selection",
        )?;

        contacts.set(vec![
            Contact {
                id: 1,
                name: "One replaced".into(),
                kind: 1,
                marker: first_marker,
            },
            Contact {
                id: 3,
                name: "Three".into(),
                kind: 1,
                marker: third_marker,
            },
        ]);
        check(
            item_names.entries().len() == 2 && named_two.get().is_none(),
            "removed item retained its named references",
        )?;
        let (same_outer, replacement) = registry_item(&recent, 0)?;
        check(
            same_outer.object_id()? == outer_one_id,
            "registry item was not reused",
        )?;
        check(
            text_value(&replacement)? == "B:One replaced:0",
            "registry view replacement",
        )?;
        drop(removed_marker);
        check(
            removed_weak.upgrade().is_none(),
            "removed item model was retained",
        )?;
        tick.set(1);
        check(
            text_value(&removed)? == "A:Two edited:0",
            "removed view still updated",
        )?;
        check(
            text_value(&replacement)? == "B:One replaced:1",
            "active view did not update",
        )?;

        let duplicate_marker = Rc::new(());
        let duplicates = Signal::new(vec![
            Contact {
                id: 9,
                name: "First".into(),
                kind: 0,
                marker: duplicate_marker.clone(),
            },
            Contact {
                id: 9,
                name: "Second".into(),
                kind: 0,
                marker: duplicate_marker,
            },
        ]);
        let invalid = ItemsView::new(duplicates, |contact| contact.id, |_| TextBlock::el());
        let detached = ItemsControl::new()?;
        check(
            matches!(
                invalid.attach(scope, &detached),
                Err(Error::DuplicateItemKey)
            ),
            "duplicate item keys were accepted",
        )?;
        check(
            detached.items()?.len()? == 0,
            "duplicate items changed the collection",
        )?;
        scope.mount(window.clone())?;
        let interactive_controls = exercise_interactive(&refs, &interactive)?;
        Ok((
            window,
            instruction,
            resources,
            strings,
            contacts,
            tick,
            removed,
            refs,
            item_names,
            interactive,
            interactive_controls,
        ))
    })?;

    DEFERRED_INPUT.with(|input| {
        *input.borrow_mut() = Some((interactive_controls.number.clone(), interactive.number))
    });
    let component_scope = scope.clone();
    Ok(Box::new(move || {
        // Keep the subscription alive until the queued events have fired.
        let _text_changed_subscription = interactive_controls.text_changed_subscription;
        let text_changed_count = interactive_controls
            .text_changed_count
            .load(Ordering::Relaxed);
        check(
            text_changed_count >= 6,
            &format!("expected six deferred TextChanged events, observed {text_changed_count}"),
        )?;
        let surface = refs
            .surface
            .get()
            .ok_or_else(|| std::io::Error::other("missing form grid surface"))?;
        let scroll: ScrollViewer = cast(
            surface
                .child()?
                .ok_or_else(|| std::io::Error::other("missing form grid scroll viewer"))?,
            "form grid host is not a ScrollViewer",
        )?;
        let panel: StackPanel = cast(
            scroll
                .content()?
                .ok_or_else(|| std::io::Error::other("missing form grid panel"))?,
            "form grid panel is not a StackPanel",
        )?;
        let driver = Grid::new()?;
        let driver_index = panel.children()?.len()?;
        panel.children()?.add(&driver)?;
        driver.update_layout()?;
        exercise_custom_editor(&component_scope, &refs, &driver)
            .map_err(|error| std::io::Error::other(format!("custom editor: {error}")))?;
        exercise_generalized_form_grid(&component_scope, &refs, &driver)
            .map_err(|error| std::io::Error::other(format!("generalized grid: {error}")))?;
        panel.children()?.remove(driver_index)?;
        check(
            interactive.number.get() == 17
                && interactive_controls.number.text()?.as_deref() == Some("17")
                && interactive
                    .converters
                    .calls
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|(id, direction, _, _, _)| {
                        *id == 2 && *direction == ConversionDirection::ConvertBack
                    })
                    .count()
                    == interactive_controls.convert_back_calls + 1,
            "model updates fed back through ConvertBack across dispatcher turns",
        )?;
        lifetime.dispose();
        check(
            refs.surface.get().is_none(),
            "disposed view retained its named controls",
        )?;
        check(
            refs.recent_list.get().is_none(),
            "disposed view retained its list",
        )?;
        check(
            item_names.entries().is_empty(),
            "disposed view retained its item names",
        )?;
        let converter_calls = interactive.converters.calls.lock().unwrap().len();
        interactive.second.title.set("after disposal".to_string());
        interactive.second.message.set(String::new());
        interactive.number.set(99);
        interactive.culture.set("de-DE".to_string());
        interactive
            .command
            .set(Some(interactive.replacement.clone()));
        check(
            text_value(&interactive_controls.title)? == "Second updated"
                && text_value(&interactive_controls.message)? == "Second message"
                && interactive_controls.detail.is_visible()?
                && interactive_controls.number.text()?.as_deref() == Some("17")
                && interactive.converters.calls.lock().unwrap().len() == converter_calls,
            "disposed interactive view still reacted to model or culture changes",
        )?;
        tick.set(2);
        check(
            text_value(&removed)? == "A:Two edited:0",
            "disposed row still updated",
        )?;
        contacts.set(Vec::new());
        resources.remove_for_theme(ThemeVariant::Default, "WindowBackground")?;
        resources.remove_for_theme(ThemeVariant::Dark, "WindowBackground")?;
        strings.set_culture("en-US")?;
        Ok((window, instruction))
    }))
}

fn main() -> CheckResult<()> {
    let outcome = Arc::new(Mutex::new(None::<std::result::Result<(), String>>));
    let result = outcome.clone();
    let app = App::load_from_env()?;
    app.without_startup_arguments().run(move |scope| {
        match exercise(scope) {
            Ok(finish) => {
                DEFERRED_CHECKS.with(|checks| *checks.borrow_mut() = Some(finish));
                let result = result.clone();
                let shutdown = scope.clone();
                scope.post(move || {
                    let stage = (|| -> CheckResult<()> {
                        let (input, number) = DEFERRED_INPUT
                            .with(|input| input.borrow_mut().take())
                            .ok_or_else(|| std::io::Error::other("missing deferred input"))?;
                        input.set_text("43")?;
                        check(number.get() == 43, "deferred edit did not update the model")?;
                        number.set(18);
                        number.set(17);
                        Ok(())
                    })();
                    if let Err(error) = stage {
                        *result.lock().unwrap() = Some(Err(error.to_string()));
                        if let Err(error) = shutdown.shutdown() {
                            eprintln!("native fixture shutdown failed: {error}");
                        }
                        return;
                    }
                    let later_shutdown = shutdown.clone();
                    let final_result = result.clone();
                    if let Err(error) = shutdown.post(move || {
                        let checked = (|| -> CheckResult<()> {
                            let finish = DEFERRED_CHECKS
                                .with(|checks| checks.borrow_mut().take())
                                .ok_or_else(|| std::io::Error::other("missing deferred checks"))?;
                            let (window, instruction) = finish()?;
                            check(window.is_visible()?, "window is not visible")?;
                            check(instruction.is_visible()?, "instruction is not visible")
                        })();
                        *final_result.lock().unwrap() =
                            Some(checked.map_err(|error| error.to_string()));
                        if let Err(error) = later_shutdown.shutdown() {
                            eprintln!("native fixture shutdown failed: {error}");
                        }
                    }) {
                        *result.lock().unwrap() =
                            Some(Err(format!("native fixture deferred post failed: {error}")));
                        if let Err(error) = shutdown.shutdown() {
                            eprintln!("native fixture shutdown failed: {error}");
                        }
                    }
                })?;
            }
            Err(error) => {
                *result.lock().unwrap() = Some(Err(error.to_string()));
                let shutdown = scope.clone();
                scope.post(move || {
                    if let Err(error) = shutdown.shutdown() {
                        eprintln!("native fixture shutdown failed: {error}");
                    }
                })?;
            }
        }
        Ok(())
    })?;
    let result = outcome
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| std::io::Error::other("native fixture never reached the UI"))?;
    result.map_err(std::io::Error::other)?;
    println!("AXAML native fixture passed");
    Ok(())
}
