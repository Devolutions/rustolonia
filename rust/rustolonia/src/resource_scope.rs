//! Rust-owned, typed resource dictionaries for code-first and AXAML views.

use crate::reactive::{on_scope_cleanup, RawSignalId};
use crate::{Error, Result, Signal, ThemeVariant};
use std::any::{type_name, Any};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

type ResourceObject = dyn Any + Send + Sync;
type PendingUpdate = Box<dyn FnOnce() + Send>;

struct StoredValue {
    value: Arc<ResourceObject>,
    type_name: &'static str,
}

impl StoredValue {
    fn new<T: Send + Sync + 'static>(value: T) -> Self {
        Self {
            value: Arc::new(value),
            type_name: type_name::<T>(),
        }
    }

    fn typed<T: Send + Sync + 'static>(&self, key: &str) -> Result<Arc<T>> {
        self.value
            .clone()
            .downcast::<T>()
            .map_err(|_| Error::ResourceTypeMismatch {
                key: key.to_string(),
                expected: type_name::<T>(),
                actual: self.type_name,
            })
    }
}

#[derive(Default)]
struct Dictionary {
    parent: Option<u64>,
    merged: Vec<u64>,
    direct: HashMap<String, StoredValue>,
    themed: HashMap<ThemeVariant, HashMap<String, StoredValue>>,
    incoming: usize,
    has_handle: bool,
}

trait ResourceWatch: Send {
    fn id(&self) -> u64;
    fn scope_id(&self) -> u64;
    fn key(&self) -> &str;
    fn prepare(&self, value: &StoredValue) -> Result<PendingUpdate>;
}

struct TypedWatch<T: 'static> {
    id: u64,
    scope_id: u64,
    key: String,
    signal: RawSignalId,
    active: Arc<AtomicBool>,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T: 'static> Drop for TypedWatch<T> {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
    }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> ResourceWatch for TypedWatch<T> {
    fn id(&self) -> u64 {
        self.id
    }

    fn scope_id(&self) -> u64 {
        self.scope_id
    }

    fn key(&self) -> &str {
        &self.key
    }

    fn prepare(&self, value: &StoredValue) -> Result<PendingUpdate> {
        let value = value.typed::<T>(&self.key)?;
        let raw_signal = self.signal;
        let active = self.active.clone();
        Ok(Box::new(move || {
            if active.load(Ordering::Acquire) {
                if let Ok(signal) = Signal::<T>::try_from_raw(raw_signal) {
                    if signal.is_alive() {
                        signal.set_if_changed((*value).clone());
                    }
                }
            }
        }))
    }
}

struct ResourceRegistry {
    next_id: u64,
    next_watch_id: u64,
    dictionaries: HashMap<u64, Dictionary>,
    watches: Vec<Box<dyn ResourceWatch>>,
    theme: ThemeVariant,
    theme_signal: RawSignalId,
    revision: u64,
}

struct ScopeLease {
    registry: Arc<Mutex<ResourceRegistry>>,
    id: u64,
}

impl Drop for ScopeLease {
    fn drop(&mut self) {
        let removed = {
            let mut registry = match self.registry.lock() {
                Ok(registry) => registry,
                Err(poisoned) => {
                    eprintln!("resource scope registry lock is poisoned during cleanup");
                    poisoned.into_inner()
                }
            };
            registry.watches.retain(|watch| watch.scope_id() != self.id);
            if let Some(dictionary) = registry.dictionaries.get_mut(&self.id) {
                dictionary.has_handle = false;
            }
            let mut removed = Vec::new();
            collect_unused_dictionaries(&mut registry, self.id, &mut removed);
            removed
        };
        drop(removed);
    }
}

/// A Rust-owned resource dictionary and its place in a resource hierarchy.
///
/// Lookups check the local dictionary, its theme dictionaries and merged
/// dictionaries, then the parent scope. Direct entries take precedence over
/// themed entries; merged dictionaries are searched in reverse insertion
/// order. Scopes and their signals must be used on the Avalonia UI thread.
///
/// Dynamic signals are active while their originating resource scope is retained
/// and until the reactive scope in which they were created is disposed. Use
/// [`crate::AppScope::retain_resource_scope`] for an application-wide dictionary
/// or keep a child scope with the view that owns it.
#[derive(Clone)]
pub struct ResourceScope {
    lease: Arc<ScopeLease>,
}

