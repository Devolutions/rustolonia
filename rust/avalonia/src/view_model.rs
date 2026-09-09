use crate::data_shapes::{
    CancellationRegistry, CancellationToken, CompletionSlots, MapKey, RangeBatch, RangeQueue,
    RangeQueueGuard, RangeRequest, RangeStates,
};
use crate::{AppScope, Error, Result, Window};
use avalonia_sys as sys;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};

/// Rust-side bookkeeping for nested view-model handles a sink has published:
/// one slot per nested-model property (replacing drops the previous handle)
/// and one ordered list per nested-model collection (mirroring the managed
/// `ObservableCollection<T>` so insert/remove/move/replace/clear stay in
/// sync). This is what keeps repeated attach/replace/detach cycles from
/// accumulating live COM references: dropping a slot's `Box` releases Rust's
/// contribution to that nested object's reference count immediately, and
/// dropping the whole `NestedSlots` (when the last `ViewModelSink` clone for
/// a mount is dropped) releases everything still tracked.
/// One nested handle slot. Boxed as `Any` so `NestedSlots` can hold the
/// generic `ViewModelHandle` without leaking its type into every map.
type NestedSlot = Box<dyn Any + Send>;

#[derive(Default)]
struct NestedSlots {
    properties: Mutex<HashMap<i32, NestedSlot>>,
    collections: Mutex<HashMap<i32, Vec<NestedSlot>>>,
    /// One slot per live map entry. A windowed collection deliberately has no
    /// equivalent: its rows are *transferred* to the range batch, because
    /// their managed lifetime is decided by page eviction, which Rust cannot
    /// observe. Map entries, like collection items, are removed explicitly.
    maps: Mutex<HashMap<i32, HashMap<MapKey, NestedSlot>>>,
    /// One slot per command holding its last published structured result.
    command_results: Mutex<HashMap<i32, NestedSlot>>,
}

impl NestedSlots {
    fn set_property(&self, property_id: i32, handle: Option<ViewModelHandle>) {
        let mut properties = self.properties.lock_slots();
        match handle {
            Some(handle) => {
                properties.insert(property_id, Box::new(handle));
            }
            None => {
                properties.remove(&property_id);
            }
        }
    }

    fn push_collection_item(&self, collection_id: i32, handle: ViewModelHandle) {
        self.collections
            .lock_slots()
            .entry(collection_id)
            .or_default()
            .push(Box::new(handle));
    }

    fn insert_collection_item(&self, collection_id: i32, index: usize, handle: ViewModelHandle) {
        let mut collections = self.collections.lock_slots();
        let items = collections.entry(collection_id).or_default();
        // Managed validated the index against its own mirror of this list; a
        // disagreement means an interleaved legacy v2 call already changed the
        // list, so append rather than panic on a stale index.
        let index = index.min(items.len());
        items.insert(index, Box::new(handle));
    }

    fn replace_collection_item(&self, collection_id: i32, index: usize, handle: ViewModelHandle) {
        let mut collections = self.collections.lock_slots();
        let items = collections.entry(collection_id).or_default();
        if index < items.len() {
            items[index] = Box::new(handle);
        } else {
            items.push(Box::new(handle));
        }
    }

    fn remove_collection_item(&self, collection_id: i32, index: usize) {
        let mut collections = self.collections.lock_slots();
        if let Some(items) = collections.get_mut(&collection_id) {
            if index < items.len() {
                items.remove(index);
            }
        }
    }

    fn move_collection_item(&self, collection_id: i32, from_index: usize, to_index: usize) {
        let mut collections = self.collections.lock_slots();
        let Some(items) = collections.get_mut(&collection_id) else {
            return;
        };
        if from_index >= items.len() || to_index >= items.len() {
            return;
        }
        let item = items.remove(from_index);
        items.insert(to_index, item);
    }

    fn clear_collection(&self, collection_id: i32) {
        let mut collections = self.collections.lock_slots();
        if let Some(items) = collections.get_mut(&collection_id) {
            items.clear();
        }
    }

    fn set_collection(&self, collection_id: i32, handles: Vec<ViewModelHandle>) {
        self.collections.lock_slots().insert(
            collection_id,
            handles
                .into_iter()
                .map(|handle| Box::new(handle) as NestedSlot)
                .collect(),
        );
    }

    fn set_map_value(&self, map_id: i32, key: MapKey, handle: Option<ViewModelHandle>) {
        let mut maps = self.maps.lock_slots();
        let entries = maps.entry(map_id).or_default();
        match handle {
            Some(handle) => {
                entries.insert(key, Box::new(handle));
            }
            None => {
                entries.remove(&key);
            }
        }
    }

    fn clear_map(&self, map_id: i32) {
        if let Some(entries) = self.maps.lock_slots().get_mut(&map_id) {
            entries.clear();
        }
    }

    fn set_command_result(&self, command_id: i32, handle: Option<ViewModelHandle>) {
        let mut results = self.command_results.lock_slots();
        match handle {
            Some(handle) => {
                results.insert(command_id, Box::new(handle));
            }
            None => {
                results.remove(&command_id);
            }
        }
    }
}

/// One nested-ownership change a batch operation implies. The batch carries the
/// managed-visible operation and this delta side by side, and the delta is
/// applied to `NestedSlots` exactly once, only when managed code reports
/// `Applied`. A stale, cancelled or failed batch simply drops its candidates,
/// which releases Rust's contribution to those never-published nested objects
/// without disturbing the slots that are still live.
enum NestedBatchDelta {
    Set(i32, Option<ViewModelHandle>),
    Add(i32, ViewModelHandle),
    Insert(i32, usize, ViewModelHandle),
    Replace(i32, usize, ViewModelHandle),
    Remove(i32, usize),
    Move(i32, usize, usize),
    Clear(i32),
    Snapshot(i32, Vec<ViewModelHandle>),
}

impl NestedBatchDelta {
    fn apply(self, nested: &NestedSlots) {
        match self {
            Self::Set(id, handle) => nested.set_property(id, handle),
            Self::Add(id, handle) => nested.push_collection_item(id, handle),
            Self::Insert(id, index, handle) => nested.insert_collection_item(id, index, handle),
            Self::Replace(id, index, handle) => nested.replace_collection_item(id, index, handle),
            Self::Remove(id, index) => nested.remove_collection_item(id, index),
            Self::Move(id, from, to) => nested.move_collection_item(id, from, to),
            Self::Clear(id) => nested.clear_collection(id),
            Self::Snapshot(id, handles) => nested.set_collection(id, handles),
        }
    }
}

/// Reconciles a batch's nested ownership. Invoked only from the batch's
/// ownership-commit capability, which managed code calls exactly once after it
/// has stored the batch's state and before it publishes any notification. A
/// batch that never reaches that point simply drops this callback, which
/// releases its candidate handles without touching the live slots.
fn reconcile_nested(nested: &NestedSlots, delta: Vec<NestedBatchDelta>) {
    for change in delta {
        change.apply(nested);
    }
}

/// Small helper so lock-poisoning has one documented, consistent message
/// across every `NestedSlots` accessor (a poisoned lock here means an
/// earlier panic happened while a nested handle was being installed or
/// removed; we still want a clear panic message rather than a generic
/// `unwrap` one).
trait LockSlots<T> {
    fn lock_slots(&self) -> std::sync::MutexGuard<'_, T>;
}

impl<T> LockSlots<T> for Mutex<T> {
    fn lock_slots(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock().expect("nested view-model slot lock poisoned")
    }
}

impl fmt::Debug for NestedSlots {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NestedSlots")
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct ViewModelSink {
    raw: sys::ComPtr<sys::IAvnRustVmSink>,
    raw2: sys::ComPtr<sys::IAvnRustVmSink2>,
    raw3: Arc<OnceLock<BatchCapability>>,
    raw4: Arc<OnceLock<ShapesCapability>>,
    raw5: Arc<OnceLock<NumbersCapability>>,
    nested: Arc<NestedSlots>,
    ranges: Arc<RangeStates>,
    completions: Arc<CompletionSlots>,
}

/// The cached result of the one-time `IAvnRustVmSink3` query. Only
/// `E_NOINTERFACE` is recorded as "absent"; any other `QueryInterface` failure
/// is recorded verbatim and reported to every later submission instead of being
/// silently downgraded to "this host has no batch support".
type BatchCapability = std::result::Result<Option<sys::ComPtr<sys::IAvnRustVmSink3>>, sys::Error>;

/// The cached result of the one-time `IAvnRustVmSink4` query, resolved with
/// exactly the same discipline as the batch capability. A host that predates
/// stage 30 reports `E_NOINTERFACE` from every map/progress/result/range call
/// instead of silently dropping the update.
type ShapesCapability = std::result::Result<Option<sys::ComPtr<sys::IAvnRustVmSink4>>, sys::Error>;

/// The cached result of the one-time `IAvnRustVmSink5` query, resolved with
/// exactly the same discipline as the batch and shapes capabilities. A host
/// whose adapter declares no scalar-number collection reports `E_NOINTERFACE`
/// from every integer/double element call instead of silently dropping it.
type NumbersCapability = std::result::Result<Option<sys::ComPtr<sys::IAvnRustVmSink5>>, sys::Error>;

impl ViewModelSink {
    /// Wraps a freshly attached v1 sink and eagerly resolves the v2
    /// capability. Failing fast here (rather than lazily on first v2 use)
    /// keeps the contract explicit: a host whose generated adapter has not
    /// been regenerated to implement `IAvnRustVmSink2` fails attach with a
    /// clear ABI error instead of silently dropping nested/nullable/
    /// collection/`CanExecute`/validation updates later.
    ///
    /// v3 is deliberately different: it is an optional capability resolved
    /// lazily on first batch submission and cached, so a v1/v2-only host still
    /// attaches and publishes normally and only `submit_batch` reports
    /// `E_NOINTERFACE`.
    fn with_state(
        raw: sys::ComPtr<sys::IAvnRustVmSink>,
        ranges: Arc<RangeStates>,
        completions: Arc<CompletionSlots>,
    ) -> Result<Self> {
        let raw2 = raw.query_interface::<sys::IAvnRustVmSink2>()?;
        Ok(Self {
            raw,
            raw2,
            raw3: Arc::new(OnceLock::new()),
            raw4: Arc::new(OnceLock::new()),
            raw5: Arc::new(OnceLock::new()),
            nested: Arc::new(NestedSlots::default()),
            ranges,
            completions,
        })
    }

