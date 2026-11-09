use rustolonia::{use_context, ComboBox, Effect, El, Error, Result, Signal, TextBlock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[path = "form_grid.rs"]
pub mod form_grid;

pub use form_grid::{finish_form_grid, form_grid};

#[derive(Clone)]
pub struct ComponentContext {
    pub culture: Signal<String>,
    pub compact_height: f64,
    pub right_margin: f64,
}

pub trait PickerValue: Copy + Eq + Send + 'static {
    fn values() -> &'static [Self];
    fn label(self, culture: &str) -> &'static str;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterKind {
    Name,
    Owner,
    Category,
}

impl PickerValue for FilterKind {
    fn values() -> &'static [Self] {
        &[Self::Name, Self::Owner, Self::Category]
    }

    fn label(self, culture: &str) -> &'static str {
        match (self, culture) {
            (Self::Name, "fr-FR") => "Nom",
            (Self::Owner, "fr-FR") => "Propriétaire",
            (Self::Category, "fr-FR") => "Catégorie",
            (Self::Name, _) => "Name",
            (Self::Owner, _) => "Owner",
            (Self::Category, _) => "Category",
        }
    }
}

pub struct EnumPickerBuilder<T: PickerValue> {
    selected: Option<Signal<T>>,
    included: Option<Signal<Vec<T>>>,
}

pub fn enum_picker<T: PickerValue>() -> EnumPickerBuilder<T> {
    EnumPickerBuilder {
        selected: None,
        included: None,
    }
}

struct Updating(Arc<AtomicBool>);

impl Updating {
    fn new(flag: Arc<AtomicBool>) -> Self {
        flag.store(true, Ordering::Release);
        Self(flag)
    }
}

impl Drop for Updating {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl<T: PickerValue> EnumPickerBuilder<T> {
    pub fn selected_value(mut self, selected: Signal<T>) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn included_values(mut self, included: Signal<Vec<T>>) -> Self {
        self.included = Some(included);
        self
    }

    pub fn into_el(self) -> El<ComboBox> {
        ComboBox::el().with_op(move |scope, combo| {
            let selected = self
                .selected
                .ok_or_else(|| Error::Binding("EnumPicker requires SelectedValue".into()))?;
            let included = self
                .included
                .ok_or_else(|| Error::Binding("EnumPicker requires IncludedValues".into()))?;
            let culture = use_context::<ComponentContext>()
                .ok_or_else(|| Error::Binding("EnumPicker requires ComponentContext".into()))?
                .culture;
            let updating = Arc::new(AtomicBool::new(false));
            let visible = Arc::new(Mutex::new(Vec::<T>::new()));

            let callback_combo = combo.clone();
            let callback_visible = visible.clone();
            let callback_updating = updating.clone();
            let subscription = combo.subscribe_selection_changed(move |_| {
                if callback_updating.load(Ordering::Acquire) {
                    return;
                }
                let result = (|| -> Result<()> {
                    let index = callback_combo.get_selected_index()?;
                    if index == -1 {
                        return Ok(());
                    }
                    let choices = callback_visible
                        .lock()
                        .map_err(|_| Error::Binding("EnumPicker choices lock poisoned".into()))?;
                    let value = usize::try_from(index)
                        .ok()
                        .and_then(|index| choices.get(index))
                        .copied()
                        .ok_or_else(|| {
                            Error::Binding("EnumPicker selected an invalid index".into())
                        })?;
                    drop(choices);
                    selected.set_if_changed(value);
                    Ok(())
                })();
                if let Err(error) = result {
                    eprintln!("EnumPicker selection update failed: {error}");
                }
            })?;
            scope.retain_subscription(subscription);

            let combo = combo.clone();
            let mut last_culture = None::<String>;
            Effect::new(move || {
                let requested = included.get();
                let selected_value = selected.get();
                let current_culture = culture.get();
                let all = T::values();
                if all.is_empty()
                    || all
                        .iter()
                        .enumerate()
                        .any(|(index, value)| all[..index].contains(value))
                    || requested.iter().any(|value| !all.contains(value))
                {
                    return Err(Error::Binding("EnumPicker received invalid choices".into()));
                }
                let choices: Vec<_> = all
                    .iter()
                    .copied()
                    .filter(|value| requested.is_empty() || requested.contains(value))
                    .collect();
                let needs_rebuild = {
                    let old = visible
                        .lock()
                        .map_err(|_| Error::Binding("EnumPicker choices lock poisoned".into()))?;
                    *old != choices || last_culture.as_deref() != Some(&current_culture)
                };
                let _guard = Updating::new(updating.clone());
                if needs_rebuild {
                    let items = combo.items()?;
                    items.clear()?;
                    for value in &choices {
                        items.add(TextBlock::new()?.text(value.label(&current_culture))?)?;
                    }
                    *visible
                        .lock()
                        .map_err(|_| Error::Binding("EnumPicker choices lock poisoned".into()))? =
                        choices.clone();
                    last_culture = Some(current_culture);
                }
                let index = match choices.iter().position(|value| *value == selected_value) {
                    Some(index) => i32::try_from(index)
                        .map_err(|_| Error::Binding("EnumPicker has too many choices".into()))?,
                    None => -1,
                };
                if combo.get_selected_index()? != index {
                    combo.set_selected_index(index)?;
                }
                Ok(())
            })?;
            Ok(())
        })
    }
}