impl fmt::Debug for ResourceScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResourceScope")
            .field("id", &self.lease.id)
            .finish_non_exhaustive()
    }
}

impl Default for ResourceScope {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceScope {
    /// Creates an application resource scope with the default theme.
    pub fn new() -> Self {
        let mut dictionaries = HashMap::new();
        dictionaries.insert(
            0,
            Dictionary {
                has_handle: true,
                ..Dictionary::default()
            },
        );
        Self {
            lease: Arc::new(ScopeLease {
                registry: Arc::new(Mutex::new(ResourceRegistry {
                    next_id: 1,
                    next_watch_id: 0,
                    dictionaries,
                    watches: Vec::new(),
                    theme: ThemeVariant::Default,
                    theme_signal: Signal::new(ThemeVariant::Default).raw_id(),
                    revision: 0,
                })),
                id: 0,
            }),
        }
    }

    /// Creates a local scope that falls back to this scope and its ancestors.
    pub fn child(&self) -> Result<Self> {
        self.new_dictionary(Some(self.lease.id))
    }

    /// Creates an independent dictionary in the same resource registry.
    ///
    /// Add it to another scope with [`ResourceScope::merge`].
    pub fn dictionary(&self) -> Result<Self> {
        self.new_dictionary(None)
    }

    fn new_dictionary(&self, parent: Option<u64>) -> Result<Self> {
        let id = {
            let mut registry = self.lock()?;
            let id = registry.next_id;
            registry.next_id += 1;
            if let Some(parent_id) = parent {
                dictionary_mut(&mut registry, parent_id)?.incoming += 1;
            }
            registry.dictionaries.insert(
                id,
                Dictionary {
                    parent,
                    has_handle: true,
                    ..Dictionary::default()
                },
            );
            id
        };
        Ok(Self {
            lease: Arc::new(ScopeLease {
                registry: self.lease.registry.clone(),
                id,
            }),
        })
    }

    /// Registers or replaces a direct resource value.
    pub fn insert<T: Send + Sync + 'static>(&self, key: impl Into<String>, value: T) -> Result<()> {
        self.insert_value(None, key.into(), StoredValue::new(value))
    }