    /// Resolves (once) the optional stage 30 richer-shapes capability.
    fn shapes_sink(&self) -> Result<&sys::ComPtr<sys::IAvnRustVmSink4>> {
        match self.raw4.get_or_init(
            || match self.raw.query_interface::<sys::IAvnRustVmSink4>() {
                Ok(value) => Ok(Some(value)),
                Err(error) if error.0 == sys::E_NOINTERFACE => Ok(None),
                Err(error) => Err(error),
            },
        ) {
            Ok(Some(sink)) => Ok(sink),
            Ok(None) => Err(Error::Abi(sys::Error(sys::E_NOINTERFACE))),
            Err(error) => Err(Error::Abi(*error)),
        }
    }

    /// Resolves (once) the optional scalar-number collection capability.
    fn numbers_sink(&self) -> Result<&sys::ComPtr<sys::IAvnRustVmSink5>> {
        match self.raw5.get_or_init(
            || match self.raw.query_interface::<sys::IAvnRustVmSink5>() {
                Ok(value) => Ok(Some(value)),
                Err(error) if error.0 == sys::E_NOINTERFACE => Ok(None),
                Err(error) => Err(error),
            },
        ) {
            Ok(Some(sink)) => Ok(sink),
            Ok(None) => Err(Error::Abi(sys::Error(sys::E_NOINTERFACE))),
            Err(error) => Err(Error::Abi(*error)),
        }
    }

    /// Resolves (once) the optional batch capability.
    fn batch_sink(&self) -> Result<&sys::ComPtr<sys::IAvnRustVmSink3>> {
        match self.raw3.get_or_init(
            || match self.raw.query_interface::<sys::IAvnRustVmSink3>() {
                Ok(value) => Ok(Some(value)),
                Err(error) if error.0 == sys::E_NOINTERFACE => Ok(None),
                Err(error) => Err(error),
            },
        ) {
            Ok(Some(sink)) => Ok(sink),
            Ok(None) => Err(Error::Abi(sys::Error(sys::E_NOINTERFACE))),
            Err(error) => Err(Error::Abi(*error)),
        }
    }

    pub fn set_string(&self, property_id: i32, value: impl AsRef<str>) -> Result<()> {
        self.raw.set_string(property_id, &utf16(value))?;
        Ok(())
    }

    pub fn set_integer(&self, property_id: i32, value: i64) -> Result<()> {
        self.raw.set_integer(property_id, value)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_boolean(&self, property_id: i32, value: bool) -> Result<()> {
        self.raw.set_boolean(property_id, value)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_double(&self, property_id: i32, value: f64) -> Result<()> {
        self.raw.set_double(property_id, value)?;
        Ok(())
    }

    pub fn add_string(&self, collection_id: i32, value: impl AsRef<str>) -> Result<()> {
        self.raw.add_string(collection_id, &utf16(value))?;
        Ok(())
    }

    /// Publishes that a nullable scalar property currently has no value.
    pub fn set_null(&self, property_id: i32) -> Result<()> {
        self.raw2.set_null(property_id)?;
        self.nested.set_property(property_id, None);
        Ok(())
    }

    /// Publishes a nested view-model property. `None` clears it (and drops
    /// Rust's own reference to the previously published nested model, if
    /// any); `Some` wraps `model` in a fresh nested COM capability exactly
    /// like a top-level mount, but without making it a window.
    pub fn set_model(&self, property_id: i32, model: Option<impl DynamicViewModel>) -> Result<()> {
        match model {
            Some(model) => {
                let handle = ViewModelHandle::new(model);
                self.raw2.set_model(property_id, Some(&handle.raw))?;
                self.nested.set_property(property_id, Some(handle));
            }
            None => {
                self.raw2.set_model(property_id, None)?;
                self.nested.set_property(property_id, None);
            }
        }
        Ok(())
    }

    /// Appends a nested view-model item to a model-kind collection.
    pub fn add_model(&self, collection_id: i32, model: impl DynamicViewModel) -> Result<()> {
        let handle = ViewModelHandle::new(model);
        self.raw2.add_model(collection_id, &handle.raw)?;
        self.nested.push_collection_item(collection_id, handle);
        Ok(())
    }

    pub fn insert_string(
        &self,
        collection_id: i32,
        index: i32,
        value: impl AsRef<str>,
    ) -> Result<()> {
        self.raw2
            .insert_string(collection_id, index, &utf16(value))?;
        Ok(())
    }

    pub fn insert_model(
        &self,
        collection_id: i32,
        index: i32,
        model: impl DynamicViewModel,
    ) -> Result<()> {
        let handle = ViewModelHandle::new(model);
        self.raw2.insert_model(collection_id, index, &handle.raw)?;
        self.nested
            .insert_collection_item(collection_id, index as usize, handle);
        Ok(())
    }

    pub fn replace_string(
        &self,
        collection_id: i32,
        index: i32,
        value: impl AsRef<str>,
    ) -> Result<()> {
        self.raw2
            .replace_string(collection_id, index, &utf16(value))?;
        Ok(())
    }

    pub fn replace_model(
        &self,
        collection_id: i32,
        index: i32,
        model: impl DynamicViewModel,
    ) -> Result<()> {
        let handle = ViewModelHandle::new(model);
        self.raw2.replace_model(collection_id, index, &handle.raw)?;
        self.nested
            .replace_collection_item(collection_id, index as usize, handle);
        Ok(())
    }

    pub fn remove_string_at(&self, collection_id: i32, index: i32) -> Result<()> {
        self.raw2.remove_at(collection_id, index)?;
        Ok(())
    }

    pub fn remove_model_at(&self, collection_id: i32, index: i32) -> Result<()> {
        self.raw2.remove_at(collection_id, index)?;
        self.nested
            .remove_collection_item(collection_id, index as usize);
        Ok(())
    }

    pub fn move_string_item(
        &self,
        collection_id: i32,
        from_index: i32,
        to_index: i32,
    ) -> Result<()> {
        self.raw2.move_item(collection_id, from_index, to_index)?;
        Ok(())
    }

    pub fn move_model_item(
        &self,
        collection_id: i32,
        from_index: i32,
        to_index: i32,
    ) -> Result<()> {
        self.raw2.move_item(collection_id, from_index, to_index)?;
        self.nested
            .move_collection_item(collection_id, from_index as usize, to_index as usize);
        Ok(())
    }

    pub fn clear_string_collection(&self, collection_id: i32) -> Result<()> {
        self.raw2.clear_collection(collection_id)?;
        Ok(())
    }

    pub fn clear_model_collection(&self, collection_id: i32) -> Result<()> {
        self.raw2.clear_collection(collection_id)?;
        self.nested.clear_collection(collection_id);
        Ok(())
    }

    // ---- Scalar-number collections (IAvnRustVmSink5) --------------------
    //
    // Only the value-carrying calls need the v5 capability. Removal, movement
    // and clearing carry no element value, so they ride the v2 sink exactly
    // like a string collection does and keep working against a host that
    // never had to grow a new vtable.

    pub fn add_integer(&self, collection_id: i32, value: i64) -> Result<()> {
        self.numbers_sink()?.add_integer(collection_id, value)?;
        Ok(())
    }

    pub fn insert_integer(&self, collection_id: i32, index: i32, value: i64) -> Result<()> {
        self.numbers_sink()?
            .insert_integer(collection_id, index, value)?;
        Ok(())
    }

    pub fn replace_integer(&self, collection_id: i32, index: i32, value: i64) -> Result<()> {
        self.numbers_sink()?
            .replace_integer(collection_id, index, value)?;
        Ok(())
    }

    pub fn add_double(&self, collection_id: i32, value: f64) -> Result<()> {
        self.numbers_sink()?.add_double(collection_id, value)?;
        Ok(())
    }

    pub fn insert_double(&self, collection_id: i32, index: i32, value: f64) -> Result<()> {
        self.numbers_sink()?
            .insert_double(collection_id, index, value)?;
        Ok(())
    }

    pub fn replace_double(&self, collection_id: i32, index: i32, value: f64) -> Result<()> {
        self.numbers_sink()?
            .replace_double(collection_id, index, value)?;
        Ok(())
    }

    pub fn remove_number_at(&self, collection_id: i32, index: i32) -> Result<()> {
        self.raw2.remove_at(collection_id, index)?;
        Ok(())
    }

    pub fn move_number_item(
        &self,
        collection_id: i32,
        from_index: i32,
        to_index: i32,
    ) -> Result<()> {
        self.raw2.move_item(collection_id, from_index, to_index)?;
        Ok(())
    }

    pub fn clear_number_collection(&self, collection_id: i32) -> Result<()> {
        self.raw2.clear_collection(collection_id)?;
        Ok(())
    }

    /// True when the attached host implements the scalar-number collection
    /// capability. A model shared by a generated adapter and the reflectable
    /// (dynamic-binding) adapter checks this once instead of treating an
    /// explicit `E_NOINTERFACE` as a failure.
    pub fn supports_number_collections(&self) -> bool {
        self.numbers_sink().is_ok()
    }

    /// Publishes a command's current `ICommand.CanExecute` state.
    /// Publishes a command's current <c>ICommand.CanExecute</c> state.
    pub fn set_command_enabled(&self, command_id: i32, enabled: bool) -> Result<()> {
        self.raw2.set_command_enabled(command_id, enabled)?;
        Ok(())
    }

    /// True when the attached host implements the stage 30 sink capability.
    /// The reflectable (dynamic-binding) adapter deliberately does not, so a
    /// model shared by both mounts checks this once instead of treating an
    /// explicit `E_NOINTERFACE` as a failure.
    pub fn supports_richer_shapes(&self) -> bool {
        self.shapes_sink().is_ok()
    }

    // ---- Stage 30: observable keyed maps -------------------------------

    /// Inserts or replaces a string value for one map key.
    pub fn map_set_string(&self, map_id: i32, key: MapKey, value: impl AsRef<str>) -> Result<()> {
        self.shapes_sink()?
            .map_set_string(map_id, &key.to_wire(), &utf16(value))?;
        self.nested.set_map_value(map_id, key, None);
        Ok(())
    }

    pub fn map_set_integer(&self, map_id: i32, key: MapKey, value: i64) -> Result<()> {
        self.shapes_sink()?
            .map_set_integer(map_id, &key.to_wire(), value)?;
        self.nested.set_map_value(map_id, key, None);
        Ok(())
    }

    pub fn map_set_boolean(&self, map_id: i32, key: MapKey, value: bool) -> Result<()> {
        self.shapes_sink()?
            .map_set_boolean(map_id, &key.to_wire(), value)?;
        self.nested.set_map_value(map_id, key, None);
        Ok(())
    }

    pub fn map_set_double(&self, map_id: i32, key: MapKey, value: f64) -> Result<()> {
        self.shapes_sink()?
            .map_set_double(map_id, &key.to_wire(), value)?;
        self.nested.set_map_value(map_id, key, None);
        Ok(())
    }

    /// Inserts or replaces a nested view-model value. The previous value's
    /// Rust handle is dropped only after the managed side accepted the new
    /// one, so a failed publication never orphans the live entry.
    pub fn map_set_model(
        &self,
        map_id: i32,
        key: MapKey,
        model: impl DynamicViewModel,
    ) -> Result<()> {
        let handle = ViewModelHandle::new(model);
        self.shapes_sink()?
            .map_set_model(map_id, &key.to_wire(), &handle.raw)?;
        self.nested.set_map_value(map_id, key, Some(handle));
        Ok(())
    }

    /// Removes one key. A missing key is a no-op, not an error.
    pub fn map_remove(&self, map_id: i32, key: MapKey) -> Result<()> {
        self.shapes_sink()?.map_remove(map_id, &key.to_wire())?;
        self.nested.set_map_value(map_id, key, None);
        Ok(())
    }

    pub fn map_clear(&self, map_id: i32) -> Result<()> {
        self.shapes_sink()?.map_clear(map_id)?;
        self.nested.clear_map(map_id);
        Ok(())
    }

    // ---- Stage 30: async progress, cancellation and structured results ---

    /// Publishes determinate (`Some`, clamped to 0..1) or indeterminate
    /// (`None`) progress for an async command.
    pub fn set_command_progress(
        &self,
        command_id: i32,
        value: Option<f64>,
        message: Option<&str>,
    ) -> Result<()> {
        let message = message.map(utf16);
        self.shapes_sink()?.set_command_progress(
            command_id,
            value.map(|value| value.clamp(0.0, 1.0)),
            message.as_deref(),
        )?;
        Ok(())
    }

    /// Publishes a command's running state. Managed code gates its generated
    /// cancel command on it.
    pub fn set_command_running(&self, command_id: i32, running: bool) -> Result<()> {
        self.shapes_sink()?
            .set_command_running(command_id, running)?;
        if !running {
            self.completions.forget(command_id);
        }
        Ok(())
    }

    /// Publishes a command's typed structured result, or clears it.
    pub fn set_command_result(
        &self,
        command_id: i32,
        model: Option<impl DynamicViewModel>,
    ) -> Result<()> {
        match model {
            Some(model) => {
                let handle = ViewModelHandle::new(model);
                self.shapes_sink()?
                    .set_command_result(command_id, Some(&handle.raw))?;
                self.nested.set_command_result(command_id, Some(handle));
            }
            None => self.clear_command_result(command_id)?,
        }
        Ok(())
    }

    /// Clears a command's structured result and drops Rust's handle to it.
    pub fn clear_command_result(&self, command_id: i32) -> Result<()> {
        self.shapes_sink()?.set_command_result(command_id, None)?;
        self.nested.set_command_result(command_id, None);
        Ok(())
    }

    /// Claims the single terminal transition of one tracked async invocation.
    /// Returns false when success, failure or cancellation already claimed it,
    /// which is what makes "completes exactly once" observable rather than
    /// merely intended.
    pub fn claim_completion(&self, command_id: i32, token: &CancellationToken) -> bool {
        self.completions.claim(command_id, token.operation_id())
    }

    // ---- Stage 30: windowed range publication ---------------------------

    /// Republishes a windowed collection's identity. Every realized page is
    /// invalidated managed-side, because rows may no longer be at the same
    /// index. This is also what primes a freshly attached window.
    pub fn publish_range_reset(
        &self,
        collection_id: i32,
        generation: i64,
        total_count: i64,
    ) -> Result<()> {
        self.ranges.set(collection_id, generation, total_count);
        let batch = sys::rust_vm_range_batch(
            sys::RUST_VM_RANGE_RESET,
            collection_id,
            generation,
            total_count,
            0,
            Vec::new(),
            None,
        );
        self.shapes_sink()?.publish_range(&batch)?;
        Ok(())
    }

    /// Asks the host to re-request every realized page at the current
    /// generation. Live values (CPU%, counters) update in place; adapters,
    /// scroll and selection stay. No-ops as `InvalidViewModelMember` when this
    /// collection has never been reset.
    pub fn publish_range_invalidate(&self, collection_id: i32) -> Result<()> {
        let (generation, total_count) =
            self.ranges
                .get(collection_id)
                .ok_or(Error::InvalidViewModelMember {
                    kind: "window",
                    id: collection_id,
                })?;
        let batch = sys::rust_vm_range_batch(
            sys::RUST_VM_RANGE_INVALIDATE,
            collection_id,
            generation,
            total_count,
            0,
            Vec::new(),
            None,
        );
        self.shapes_sink()?.publish_range(&batch)?;
        Ok(())
    }

    /// Publishes one realized page.
    ///
    /// Ownership of every element model transfers to the batch: Rust keeps no
    /// slot for a windowed row, because its managed lifetime is decided by page
    /// eviction, which Rust cannot observe. A stale or rejected batch simply
    /// drops, releasing those models without touching anything live.
    pub fn publish_range(&self, batch: RangeBatch) -> Result<BatchCompletion> {
        let (completion, callback) = BatchCompletion::channel();
        self.publish_range_raw(batch, callback).map(|()| completion)
    }

    /// Same as [`ViewModelSink::publish_range`], with a callback invoked after
    /// the UI dispatcher applies or rejects the range.
    pub fn publish_range_with_callback(
        &self,
        batch: RangeBatch,
        callback: impl FnOnce(BatchOutcome) + Send + 'static,
    ) -> Result<()> {
        self.publish_range_raw(batch, move |outcome, error| {
            callback(BatchOutcome::from_wire(outcome, error))
        })
    }

    fn publish_range_raw(
        &self,
        batch: RangeBatch,
        callback: impl FnOnce(i32, i32) + Send + 'static,
    ) -> Result<()> {
        // Resolve the optional capability before taking ownership of the
        // elements, so a host without stage 30 support leaves them untouched.
        let sink = self.shapes_sink()?.clone();
        let (collection_id, generation, total_count, offset, items) = batch.into_parts();
        let raw = sys::rust_vm_range_batch(
            sys::RUST_VM_RANGE_FILL,
            collection_id,
            generation,
            total_count,
            offset,
            items,
            Some(Box::new(callback)),
        );
        sink.publish_range(&raw)?;
        Ok(())
    }

    /// Starts a range batch for `collection_id` at the collection's currently
    /// published generation and total count.
    pub fn range_batch(&self, collection_id: i32, offset: i64) -> Option<RangeBatch> {
        let (generation, total_count) = self.ranges.get(collection_id)?;
        Some(RangeBatch::new(
            collection_id,
            generation,
            total_count,
            offset,
        ))
    }

    #[doc(hidden)]
    pub fn push_range_model(&self, batch: &mut RangeBatch, model: impl DynamicViewModel) {
        // The handle's single reference moves straight into the batch.
        let handle = ViewModelHandle::new(model);
        batch.push_raw_model(handle.into_raw());
    }

    /// Publishes (or clears, when `message` is `None`) a validation error for
    /// a property, surfaced to managed code through `INotifyDataErrorInfo`.
    pub fn set_property_error(&self, property_id: i32, message: Option<&str>) -> Result<()> {
        match message {
            Some(message) => self
                .raw2
                .set_property_error(property_id, Some(&utf16(message)))?,
            None => self.raw2.set_property_error(property_id, None)?,
        }
        Ok(())
    }

    /// Submits one immutable worker-safe update batch. Unlike the legacy sink
    /// methods, this does not synchronously enter managed code beyond enqueueing
    /// its dispatcher callback. Applications publishing off the UI thread should
    /// use generated named batch builders rather than per-item methods.
    pub fn submit_batch(&self, batch: ViewModelBatch) -> Result<BatchCompletion> {
        let (completion, callback) = BatchCompletion::channel();
        self.submit(batch, callback).map(|()| completion)
    }

    /// Same as [`ViewModelSink::submit_batch`], with a callback invoked after the
    /// UI dispatcher applies/rejects the batch. The callback is never invoked on
    /// the submitting stack and no view-model lock is held while it runs.
    pub fn submit_batch_with_callback(
        &self,
        batch: ViewModelBatch,
        callback: impl FnOnce(BatchOutcome) + Send + 'static,
    ) -> Result<()> {
        self.submit(batch, move |outcome, error| {
            callback(BatchOutcome::from_wire(outcome, error))
        })
    }

    fn submit(
        &self,
        batch: ViewModelBatch,
        callback: impl FnOnce(i32, i32) + Send + 'static,
    ) -> Result<()> {
        // Resolve the optional capability before taking ownership of the batch,
        // so a v1/v2-only host leaves the caller's nested candidates untouched.
        let sink = self.batch_sink()?.clone();
        let ViewModelBatch {
            generation,
            operations,
            delta,
        } = batch;
        let nested = self.nested.clone();
        let raw = sys::rust_vm_update_batch(
            generation,
            operations,
            // Ownership moves on the managed ownership commit, which lands
            // between the state commit and the notifications. The completion
            // callback below deliberately no longer reconciles anything.
            Some(Box::new(move || reconcile_nested(&nested, delta))),
            Some(Box::new(callback)),
        );
        sink.submit_batch(&raw)?;
        Ok(())
    }
}

/// Result delivered asynchronously for a batch submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchOutcome {
    Applied,
    Stale,
    Cancelled,
    Error(i32),
}

impl BatchOutcome {
    fn from_wire(outcome: i32, error: i32) -> Self {
        match outcome {
            0 => Self::Applied,
            1 => Self::Stale,
            2 => Self::Cancelled,
            _ => Self::Error(error),
        }
    }
}

/// A handle to an asynchronous batch completion. Dropping it is safe; Rust's
/// batch object still owns the sender until managed code reaches a terminal
/// outcome. `try_take` is useful in executor-neutral applications.
pub struct BatchCompletion(Receiver<BatchOutcome>);

impl BatchCompletion {
    fn channel() -> (Self, sys::RustVmBatchCompletion) {
        let (sender, receiver) = mpsc::channel();
        (
            Self(receiver),
            Box::new(move |outcome, error| {
                let _ = sender.send(BatchOutcome::from_wire(outcome, error));
            }),
        )
    }