    /// Registers or replaces a value in a theme dictionary.
    pub fn insert_for_theme<T: Send + Sync + 'static>(
        &self,
        theme: ThemeVariant,
        key: impl Into<String>,
        value: T,
    ) -> Result<()> {
        self.insert_value(Some(theme), key.into(), StoredValue::new(value))
    }

    fn insert_value(
        &self,
        theme: Option<ThemeVariant>,
        key: String,
        value: StoredValue,
    ) -> Result<()> {
        validate_key(&key)?;
        let (revision, updates, old) = {
            let mut registry = self.lock()?;
            let old = values_mut(dictionary_mut(&mut registry, self.lease.id)?, theme)
                .insert(key.clone(), value);
            match prepare_updates(&registry) {
                Ok(updates) => {
                    registry.revision = registry.revision.wrapping_add(1);
                    (registry.revision, updates, old)
                }
                Err(error) => {
                    let values = values_mut(dictionary_mut(&mut registry, self.lease.id)?, theme);
                    let rejected = match old {
                        Some(old) => values.insert(key, old),
                        None => values.remove(&key),
                    };
                    drop(registry);
                    drop(rejected);
                    return Err(error);
                }
            }
        };
        drop(old);
        self.apply_updates(revision, updates)
    }

    /// Removes a direct value. Active dynamic references must still resolve.
    pub fn remove(&self, key: impl AsRef<str>) -> Result<bool> {
        self.remove_value(None, key.as_ref())
    }

    /// Removes a themed value. Active dynamic references must still resolve.
    pub fn remove_for_theme(&self, theme: ThemeVariant, key: impl AsRef<str>) -> Result<bool> {
        self.remove_value(Some(theme), key.as_ref())
    }

    fn remove_value(&self, theme: Option<ThemeVariant>, key: &str) -> Result<bool> {
        validate_key(key)?;
        let (revision, updates, removed) = {
            let mut registry = self.lock()?;
            let Some(removed) =
                values_mut(dictionary_mut(&mut registry, self.lease.id)?, theme).remove(key)
            else {
                return Ok(false);
            };
            match prepare_updates(&registry) {
                Ok(updates) => {
                    registry.revision = registry.revision.wrapping_add(1);
                    (registry.revision, updates, removed)
                }
                Err(error) => {
                    values_mut(dictionary_mut(&mut registry, self.lease.id)?, theme)
                        .insert(key.to_string(), removed);
                    return Err(error);
                }
            }
        };
        drop(removed);
        self.apply_updates(revision, updates)?;
        Ok(true)
    }

    /// Adds a dictionary to the merged dictionaries, searched last-added first.
    pub fn merge(&self, dictionary: &Self) -> Result<()> {
        self.require_same_registry(dictionary)?;
        let (revision, updates) = {
            let mut registry = self.lock()?;
            if reaches_dictionary(
                &registry,
                dictionary.lease.id,
                self.lease.id,
                &mut HashSet::new(),
            ) {
                return Err(Error::InvalidResourceScope(
                    "merged resource dictionaries cannot form a cycle".to_string(),
                ));
            }
            if dictionary_mut(&mut registry, self.lease.id)?
                .merged
                .contains(&dictionary.lease.id)
            {
                return Err(Error::InvalidResourceScope(
                    "resource dictionary is already merged into this scope".to_string(),
                ));
            }
            dictionary_mut(&mut registry, self.lease.id)?
                .merged
                .push(dictionary.lease.id);
            dictionary_mut(&mut registry, dictionary.lease.id)?.incoming += 1;
            match prepare_updates(&registry) {
                Ok(updates) => {
                    registry.revision = registry.revision.wrapping_add(1);
                    (registry.revision, updates)
                }
                Err(error) => {
                    dictionary_mut(&mut registry, self.lease.id)?.merged.pop();
                    dictionary_mut(&mut registry, dictionary.lease.id)?.incoming -= 1;
                    return Err(error);
                }
            }
        };
        self.apply_updates(revision, updates)
    }

    /// Removes a merged dictionary, provided active dynamic references still resolve.
    pub fn unmerge(&self, dictionary: &Self) -> Result<bool> {
        self.require_same_registry(dictionary)?;
        let (revision, updates) = {
            let mut registry = self.lock()?;
            let merged = &mut dictionary_mut(&mut registry, self.lease.id)?.merged;
            let Some(index) = merged.iter().position(|id| *id == dictionary.lease.id) else {
                return Ok(false);
            };
            merged.remove(index);
            match prepare_updates(&registry) {
                Ok(updates) => {
                    dictionary_mut(&mut registry, dictionary.lease.id)?.incoming -= 1;
                    registry.revision = registry.revision.wrapping_add(1);
                    (registry.revision, updates)
                }
                Err(error) => {
                    dictionary_mut(&mut registry, self.lease.id)?
                        .merged
                        .insert(index, dictionary.lease.id);
                    return Err(error);
                }
            }
        };
        self.apply_updates(revision, updates)?;
        Ok(true)
    }

    /// Returns a snapshot of the typed resource, suitable for `StaticResource`.
    pub fn get<T: Clone + Send + Sync + 'static>(&self, key: impl AsRef<str>) -> Result<T> {
        let key = key.as_ref();
        validate_key(key)?;
        let value = {
            let registry = self.lock()?;
            resolve(&registry, self.lease.id, key)?.typed::<T>(key)?
        };
        Ok((*value).clone())
    }

    /// Creates a typed signal that follows resource, hierarchy, and theme changes.
    ///
    /// Mutations that would leave any active signal missing or change its type
    /// are rejected and leave the registry unchanged.
    pub fn dynamic<T: Clone + PartialEq + Send + Sync + 'static>(
        &self,
        key: impl Into<String>,
    ) -> Result<Signal<T>> {
        let key = key.into();
        let signal = Signal::new_scoped(self.get::<T>(&key)?);
        let (watch_id, pending) = {
            let mut registry = self.lock()?;
            let watch_id = registry.next_watch_id;
            registry.next_watch_id += 1;
            let watch = TypedWatch::<T> {
                id: watch_id,
                scope_id: self.lease.id,
                key,
                signal: signal.raw_id(),
                active: Arc::new(AtomicBool::new(true)),
                _marker: std::marker::PhantomData,
            };
            let pending = watch.prepare(resolve(&registry, watch.scope_id, &watch.key)?)?;
            registry.watches.push(Box::new(watch));
            (watch_id, pending)
        };
        let registry = Arc::downgrade(&self.lease.registry);
        on_scope_cleanup(move || {
            if let Some(registry) = registry.upgrade() {
                let mut registry = match registry.lock() {
                    Ok(registry) => registry,
                    Err(poisoned) => {
                        eprintln!("resource scope registry lock is poisoned during cleanup");
                        poisoned.into_inner()
                    }
                };
                registry.watches.retain(|watch| watch.id() != watch_id);
            }
        });
        pending();
        Ok(signal)
    }

    /// Returns the registry's current theme.
    pub fn theme(&self) -> Result<ThemeVariant> {
        Ok(self.lock()?.theme)
    }

    /// Returns a signal of the registry's current theme.
    pub fn theme_signal(&self) -> Result<Signal<ThemeVariant>> {
        Signal::try_from_raw(self.lock()?.theme_signal)
    }

    /// Changes the theme after validating all active dynamic references.
    pub fn set_theme(&self, theme: ThemeVariant) -> Result<()> {
        let (revision, updates, raw_signal) = {
            let mut registry = self.lock()?;
            let previous = registry.theme;
            registry.theme = theme;
            match prepare_updates(&registry) {
                Ok(updates) => {
                    registry.revision = registry.revision.wrapping_add(1);
                    (registry.revision, updates, registry.theme_signal)
                }
                Err(error) => {
                    registry.theme = previous;
                    return Err(error);
                }
            }
        };
        let signal = Signal::<ThemeVariant>::try_from_raw(raw_signal)?;
        signal.set_if_changed(theme);
        self.apply_updates(revision, updates)
    }

    fn require_same_registry(&self, other: &Self) -> Result<()> {
        if Arc::ptr_eq(&self.lease.registry, &other.lease.registry) {
            Ok(())
        } else {
            Err(Error::InvalidResourceScope(
                "merged dictionaries must belong to the same resource registry".to_string(),
            ))
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, ResourceRegistry>> {
        self.lease.registry.lock().map_err(|_| {
            Error::InvalidResourceScope("resource registry lock is poisoned".to_string())
        })
    }

    fn apply_updates(&self, revision: u64, updates: Vec<PendingUpdate>) -> Result<()> {
        for update in updates {
            if self.lock()?.revision != revision {
                break;
            }
            update();
        }
        Ok(())
    }
}

fn validate_key(key: &str) -> Result<()> {
    if key.trim().is_empty() {
        Err(Error::InvalidResourceScope(
            "resource keys cannot be empty".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn dictionary_mut(registry: &mut ResourceRegistry, id: u64) -> Result<&mut Dictionary> {
    registry.dictionaries.get_mut(&id).ok_or_else(|| {
        Error::InvalidResourceScope("resource dictionary is no longer available".to_string())
    })
}

fn values_mut(
    dictionary: &mut Dictionary,
    theme: Option<ThemeVariant>,
) -> &mut HashMap<String, StoredValue> {
    match theme {
        Some(theme) => dictionary.themed.entry(theme).or_default(),
        None => &mut dictionary.direct,
    }
}

fn resolve<'a>(
    registry: &'a ResourceRegistry,
    scope_id: u64,
    key: &str,
) -> Result<&'a StoredValue> {
    let mut current = Some(scope_id);
    while let Some(id) = current {
        let dictionary = registry.dictionaries.get(&id).ok_or_else(|| {
            Error::InvalidResourceScope("resource dictionary is no longer available".to_string())
        })?;
        if let Some(value) = resolve_dictionary(registry, id, key, registry.theme) {
            return Ok(value);
        }
        current = dictionary.parent;
    }
    Err(Error::MissingScopedResource {
        key: key.to_string(),
        theme: registry.theme,
    })
}

fn resolve_dictionary<'a>(
    registry: &'a ResourceRegistry,
    id: u64,
    key: &str,
    theme: ThemeVariant,
) -> Option<&'a StoredValue> {
    let dictionary = registry.dictionaries.get(&id)?;
    if let Some(value) = dictionary.direct.get(key) {
        return Some(value);
    }
    if theme != ThemeVariant::Default {
        if let Some(value) = dictionary
            .themed
            .get(&theme)
            .and_then(|values| values.get(key))
        {
            return Some(value);
        }
    }
    if let Some(value) = dictionary
        .themed
        .get(&ThemeVariant::Default)
        .and_then(|values| values.get(key))
    {
        return Some(value);
    }
    dictionary
        .merged
        .iter()
        .rev()
        .find_map(|merged| resolve_dictionary(registry, *merged, key, theme))
}

fn prepare_updates(registry: &ResourceRegistry) -> Result<Vec<PendingUpdate>> {
    registry
        .watches
        .iter()
        .map(|watch| watch.prepare(resolve(registry, watch.scope_id(), watch.key())?))
        .collect()
}

fn reaches_dictionary(
    registry: &ResourceRegistry,
    current: u64,
    target: u64,
    visited: &mut HashSet<u64>,
) -> bool {
    if current == target {
        return true;
    }
    if !visited.insert(current) {
        return false;
    }
    registry
        .dictionaries
        .get(&current)
        .is_some_and(|dictionary| {
            dictionary
                .parent
                .into_iter()
                .chain(dictionary.merged.iter().copied())
                .any(|id| reaches_dictionary(registry, id, target, visited))
        })
}

fn collect_unused_dictionaries(
    registry: &mut ResourceRegistry,
    id: u64,
    removed: &mut Vec<Dictionary>,
) {
    let Some(dictionary) = registry.dictionaries.get(&id) else {
        return;
    };
    if dictionary.has_handle || dictionary.incoming != 0 {
        return;
    }
    let dictionary = registry
        .dictionaries
        .remove(&id)
        .expect("dictionary was just checked");
    let outgoing = dictionary
        .parent
        .into_iter()
        .chain(dictionary.merged.iter().copied())
        .collect::<Vec<_>>();
    removed.push(dictionary);
    for next in outgoing {
        if let Some(parent) = registry.dictionaries.get_mut(&next) {
            parent.incoming -= 1;
        }
        collect_unused_dictionaries(registry, next, removed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Brush, Color, Effect};

    #[test]
    fn local_parent_and_merged_dictionaries_have_predictable_precedence() {
        let application = ResourceScope::new();
        let first = application.dictionary().unwrap();
        let second = application.dictionary().unwrap();
        first.insert("label", String::from("first")).unwrap();
        second.insert("label", String::from("second")).unwrap();
        application.merge(&first).unwrap();
        application.merge(&second).unwrap();
        let child = application.child().unwrap();
        let label = child.dynamic::<String>("label").unwrap();
        assert_eq!(label.get(), "second");

        child.insert("label", String::from("local")).unwrap();
        assert_eq!(label.get(), "local");
        child.remove("label").unwrap();
        assert_eq!(label.get(), "second");
        application
            .insert("label", String::from("application"))
            .unwrap();
        assert_eq!(label.get(), "application");
        application.remove("label").unwrap();
        assert_eq!(label.get(), "second");
        application.unmerge(&second).unwrap();
        assert_eq!(label.get(), "first");
        assert!(matches!(
            application.unmerge(&first),
            Err(Error::MissingScopedResource { .. })
        ));
        assert_eq!(label.get(), "first");
    }

    #[test]
    fn static_snapshots_and_dynamic_values_follow_theme_dictionaries() {
        let resources = ResourceScope::new();
        let default = Some(Brush::solid(Color::rgb(20, 30, 40)));
        let dark = Some(Brush::solid(Color::rgb(200, 210, 220)));
        resources
            .insert_for_theme(ThemeVariant::Default, "accent", default)
            .unwrap();
        resources
            .insert_for_theme(ThemeVariant::Dark, "accent", dark)
            .unwrap();
        let snapshot = resources.get::<Option<Brush>>("accent").unwrap();
        let dynamic = resources.dynamic::<Option<Brush>>("accent").unwrap();
        let theme = resources.theme_signal().unwrap();

        resources.set_theme(ThemeVariant::Dark).unwrap();
        assert_eq!(theme.get(), ThemeVariant::Dark);
        assert_eq!(dynamic.get(), dark);
        assert_eq!(snapshot, default);
        resources.insert("accent", default).unwrap();
        assert_eq!(dynamic.get(), default);
        resources.remove("accent").unwrap();
        assert_eq!(dynamic.get(), dark);
        resources
            .remove_for_theme(ThemeVariant::Dark, "accent")
            .unwrap();
        assert_eq!(dynamic.get(), default);
        assert!(resources
            .remove_for_theme(ThemeVariant::Default, "accent")
            .is_err());
        assert_eq!(dynamic.get(), default);
    }

    #[test]
    fn invalid_mutations_are_rejected_without_changing_active_resources() {
        let resources = ResourceScope::new();
        resources
            .insert_for_theme(ThemeVariant::Light, "title", String::from("Light"))
            .unwrap();
        resources.set_theme(ThemeVariant::Light).unwrap();
        let title = resources.dynamic::<String>("title").unwrap();

        assert!(matches!(
            resources.set_theme(ThemeVariant::Dark),
            Err(Error::MissingScopedResource { .. })
        ));
        assert_eq!(resources.theme().unwrap(), ThemeVariant::Light);
        assert!(matches!(
            resources.insert("title", 42_i32),
            Err(Error::ResourceTypeMismatch { .. })
        ));
        assert_eq!(title.get(), "Light");
        assert_eq!(resources.get::<String>("title").unwrap(), "Light");
        assert!(resources.insert("", "invalid").is_err());

        let child = resources.child().unwrap();
        assert!(resources.merge(&child).is_err());
        assert!(resources.merge(&ResourceScope::new()).is_err());
        let first = resources.dictionary().unwrap();
        let second = resources.dictionary().unwrap();
        first.merge(&second).unwrap();
        assert!(second.merge(&first).is_err());
    }

    #[test]
    fn dropping_a_scope_stops_its_dynamic_updates_and_releases_dictionaries() {
        let application = ResourceScope::new();
        application.insert("label", String::from("before")).unwrap();
        let child = application.child().unwrap();
        let label = child.dynamic::<String>("label").unwrap();
        let child_id = child.lease.id;
        assert_eq!(application.lock().unwrap().watches.len(), 1);
        drop(child);
        assert_eq!(application.lock().unwrap().watches.len(), 0);
        assert!(!application
            .lock()
            .unwrap()
            .dictionaries
            .contains_key(&child_id));
        application.insert("label", String::from("after")).unwrap();
        assert_eq!(label.get(), "before");

        let child = application.child().unwrap();
        drop(application);
        assert_eq!(child.get::<String>("label").unwrap(), "after");
    }

    #[test]
    fn disposing_a_view_removes_its_dynamic_resource_subscription() {
        let resources = ResourceScope::new();
        resources.insert("label", String::from("before")).unwrap();
        let scope = crate::ReactiveScope::new();
        let _label = scope.run(|| resources.dynamic::<String>("label").unwrap());
        assert_eq!(resources.lock().unwrap().watches.len(), 1);
        scope.dispose();
        assert!(resources.lock().unwrap().watches.is_empty());
        assert!(resources.remove("label").unwrap());
    }

    #[test]
    fn disposing_a_view_during_resource_updates_skips_its_pending_signal() {
        let resources = ResourceScope::new();
        resources.insert("label", String::from("before")).unwrap();
        let trigger = resources.dynamic::<String>("label").unwrap();
        let view = crate::ReactiveScope::new();
        let _view_signal = view.run(|| resources.dynamic::<String>("label").unwrap());
        let survivor = resources.dynamic::<String>("label").unwrap();
        let effect = Effect::new(move || {
            if trigger.get() == "after" {
                view.dispose();
            }
            Ok(())
        })
        .unwrap();

        resources.insert("label", String::from("after")).unwrap();
        assert_eq!(survivor.get(), "after");
        assert_eq!(resources.lock().unwrap().watches.len(), 2);
        effect.dispose();
    }

    #[test]
    fn reentrant_changes_refresh_all_dynamic_resources() {
        let resources = ResourceScope::new();
        resources.insert("first", String::from("old")).unwrap();
        resources.insert("second", String::from("old")).unwrap();
        let first = resources.dynamic::<String>("first").unwrap();
        let second = resources.dynamic::<String>("second").unwrap();
        let nested = resources.clone();
        let effect = Effect::new(move || {
            if first.get() == "trigger" {
                nested.insert("second", String::from("nested"))?;
            }
            Ok(())
        })
        .unwrap();

        resources.insert("first", String::from("trigger")).unwrap();
        assert_eq!(first.get(), "trigger");
        assert_eq!(second.get(), "nested");
        effect.dispose();
    }
}