    pub fn wait(self) -> std::result::Result<BatchOutcome, mpsc::RecvError> {
        self.0.recv()
    }

    pub fn try_take(&self) -> std::result::Result<BatchOutcome, TryRecvError> {
        self.0.try_recv()
    }
}

/// Immutable IR-independent batch payload. Its builders own all strings and
/// COM references, so after `submit_batch` starts it is read-only and needs no
/// Rust locks. Generated model-specific wrappers expose named methods over this
/// type; application code should not use the numeric constructor directly.
#[derive(Default)]
pub struct ViewModelBatch {
    generation: i64,
    operations: Vec<sys::RustVmUpdate>,
    delta: Vec<NestedBatchDelta>,
}

impl ViewModelBatch {
    #[doc(hidden)]
    pub fn new(generation: i64) -> Self {
        Self {
            generation,
            operations: Vec::new(),
            delta: Vec::new(),
        }
    }

    #[doc(hidden)]
    pub fn push(&mut self, update: sys::RustVmUpdate) {
        self.operations.push(update);
    }

    #[doc(hidden)]
    pub fn push_string(&mut self, kind: i32, target_id: i32, index: i32, value: impl AsRef<str>) {
        let mut update = sys::RustVmUpdate::new(kind, target_id);
        update.index = index;
        update.text = Some(value.as_ref().to_owned());
        self.push(update);
    }

    #[doc(hidden)]
    pub fn push_integer(&mut self, target_id: i32, value: i64) {
        let mut update = sys::RustVmUpdate::new(2, target_id);
        update.integer = value;
        self.push(update);
    }

    #[doc(hidden)]
    pub fn push_null(&mut self, target_id: i32) {
        self.push(sys::RustVmUpdate::new(5, target_id));
    }

    #[doc(hidden)]
    pub fn push_boolean(&mut self, kind: i32, target_id: i32, value: bool) {
        let mut update = sys::RustVmUpdate::new(kind, target_id);
        update.boolean = i32::from(value);
        self.push(update);
    }

    #[doc(hidden)]
    pub fn push_double(&mut self, target_id: i32, value: f64) {
        let mut update = sys::RustVmUpdate::new(4, target_id);
        update.double = value;
        self.push(update);
    }

    #[doc(hidden)]
    pub fn push_indices(&mut self, kind: i32, target_id: i32, index: i32, index2: i32) {
        let mut update = sys::RustVmUpdate::new(kind, target_id);
        update.index = index;
        update.index2 = index2;
        self.push(update);
    }

    /// Pushes an index-only operation on a nested-model collection, together
    /// with the matching ownership delta.
    #[doc(hidden)]
    pub fn push_model_indices(&mut self, kind: i32, target_id: i32, index: i32, index2: i32) {
        self.push_indices(kind, target_id, index, index2);
        match kind {
            13 => self
                .delta
                .push(NestedBatchDelta::Remove(target_id, index as usize)),
            14 => self.delta.push(NestedBatchDelta::Move(
                target_id,
                index as usize,
                index2 as usize,
            )),
            _ => {}
        }
    }

    #[doc(hidden)]
    pub fn push_model_clear(&mut self, target_id: i32) {
        self.push(sys::RustVmUpdate::new(19, target_id));
        self.delta.push(NestedBatchDelta::Clear(target_id));
    }

    /// Clears a nested-model *property*: managed treats a model property as
    /// always nullable, so `SetNull` is the clearing operation for it.
    #[doc(hidden)]
    pub fn push_model_null(&mut self, target_id: i32) {
        self.push(sys::RustVmUpdate::new(5, target_id));
        self.delta.push(NestedBatchDelta::Set(target_id, None));
    }

    /// Clears a property's validation error. The boolean flag is what keeps an
    /// empty message distinguishable from "no message" over the wire.
    #[doc(hidden)]
    pub fn push_clear_error(&mut self, target_id: i32) {
        let mut update = sys::RustVmUpdate::new(18, target_id);
        update.boolean = 1;
        self.push(update);
    }

    #[doc(hidden)]
    pub fn push_string_snapshot<S: AsRef<str>>(
        &mut self,
        target_id: i32,
        values: impl IntoIterator<Item = S>,
    ) {
        let mut update = sys::RustVmUpdate::new(15, target_id);
        update.snapshot_strings = Some(
            values
                .into_iter()
                .map(|value| value.as_ref().to_owned())
                .collect(),
        );
        self.push(update);
    }

    #[doc(hidden)]
    pub fn push_model(
        &mut self,
        kind: i32,
        target_id: i32,
        index: i32,
        model: impl DynamicViewModel,
    ) {
        let mut update = sys::RustVmUpdate::new(kind, target_id);
        update.index = index;
        let handle = ViewModelHandle::new(model);
        update.model = Some(handle.raw.clone());
        // The update always ships, even for a tag with no ownership delta, so a
        // batch can never silently lose an operation managed code expects.
        match kind {
            6 => self
                .delta
                .push(NestedBatchDelta::Set(target_id, Some(handle))),
            8 => self.delta.push(NestedBatchDelta::Add(target_id, handle)),
            10 => self
                .delta
                .push(NestedBatchDelta::Insert(target_id, index as usize, handle)),
            12 => self
                .delta
                .push(NestedBatchDelta::Replace(target_id, index as usize, handle)),
            _ => {}
        }
        self.push(update);
    }

    #[doc(hidden)]
    pub fn push_model_snapshot<M: DynamicViewModel>(
        &mut self,
        target_id: i32,
        models: impl IntoIterator<Item = M>,
    ) {
        let mut update = sys::RustVmUpdate::new(16, target_id);
        let handles: Vec<_> = models.into_iter().map(ViewModelHandle::new).collect();
        update.snapshot_models = Some(handles.iter().map(|handle| handle.raw.clone()).collect());
        self.delta
            .push(NestedBatchDelta::Snapshot(target_id, handles));
        self.push(update);
    }
}

pub trait DynamicViewModel: Send + 'static {
    fn attach(&mut self, sink: ViewModelSink) -> Result<()>;
    fn detach(&mut self) -> Result<()>;
    fn set_string(&mut self, property_id: i32, value: String) -> Result<()>;
    fn set_integer(&mut self, property_id: i32, value: i64) -> Result<()>;
    fn set_boolean(&mut self, property_id: i32, value: bool) -> Result<()>;
    fn set_double(&mut self, property_id: i32, value: f64) -> Result<()>;
    fn execute(&mut self, command_id: i32, parameter: Option<String>) -> Result<()>;
    fn begin_async(&mut self, command_id: i32, parameter: Option<String>) -> Result<()>;

    /// Starts a tracked async invocation with a cancellation token whose
    /// handle managed code can cancel. The default forwards to
    /// [`DynamicViewModel::begin_async`], so a model that predates stage 30
    /// keeps working -- it simply ignores cancellation.
    fn begin_async_tracked(
        &mut self,
        command_id: i32,
        parameter: Option<String>,
        _token: CancellationToken,
    ) -> Result<()> {
        self.begin_async(command_id, parameter)
    }

    /// Realizes one page of a windowed collection. Called on the runtime's
    /// dedicated range thread, never on the UI thread, so it may take as long
    /// as the dataset needs. The default rejects the request, which is what a
    /// model that declares no windowed collection should do.
    fn request_range(&mut self, _request: RangeRequest) -> Result<()> {
        Err(Error::Abi(sys::Error(sys::E_NOTIMPL)))
    }
}

struct ViewModelHandle {
    raw: sys::ComPtr<sys::IAvnRustViewModel>,
}

impl ViewModelHandle {
    /// Consumes the handle, transferring its single COM reference to the
    /// caller. Used for windowed range rows, whose managed lifetime Rust does
    /// not track.
    fn into_raw(self) -> sys::ComPtr<sys::IAvnRustViewModel> {
        self.raw
    }

    fn new(model: impl DynamicViewModel) -> Self {
        let model = Arc::new(Mutex::new(model));
        let ranges = Arc::new(RangeStates::default());
        let completions = Arc::new(CompletionSlots::default());
        let cancellations = Arc::new(CancellationRegistry::default());
        let queue = Arc::new(RangeQueue::default());
        let attach_model = model.clone();
        let detach_model = model.clone();
        let string_model = model.clone();
        let integer_model = model.clone();
        let boolean_model = model.clone();
        let double_model = model.clone();
        let execute_model = model.clone();
        let async_model = model.clone();
        let tracked_model = model.clone();
        let range_model = model;
        let attach_ranges = ranges.clone();
        let attach_completions = completions.clone();
        let state_ranges = ranges;
        let guard = RangeQueueGuard(queue);
        let tracked_cancellations = cancellations.clone();
        let raw = sys::rust_view_model_with_control(
            sys::RustViewModelCallbacks {
                attach: Box::new(move |sink| {
                    map_result((|| {
                        let sink = ViewModelSink::with_state(
                            sink,
                            attach_ranges.clone(),
                            attach_completions.clone(),
                        )?;
                        attach_model
                            .lock()
                            .expect("Rust view-model lock poisoned")
                            .attach(sink)
                    })())
                }),
                detach: Box::new(move || {
                    map_result(
                        detach_model
                            .lock()
                            .expect("Rust view-model lock poisoned")
                            .detach(),
                    )
                }),
                set_string: Box::new(move |id, value| {
                    map_result(
                        string_model
                            .lock()
                            .expect("Rust view-model lock poisoned")
                            .set_string(id, value),
                    )
                }),
                set_integer: Box::new(move |id, value| {
                    map_result(
                        integer_model
                            .lock()
                            .expect("Rust view-model lock poisoned")
                            .set_integer(id, value),
                    )
                }),
                set_boolean: Box::new(move |id, value| {
                    map_result(
                        boolean_model
                            .lock()
                            .expect("Rust view-model lock poisoned")
                            .set_boolean(id, value),
                    )
                }),
                set_double: Box::new(move |id, value| {
                    map_result(
                        double_model
                            .lock()
                            .expect("Rust view-model lock poisoned")
                            .set_double(id, value),
                    )
                }),
                execute: Box::new(move |id, parameter| {
                    map_result(
                        execute_model
                            .lock()
                            .expect("Rust view-model lock poisoned")
                            .execute(id, parameter),
                    )
                }),
                begin_async: Box::new(move |id, parameter| {
                    map_result(
                        async_model
                            .lock()
                            .expect("Rust view-model lock poisoned")
                            .begin_async(id, parameter),
                    )
                }),
            },
            sys::RustViewModelControlCallbacks {
                // Reads only the range-state map; it never touches the
                // view-model lock, so the UI thread cannot queue behind a
                // running Rust worker.
                get_range_state: Box::new(move |collection_id| {
                    state_ranges
                        .get(collection_id)
                        .ok_or(sys::Error(sys::E_INVALIDARG))
                }),
                // Enqueues and returns; the dedicated drain thread is the only
                // place the model is locked for a range request. The queue
                // guard lives in this closure, so releasing the COM object
                // closes the queue and the drain thread exits.
                request_range: Box::new(move |collection_id, offset, length, generation| {
                    let request = RangeRequest {
                        collection_id,
                        offset,
                        length,
                        generation,
                    };
                    let enqueued = guard.0.enqueue(request);
                    if enqueued.start {
                        let worker = guard.0.clone();
                        let model = range_model.clone();
                        std::thread::Builder::new()
                            .name("avalonia-rust-range".to_owned())
                            .spawn(move || {
                                while let Some(request) = worker.take() {
                                    let _ = model
                                        .lock()
                                        .expect("Rust view-model lock poisoned")
                                        .request_range(request);
                                }
                            })
                            .map_err(|_| sys::Error(sys::E_FAIL))?;
                        // Latched only now, so a failed spawn leaves the queue
                        // unstarted and the next request tries again.
                        guard.0.mark_started();
                    }
                    Ok(enqueued.dropped.unwrap_or((0, -1)))
                }),
                // Flips a flag; never blocks behind the worker it cancels.
                cancel_async: Box::new(move |_, operation_id| {
                    cancellations.cancel(operation_id);
                    Ok(())
                }),
            },
            Some(Box::new(move |id, parameter, operation_id| {
                let token = tracked_cancellations.create(operation_id);
                map_result(
                    tracked_model
                        .lock()
                        .expect("Rust view-model lock poisoned")
                        .begin_async_tracked(id, parameter, token),
                )
            })),
        );
        Self { raw }
    }
}

impl AppScope {
    pub fn mount_dynamic_view_model(
        &self,
        view_id: i32,
        model: impl DynamicViewModel,
    ) -> Result<()> {
        let model = ViewModelHandle::new(model);
        let raw = self
            .application()
            .create_rust_vm_window(view_id, &model.raw)?;
        self.retain_object(model);
        self.mount(Window { raw })
    }
}

fn utf16(value: impl AsRef<str>) -> Vec<u16> {
    value.as_ref().encode_utf16().chain(Some(0)).collect()
}

fn map_result(result: Result<()>) -> sys::Result<()> {
    result.map_err(|error| match error {
        Error::Abi(error) => error,
        _ => sys::Error(sys::E_FAIL),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    /// A minimal `DynamicViewModel` whose `Drop` increments a shared counter,
    /// so tests can assert that nested handles are actually released (not
    /// merely unreferenced from the managed side) when they are replaced,
    /// removed, or the owning sink/slots are dropped.
    struct CountedModel {
        alive: Arc<AtomicUsize>,
    }

    impl CountedModel {
        fn new(alive: Arc<AtomicUsize>) -> Self {
            alive.fetch_add(1, Ordering::SeqCst);
            Self { alive }
        }
    }

    impl Drop for CountedModel {
        fn drop(&mut self) {
            self.alive.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl DynamicViewModel for CountedModel {
        fn attach(&mut self, _sink: ViewModelSink) -> Result<()> {
            Ok(())
        }
        fn detach(&mut self) -> Result<()> {
            Ok(())
        }
        fn set_string(&mut self, _property_id: i32, _value: String) -> Result<()> {
            Ok(())
        }
        fn set_integer(&mut self, _property_id: i32, _value: i64) -> Result<()> {
            Ok(())
        }
        fn set_boolean(&mut self, _property_id: i32, _value: bool) -> Result<()> {
            Ok(())
        }
        fn set_double(&mut self, _property_id: i32, _value: f64) -> Result<()> {
            Ok(())
        }
        fn execute(&mut self, _command_id: i32, _parameter: Option<String>) -> Result<()> {
            Ok(())
        }
        fn begin_async(&mut self, _command_id: i32, _parameter: Option<String>) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn nested_property_slot_replaces_and_drops_the_previous_handle() {
        let alive = Arc::new(AtomicUsize::new(0));
        let slots = NestedSlots::default();

        slots.set_property(
            1,
            Some(ViewModelHandle::new(CountedModel::new(alive.clone()))),
        );
        assert_eq!(1, alive.load(Ordering::SeqCst));

        // Replacing must drop the previous handle, not accumulate it.
        slots.set_property(
            1,
            Some(ViewModelHandle::new(CountedModel::new(alive.clone()))),
        );
        assert_eq!(1, alive.load(Ordering::SeqCst));

        slots.set_property(1, None);
        assert_eq!(0, alive.load(Ordering::SeqCst));
    }

    #[test]
    fn nested_collection_slots_track_insert_move_replace_remove_and_clear() {
        let alive = Arc::new(AtomicUsize::new(0));
        let slots = NestedSlots::default();
        let make = || ViewModelHandle::new(CountedModel::new(alive.clone()));

        slots.push_collection_item(1, make());
        slots.push_collection_item(1, make());
        slots.insert_collection_item(1, 0, make());
        assert_eq!(3, alive.load(Ordering::SeqCst));

        slots.replace_collection_item(1, 0, make());
        assert_eq!(
            3,
            alive.load(Ordering::SeqCst),
            "replace must not leak the old handle"
        );

        slots.move_collection_item(1, 0, 2);
        assert_eq!(
            3,
            alive.load(Ordering::SeqCst),
            "move must not drop or duplicate handles"
        );

        slots.remove_collection_item(1, 0);
        assert_eq!(2, alive.load(Ordering::SeqCst));

        slots.clear_collection(1);
        assert_eq!(0, alive.load(Ordering::SeqCst));
    }

    #[test]
    fn scalar_collection_operations_do_not_touch_nested_slots() {
        let alive = Arc::new(AtomicUsize::new(0));
        let slots = NestedSlots::default();
        slots.push_collection_item(2, ViewModelHandle::new(CountedModel::new(alive.clone())));

        // String collection changes deliberately have no nested-slot operation:
        // they must not look up collection 1 in the model-only slot map.
        assert_eq!(1, alive.load(Ordering::SeqCst));
        assert_eq!(1, slots.collections.lock_slots()[&2].len());

        slots.clear_collection(2);
        assert_eq!(0, alive.load(Ordering::SeqCst));
    }

    #[test]
    fn dropping_the_nested_slots_releases_every_tracked_handle() {
        let alive = Arc::new(AtomicUsize::new(0));
        {
            let slots = NestedSlots::default();
            slots.set_property(
                1,
                Some(ViewModelHandle::new(CountedModel::new(alive.clone()))),
            );
            slots.push_collection_item(2, ViewModelHandle::new(CountedModel::new(alive.clone())));
            slots.push_collection_item(2, ViewModelHandle::new(CountedModel::new(alive.clone())));
            assert_eq!(3, alive.load(Ordering::SeqCst));
        }
        assert_eq!(0, alive.load(Ordering::SeqCst));
    }

    #[test]
    fn repeated_mount_style_create_and_drop_cycles_do_not_leak() {
        // Mirrors the "repeatedly attach/detach models" conformance
        // requirement at the `ViewModelHandle` level: creating and dropping
        // many handles (as `AppScope::mount_dynamic_view_model` and
        // `ViewModelSink::set_model`/`add_model` do internally) must not
        // accumulate live models.
        let alive = Arc::new(AtomicUsize::new(0));
        for _ in 0..200 {
            let handle = ViewModelHandle::new(CountedModel::new(alive.clone()));
            assert_eq!(1, alive.load(Ordering::SeqCst));
            drop(handle);
        }
        assert_eq!(0, alive.load(Ordering::SeqCst));
    }

    /// The wire outcome tags, mirroring `Avalonia.Rust.RustVmBatchOutcome`.
    const APPLIED: i32 = 0;
    const STALE: i32 = 1;
    const CANCELLED: i32 = 2;
    const ERROR: i32 = 3;

    fn batch_with_models(generation: i64, alive: &Arc<AtomicUsize>) -> ViewModelBatch {
        let mut batch = ViewModelBatch::new(generation);
        batch.push_model(6, 1, 0, CountedModel::new(alive.clone()));
        batch.push_model(8, 2, 0, CountedModel::new(alive.clone()));
        batch
    }

    /// Builds the real immutable nano-COM batch exactly as
    /// `ViewModelSink::submit` does, so tests exercise the shipped ownership and
    /// completion plumbing rather than a stand-in.
    fn raw_batch(
        batch: ViewModelBatch,
        nested: &Arc<NestedSlots>,
    ) -> (sys::ComPtr<sys::IAvnRustVmUpdateBatch>, BatchCompletion) {
        let (completion, callback) = BatchCompletion::channel();
        let ViewModelBatch {
            generation,
            operations,
            delta,
        } = batch;
        let nested = nested.clone();
        let raw = sys::rust_vm_update_batch(
            generation,
            operations,
            Some(Box::new(move || reconcile_nested(&nested, delta))),
            Some(callback),
        );
        (raw, completion)
    }

    #[test]
    fn committing_ownership_reconciles_nested_slots_exactly_once() {
        let alive = Arc::new(AtomicUsize::new(0));
        let slots = Arc::new(NestedSlots::default());
        let (raw, completion) = raw_batch(batch_with_models(1, &alive), &slots);
        assert_eq!(2, alive.load(Ordering::SeqCst), "the batch owns candidates");
        assert!(slots.properties.lock_slots().is_empty());

        let ownership = raw
            .query_interface::<sys::IAvnRustVmUpdateBatch2>()
            .expect("the batch must expose its ownership capability");
        ownership.commit_ownership().expect("commit must succeed");

        assert_eq!(1, slots.properties.lock_slots().len());
        assert_eq!(1, slots.collections.lock_slots()[&2].len());

        // A second call is a no-op: the callback was consumed by the first.
        ownership.commit_ownership().expect("commit must stay safe");
        assert_eq!(1, slots.properties.lock_slots().len());
        assert_eq!(1, slots.collections.lock_slots()[&2].len());

        // Completion still arrives after ownership, and only once.
        raw.complete(APPLIED, 0).expect("complete must succeed");
        drop(ownership);
        drop(raw);
        assert_eq!(Ok(BatchOutcome::Applied), completion.wait());
        assert_eq!(2, alive.load(Ordering::SeqCst), "committed handles live on");

        // A second applied batch replaces the property slot and appends to the
        // collection, so ownership tracks managed state rather than accumulating.
        let (second, _) = raw_batch(batch_with_models(2, &alive), &slots);
        second
            .query_interface::<sys::IAvnRustVmUpdateBatch2>()
            .expect("capability")
            .commit_ownership()
            .expect("commit");
        drop(second);
        assert_eq!(3, alive.load(Ordering::SeqCst));
        assert_eq!(2, slots.collections.lock_slots()[&2].len());
    }

    #[test]
    fn a_batch_that_never_commits_ownership_drops_its_candidates_unchanged() {
        let alive = Arc::new(AtomicUsize::new(0));
        let slots = Arc::new(NestedSlots::default());
        slots.set_property(
            1,
            Some(ViewModelHandle::new(CountedModel::new(alive.clone()))),
        );
        slots.push_collection_item(2, ViewModelHandle::new(CountedModel::new(alive.clone())));
        assert_eq!(2, alive.load(Ordering::SeqCst));

        for outcome in [STALE, CANCELLED, ERROR] {
            let (raw, completion) = raw_batch(batch_with_models(7, &alive), &slots);
            assert_eq!(4, alive.load(Ordering::SeqCst));

            // Stale/cancel/error never reach the ownership commit.
            raw.complete(outcome, 0).expect("complete must succeed");
            drop(raw);

            assert_eq!(
                2,
                alive.load(Ordering::SeqCst),
                "outcome {outcome} must drop its own candidates only"
            );
            assert_eq!(1, slots.properties.lock_slots().len());
            assert_eq!(1, slots.collections.lock_slots()[&2].len());
            assert!(matches!(
                completion.wait(),
                Ok(BatchOutcome::Stale | BatchOutcome::Cancelled | BatchOutcome::Error(_))
            ));
        }
    }

    #[test]
    fn a_nested_update_published_after_ownership_applies_on_top_of_the_batch() {
        // Mirrors an observer that synchronously publishes a nested update from
        // a batch notification: managed code commits ownership first, so the
        // synchronous update lands on the batch's slots instead of racing them.
        let alive = Arc::new(AtomicUsize::new(0));
        let slots = Arc::new(NestedSlots::default());
        let mut batch = ViewModelBatch::new(1);
        batch.push_model_snapshot(
            2,
            (0..2)
                .map(|_| CountedModel::new(alive.clone()))
                .collect::<Vec<_>>(),
        );
        let (raw, _completion) = raw_batch(batch, &slots);

        raw.query_interface::<sys::IAvnRustVmUpdateBatch2>()
            .expect("capability")
            .commit_ownership()
            .expect("commit");
        assert_eq!(2, slots.collections.lock_slots()[&2].len());

        // The "observer" update.
        slots.push_collection_item(2, ViewModelHandle::new(CountedModel::new(alive.clone())));
        assert_eq!(3, slots.collections.lock_slots()[&2].len());

        raw.complete(APPLIED, 0).expect("complete");
        drop(raw);
        assert_eq!(
            3,
            alive.load(Ordering::SeqCst),
            "the batch's handles and the observer's must both be tracked"
        );
        assert_eq!(3, slots.collections.lock_slots()[&2].len());
    }

    #[test]
    fn synchronous_collection_updates_work_after_an_applied_batch_snapshot() {
        let alive = Arc::new(AtomicUsize::new(0));
        let slots = Arc::new(NestedSlots::default());
        let mut batch = ViewModelBatch::new(1);
        batch.push_model_snapshot(
            2,
            (0..3)
                .map(|_| CountedModel::new(alive.clone()))
                .collect::<Vec<_>>(),
        );
        let (raw, _completion) = raw_batch(batch, &slots);
        raw.query_interface::<sys::IAvnRustVmUpdateBatch2>()
            .expect("capability")
            .commit_ownership()
            .expect("commit");
        raw.complete(APPLIED, 0).expect("complete");
        drop(raw);
        assert_eq!(3, alive.load(Ordering::SeqCst));

        // The snapshot must leave a real slot list behind so the synchronous
        // v2 path keeps working without panicking on a missing entry.
        slots.replace_collection_item(2, 1, ViewModelHandle::new(CountedModel::new(alive.clone())));
        assert_eq!(3, alive.load(Ordering::SeqCst));
        slots.move_collection_item(2, 0, 2);
        assert_eq!(3, alive.load(Ordering::SeqCst));
        slots.remove_collection_item(2, 0);
        assert_eq!(2, alive.load(Ordering::SeqCst));
        slots.clear_collection(2);
        assert_eq!(0, alive.load(Ordering::SeqCst));
    }

    #[test]
    fn out_of_range_slot_updates_are_ignored_rather_than_panicking() {
        let alive = Arc::new(AtomicUsize::new(0));
        let slots = NestedSlots::default();
        slots.push_collection_item(2, ViewModelHandle::new(CountedModel::new(alive.clone())));

        slots.remove_collection_item(2, 9);
        slots.remove_collection_item(7, 0);
        slots.move_collection_item(2, 0, 9);
        slots.move_collection_item(7, 0, 0);
        assert_eq!(1, alive.load(Ordering::SeqCst));
    }

    #[test]
    fn every_model_operation_is_emitted_even_without_an_ownership_delta() {
        let alive = Arc::new(AtomicUsize::new(0));
        let mut batch = ViewModelBatch::new(1);
        // 16 (a model snapshot tag) has no per-item delta in `push_model`; the
        // operation must still reach managed code.
        batch.push_model(16, 2, 0, CountedModel::new(alive.clone()));
        assert_eq!(1, batch.operations.len());
        assert_eq!(16, batch.operations[0].kind);
        assert!(batch.operations[0].model.is_some());
        assert!(batch.delta.is_empty());
    }

    #[test]
    fn clearing_a_model_property_and_an_error_uses_distinguishable_wire_shapes() {
        let mut batch = ViewModelBatch::new(1);
        batch.push_model_null(7);
        batch.push_clear_error(3);
        batch.push_string(18, 4, 0, "");

        assert_eq!(5, batch.operations[0].kind);
        assert!(matches!(batch.delta[0], NestedBatchDelta::Set(7, None)));

        // An explicit clear is flagged; an empty message is not.
        assert_eq!(18, batch.operations[1].kind);
        assert_eq!(1, batch.operations[1].boolean);
        assert_eq!(18, batch.operations[2].kind);
        assert_eq!(0, batch.operations[2].boolean);
        assert_eq!(Some(String::new()), batch.operations[2].text);
    }

    /// A hand-rolled sink whose `QueryInterface` answers a chosen HRESULT for
    /// the v3 batch IID while still satisfying the mandatory v2 query, so the
    /// lazy capability cache can be tested for both "absent" and "failed".
    #[repr(C)]
    struct FakeSink {
        vtbl: *const FakeSinkVtbl,
        references: AtomicU32,
        sink3_result: i32,
        sink4_result: i32,
        sink5_result: i32,
        queries: Arc<AtomicUsize>,
        shape_queries: Arc<AtomicUsize>,
        number_queries: Arc<AtomicUsize>,
    }

    #[repr(C)]
    struct FakeSinkVtbl {
        query_interface: unsafe extern "system" fn(
            *mut FakeSink,
            *const sys::Guid,
            *mut *mut std::ffi::c_void,
        ) -> i32,
        add_ref: unsafe extern "system" fn(*mut FakeSink) -> u32,
        release: unsafe extern "system" fn(*mut FakeSink) -> u32,
    }

    static FAKE_SINK_VTBL: FakeSinkVtbl = FakeSinkVtbl {
        query_interface: fake_query_interface,
        add_ref: fake_add_ref,
        release: fake_release,
    };

    unsafe extern "system" fn fake_query_interface(
        this: *mut FakeSink,
        iid: *const sys::Guid,
        result: *mut *mut std::ffi::c_void,
    ) -> i32 {
        *result = std::ptr::null_mut();

        if *iid == <sys::IAvnRustVmSink3 as sys::ComInterface>::IID {
            (*this).queries.fetch_add(1, Ordering::SeqCst);
            return (*this).sink3_result;
        }
        if *iid == <sys::IAvnRustVmSink4 as sys::ComInterface>::IID {
            (*this).shape_queries.fetch_add(1, Ordering::SeqCst);
            return (*this).sink4_result;
        }
        if *iid == <sys::IAvnRustVmSink5 as sys::ComInterface>::IID {
            (*this).number_queries.fetch_add(1, Ordering::SeqCst);
            return (*this).sink5_result;
        }
        // Everything else (IUnknown, v1 and v2) resolves to the same object;
        // only the header layout matters for these tests.
        fake_add_ref(this);
        *result = this.cast();
        sys::S_OK
    }

    unsafe extern "system" fn fake_add_ref(this: *mut FakeSink) -> u32 {
        (*this).references.fetch_add(1, Ordering::Relaxed) + 1
    }

    unsafe extern "system" fn fake_release(this: *mut FakeSink) -> u32 {
        let remaining = (*this).references.fetch_sub(1, Ordering::Release) - 1;
        if remaining == 0 {
            drop(Box::from_raw(this));
        }
        remaining
    }

    fn fake_sink(sink3_result: i32) -> (ViewModelSink, Arc<AtomicUsize>) {
        let (sink, queries, _) = fake_sink_with_shapes(sink3_result, sys::E_NOINTERFACE);
        (sink, queries)
    }

    fn fake_sink_with_shapes(
        sink3_result: i32,
        sink4_result: i32,
    ) -> (ViewModelSink, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let (sink, queries, shape_queries, _) =
            fake_sink_with_capabilities(sink3_result, sink4_result, sys::E_NOINTERFACE);
        (sink, queries, shape_queries)
    }

    fn fake_sink_with_capabilities(
        sink3_result: i32,
        sink4_result: i32,
        sink5_result: i32,
    ) -> (
        ViewModelSink,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
    ) {
        let queries = Arc::new(AtomicUsize::new(0));
        let shape_queries = Arc::new(AtomicUsize::new(0));
        let number_queries = Arc::new(AtomicUsize::new(0));
        let object = Box::new(FakeSink {
            vtbl: &FAKE_SINK_VTBL,
            references: AtomicU32::new(1),
            sink3_result,
            sink4_result,
            sink5_result,
            queries: queries.clone(),
            shape_queries: shape_queries.clone(),
            number_queries: number_queries.clone(),
        });
        let raw = unsafe {
            sys::ComPtr::<sys::IAvnRustVmSink>::from_raw(Box::into_raw(object).cast())
                .expect("non-null")
        };
        (
            ViewModelSink::with_state(
                raw,
                Arc::new(RangeStates::default()),
                Arc::new(CompletionSlots::default()),
            )
            .expect("v2 must resolve"),
            queries,
            shape_queries,
            number_queries,
        )
    }

    /// An absent v5 capability must be an explicit, cached ABI error on every
    /// value-carrying call rather than a silently dropped element.
    #[test]
    fn an_absent_number_capability_is_cached_as_e_nointerface() {
        let (sink, _, _, number_queries) =
            fake_sink_with_capabilities(sys::E_NOINTERFACE, sys::E_NOINTERFACE, sys::E_NOINTERFACE);

        for _ in 0..3 {
            match sink.add_double(8, 1.0) {
                Err(Error::Abi(error)) => assert_eq!(sys::E_NOINTERFACE, error.0),
                other => panic!("expected E_NOINTERFACE, got {other:?}"),
            }
        }
        match sink.add_integer(9, 1) {
            Err(Error::Abi(error)) => assert_eq!(sys::E_NOINTERFACE, error.0),
            other => panic!("expected E_NOINTERFACE, got {other:?}"),
        }
        assert!(!sink.supports_number_collections());
        assert_eq!(
            1,
            number_queries.load(Ordering::SeqCst),
            "the query is cached"
        );
    }

    #[test]
    fn a_failed_number_capability_query_is_reported_verbatim_not_as_absence() {
        let (sink, _, _, _) =
            fake_sink_with_capabilities(sys::E_NOINTERFACE, sys::E_NOINTERFACE, sys::E_FAIL);

        match sink.replace_integer(9, 0, 1) {
            Err(Error::Abi(error)) => assert_eq!(sys::E_FAIL, error.0),
            other => panic!("expected E_FAIL, got {other:?}"),
        }
        assert!(!sink.supports_number_collections());
    }

    #[test]
    fn an_absent_batch_capability_is_cached_as_e_nointerface() {
        let (sink, queries) = fake_sink(sys::E_NOINTERFACE);

        for _ in 0..3 {
            match sink.batch_sink() {
                Err(Error::Abi(error)) => assert_eq!(sys::E_NOINTERFACE, error.0),
                other => panic!("expected E_NOINTERFACE, got {other:?}"),
            }
        }
        assert_eq!(1, queries.load(Ordering::SeqCst), "the query is cached");
    }

    #[test]
    fn a_failed_batch_capability_query_is_reported_verbatim_not_as_absence() {
        let (sink, queries) = fake_sink(sys::E_FAIL);

        for _ in 0..3 {
            match sink.batch_sink() {
                Err(Error::Abi(error)) => assert_eq!(
                    sys::E_FAIL,
                    error.0,
                    "a transport failure must not be downgraded to E_NOINTERFACE"
                ),
                other => panic!("expected E_FAIL, got {other:?}"),
            }
        }
        assert_eq!(1, queries.load(Ordering::SeqCst), "the failure is cached");
    }
    /// The stage 30 capability is optional and resolved exactly once, with the
    /// same discipline as the batch capability: a producer attached to a host
    /// that predates it reports `E_NOINTERFACE` from every richer-shape call
    /// instead of silently dropping the update.
    #[test]
    fn an_absent_richer_shapes_capability_is_reported_and_cached() {
        let (sink, _, shape_queries) =
            fake_sink_with_shapes(sys::E_NOINTERFACE, sys::E_NOINTERFACE);

        assert!(!sink.supports_richer_shapes());
        for _ in 0..3 {
            match sink.map_set_integer(1, MapKey::Text("Error".to_owned()), 1) {
                Err(Error::Abi(error)) => assert_eq!(sys::E_NOINTERFACE, error.0),
                other => panic!("expected E_NOINTERFACE, got {other:?}"),
            }
        }
        match sink.publish_range_reset(5, 1, 100) {
            Err(Error::Abi(error)) => assert_eq!(sys::E_NOINTERFACE, error.0),
            other => panic!("expected E_NOINTERFACE, got {other:?}"),
        }
        assert_eq!(
            1,
            shape_queries.load(Ordering::SeqCst),
            "the capability query is cached"
        );
    }

    #[test]
    fn a_failed_richer_shapes_query_is_reported_verbatim_not_as_absence() {
        let (sink, _, _) = fake_sink_with_shapes(sys::E_NOINTERFACE, sys::E_FAIL);

        match sink.map_clear(1) {
            Err(Error::Abi(error)) => assert_eq!(sys::E_FAIL, error.0),
            other => panic!("expected E_FAIL, got {other:?}"),
        }
        assert!(!sink.supports_richer_shapes());
    }

    /// A range batch is only meaningful against a published generation, so
    /// `range_batch` refuses to build one for a collection whose identity was
    /// never reset.
    #[test]
    fn a_range_page_requires_a_published_generation() {
        let (sink, _, _) = fake_sink_with_shapes(sys::E_NOINTERFACE, sys::E_NOINTERFACE);

        assert!(sink.range_batch(5, 0).is_none());
    }

    #[test]
    fn a_completion_is_claimed_once_per_invocation() {
        let (sink, _, _) = fake_sink_with_shapes(sys::E_NOINTERFACE, sys::E_NOINTERFACE);
        let first = CancellationToken::none();

        assert!(sink.claim_completion(3, &first));
        assert!(!sink.claim_completion(3, &first));
        assert!(sink.claim_completion(4, &first), "claims are per command");
    }

    /// Map entries own their nested handles exactly like collection items: a
    /// replaced or removed value releases Rust's contribution immediately.
    #[test]
    fn map_slots_release_replaced_and_removed_nested_handles() {
        let alive = Arc::new(AtomicUsize::new(0));
        let slots = NestedSlots::default();

        slots.set_map_value(
            1,
            MapKey::Text("a".to_owned()),
            Some(ViewModelHandle::new(CountedModel::new(alive.clone()))),
        );
        slots.set_map_value(
            1,
            MapKey::Text("b".to_owned()),
            Some(ViewModelHandle::new(CountedModel::new(alive.clone()))),
        );
        assert_eq!(2, alive.load(Ordering::SeqCst));

        slots.set_map_value(
            1,
            MapKey::Text("a".to_owned()),
            Some(ViewModelHandle::new(CountedModel::new(alive.clone()))),
        );
        assert_eq!(
            2,
            alive.load(Ordering::SeqCst),
            "replace drops the previous"
        );

        slots.set_map_value(1, MapKey::Text("b".to_owned()), None);
        assert_eq!(1, alive.load(Ordering::SeqCst));

        slots.clear_map(1);
        assert_eq!(0, alive.load(Ordering::SeqCst));
    }

    #[test]
    fn command_result_slots_replace_and_release_exactly_one_handle() {
        let alive = Arc::new(AtomicUsize::new(0));
        let slots = NestedSlots::default();

        slots.set_command_result(
            3,
            Some(ViewModelHandle::new(CountedModel::new(alive.clone()))),
        );
        slots.set_command_result(
            3,
            Some(ViewModelHandle::new(CountedModel::new(alive.clone()))),
        );
        assert_eq!(1, alive.load(Ordering::SeqCst));

        slots.set_command_result(3, None);
        assert_eq!(0, alive.load(Ordering::SeqCst));
    }
    /// A model that records the range requests the runtime's drain thread
    /// delivers, so the nonblocking contract can be asserted rather than
    /// assumed.
    struct RangeModel {
        seen: Arc<Mutex<Vec<RangeRequest>>>,
        signal: Arc<std::sync::Condvar>,
        delivered_on: Arc<Mutex<Option<std::thread::ThreadId>>>,
    }

    impl DynamicViewModel for RangeModel {
        fn attach(&mut self, _sink: ViewModelSink) -> Result<()> {
            Ok(())
        }
        fn detach(&mut self) -> Result<()> {
            Ok(())
        }
        fn set_string(&mut self, _property_id: i32, _value: String) -> Result<()> {
            Ok(())
        }
        fn set_integer(&mut self, _property_id: i32, _value: i64) -> Result<()> {
            Ok(())
        }
        fn set_boolean(&mut self, _property_id: i32, _value: bool) -> Result<()> {
            Ok(())
        }
        fn set_double(&mut self, _property_id: i32, _value: f64) -> Result<()> {
            Ok(())
        }
        fn execute(&mut self, _command_id: i32, _parameter: Option<String>) -> Result<()> {
            Ok(())
        }
        fn begin_async(&mut self, _command_id: i32, _parameter: Option<String>) -> Result<()> {
            Ok(())
        }
        fn request_range(&mut self, request: RangeRequest) -> Result<()> {
            *self.delivered_on.lock().expect("thread lock") = Some(std::thread::current().id());
            self.seen.lock().expect("seen lock").push(request);
            self.signal.notify_all();
            Ok(())
        }
    }

    /// `IAvnRustRangeSource::RequestRange` is called on the UI thread. It must
    /// return immediately and hand the work to the runtime's own thread, so a
    /// long-running model callback can never stall the UI.
    #[test]
    fn a_range_request_returns_immediately_and_is_delivered_off_the_calling_thread() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let signal = Arc::new(std::sync::Condvar::new());
        let delivered_on = Arc::new(Mutex::new(None));
        let handle = ViewModelHandle::new(RangeModel {
            seen: seen.clone(),
            signal: signal.clone(),
            delivered_on: delivered_on.clone(),
        });
        let source = handle
            .raw
            .query_interface::<sys::IAvnRustRangeSource>()
            .expect("range source");
        let caller = std::thread::current().id();

        // Issue the requests one at a time and wait for each delivery: whether two
        // identical requests queue before the drain thread wakes is timing-dependent
        // on loaded runners, so this test observes one delivery per request instead of
        // racing the queue's coalescing window.
        source.request_range(5, 0, 64, 3).expect("request accepted");
        let mut guard = seen.lock().expect("seen lock");
        while guard.is_empty() {
            let (next, timeout) = signal
                .wait_timeout(guard, std::time::Duration::from_secs(30))
                .expect("condvar");
            assert!(!timeout.timed_out(), "the drain thread never delivered");
            guard = next;
        }
        source
            .request_range(5, 64, 64, 3)
            .expect("request accepted");
        while guard.len() < 2 {
            let (next, timeout) = signal
                .wait_timeout(guard, std::time::Duration::from_secs(30))
                .expect("condvar");
            assert!(!timeout.timed_out(), "the drain thread never delivered");
            guard = next;
        }
        assert_eq!(
            vec![
                RangeRequest {
                    collection_id: 5,
                    offset: 0,
                    length: 64,
                    generation: 3
                },
                RangeRequest {
                    collection_id: 5,
                    offset: 64,
                    length: 64,
                    generation: 3
                },
            ],
            *guard
        );
        drop(guard);
        assert_ne!(
            Some(caller),
            *delivered_on.lock().expect("thread lock"),
            "the model must be locked on the runtime's range thread, not the caller's"
        );
    }

    /// Releasing the view model closes the queue, so the drain thread exits
    /// instead of outliving the model it serves.
    #[test]
    fn releasing_the_view_model_stops_the_range_thread() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let signal = Arc::new(std::sync::Condvar::new());
        let alive = Arc::new(AtomicUsize::new(0));
        let handle = ViewModelHandle::new(TrackedRangeModel {
            inner: RangeModel {
                seen,
                signal,
                delivered_on: Arc::new(Mutex::new(None)),
            },
            alive: alive.clone(),
        });
        alive.store(1, Ordering::SeqCst);
        let source = handle
            .raw
            .query_interface::<sys::IAvnRustRangeSource>()
            .expect("range source");
        source.request_range(5, 0, 8, 1).expect("request accepted");

        drop(source);
        drop(handle);

        // The drain thread holds the last model reference until it observes the
        // closed queue; give it a bounded window to exit.
        for _ in 0..500 {
            if alive.load(Ordering::SeqCst) == 0 {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the range thread kept the model alive after release");
    }

    struct TrackedRangeModel {
        inner: RangeModel,
        alive: Arc<AtomicUsize>,
    }

    impl Drop for TrackedRangeModel {
        fn drop(&mut self) {
            self.alive.store(0, Ordering::SeqCst);
        }
    }

    impl DynamicViewModel for TrackedRangeModel {
        fn attach(&mut self, sink: ViewModelSink) -> Result<()> {
            self.inner.attach(sink)
        }
        fn detach(&mut self) -> Result<()> {
            self.inner.detach()
        }
        fn set_string(&mut self, property_id: i32, value: String) -> Result<()> {
            self.inner.set_string(property_id, value)
        }
        fn set_integer(&mut self, property_id: i32, value: i64) -> Result<()> {
            self.inner.set_integer(property_id, value)
        }
        fn set_boolean(&mut self, property_id: i32, value: bool) -> Result<()> {
            self.inner.set_boolean(property_id, value)
        }
        fn set_double(&mut self, property_id: i32, value: f64) -> Result<()> {
            self.inner.set_double(property_id, value)
        }
        fn execute(&mut self, command_id: i32, parameter: Option<String>) -> Result<()> {
            self.inner.execute(command_id, parameter)
        }
        fn begin_async(&mut self, command_id: i32, parameter: Option<String>) -> Result<()> {
            self.inner.begin_async(command_id, parameter)
        }
        fn request_range(&mut self, request: RangeRequest) -> Result<()> {
            self.inner.request_range(request)
        }
    }
}
