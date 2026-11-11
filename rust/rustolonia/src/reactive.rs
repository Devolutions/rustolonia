//! Single-threaded fine-grained reactive primitives (`Signal`, `Memo`, `Effect`, `NodeRef`)
//! for declarative and imperative UI authoring.
//!
//! Handles are `Copy` indices into the UI-thread reactive arena, so they can be
//! captured directly by multiple event closures (`on_click`, `on_value_changed`)
//! without cloning control handles or wrapping state in `Arc<Mutex<_>>`.

use crate::{AppContext, AppScope, Error, Result};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

struct SignalSlot {
    value: Option<Box<dyn Any>>,
    subscribers: Vec<usize>,
}

type EffectCallback = Box<dyn FnMut() -> Result<()>>;
type ScopeCleanup = Box<dyn FnOnce()>;

struct EffectSlot {
    callback: Option<EffectCallback>,
    dependencies: Vec<usize>,
    scope_id: usize,
    active: bool,
    running: bool,
    queued: bool,
}

struct ScopeSlot {
    parent: Option<usize>,
    children: Vec<usize>,
    effects: Vec<usize>,
    contexts: HashMap<TypeId, Box<dyn Any>>,
    cleanups: Vec<ScopeCleanup>,
    active: bool,
}

#[derive(Default)]
struct ReactiveRuntime {
    signals: Vec<SignalSlot>,
    effects: Vec<EffectSlot>,
    scopes: Vec<ScopeSlot>,
    tracking_stack: Vec<usize>,
    scope_stack: Vec<usize>,
    pending_effects: VecDeque<usize>,
    flushing: bool,
}

#[derive(Default)]
struct UiCallbackPool {
    next_id: u64,
    slots: HashMap<u64, Box<dyn Any>>,
}

static ORPHANED_UI_CALLBACKS: Mutex<Vec<(ThreadId, u64)>> = Mutex::new(Vec::new());

thread_local! {
    static RUNTIME: RefCell<ReactiveRuntime> = RefCell::new(ReactiveRuntime::default());
    static UI_CALLBACKS: RefCell<UiCallbackPool> = RefCell::new(UiCallbackPool::default());
}

fn drain_orphaned_ui_callbacks_for_current_thread() {
    let current = std::thread::current().id();
    let ids: Vec<u64> = {
        let Ok(mut orphaned) = ORPHANED_UI_CALLBACKS.lock() else {
            return;
        };
        if !orphaned.iter().any(|(tid, _)| *tid == current) {
            return;
        }
        let mut collected = Vec::new();
        orphaned.retain(|(tid, id)| {
            if *tid == current {
                collected.push(*id);
                false
            } else {
                true
            }
        });
        collected
    };
    if ids.is_empty() {
        return;
    }
    let removed: Vec<Box<dyn Any>> = UI_CALLBACKS.with(|pool| {
        let mut pool = pool.borrow_mut();
        ids.into_iter()
            .filter_map(|id| pool.slots.remove(&id))
            .collect()
    });
    drop(removed);
}

struct UiCallbackDrop {
    thread: ThreadId,
    id: u64,
}

impl Drop for UiCallbackDrop {
    fn drop(&mut self) {
        if std::thread::current().id() == self.thread {
            let removed = UI_CALLBACKS.with(|pool| pool.borrow_mut().slots.remove(&self.id));
            drop(removed);
        } else if let Ok(mut orphaned) = ORPHANED_UI_CALLBACKS.lock() {
            orphaned.push((self.thread, self.id));
        }
    }
}

/// A `Clone + Send + Sync` handle to a `!Send` closure pinned to its creating UI thread.
///
/// The closure itself never leaves thread-local storage, even if the last handle
/// is dropped from a background thread.
pub(crate) struct UiCallbackHandle<F: 'static> {
    inner: Arc<UiCallbackDrop>,
    scope: ReactiveScope,
    _marker: PhantomData<fn() -> F>,
}

impl<F: 'static> Clone for UiCallbackHandle<F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            scope: self.scope,
            _marker: PhantomData,
        }
    }
}

impl<F: 'static> fmt::Debug for UiCallbackHandle<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UiCallbackHandle")
            .field("id", &self.inner.id)
            .finish()
    }
}

impl<F: 'static> UiCallbackHandle<F> {
    pub(crate) fn new(callback: F) -> Self {
        drain_orphaned_ui_callbacks_for_current_thread();
        let thread = std::thread::current().id();
        let id = UI_CALLBACKS.with(|pool| {
            let mut pool = pool.borrow_mut();
            let id = pool.next_id.wrapping_add(1);
            pool.next_id = id;
            let cell: Rc<RefCell<F>> = Rc::new(RefCell::new(callback));
            pool.slots.insert(id, Box::new(cell));
            id
        });
        Self {
            inner: Arc::new(UiCallbackDrop { thread, id }),
            scope: ReactiveScope::current(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn with<R>(&self, invoke: impl FnOnce(&mut F) -> R) -> Result<R> {
        if std::thread::current().id() != self.inner.thread {
            return Err(Error::NoUiContext);
        }
        if !self.scope.is_active() {
            return Err(Error::NoUiContext);
        }
        drain_orphaned_ui_callbacks_for_current_thread();
        let cell: Option<Rc<RefCell<F>>> = UI_CALLBACKS.with(|pool| {
            pool.borrow()
                .slots
                .get(&self.inner.id)
                .and_then(|boxed| boxed.downcast_ref::<Rc<RefCell<F>>>())
                .cloned()
        });
        let cell = cell.ok_or(Error::NoUiContext)?;
        let mut guard = cell.borrow_mut();
        Ok(self.scope.run(|| invoke(&mut *guard)))
    }
}

// Single-view hosts (browser, Android) keep the session for the process lifetime.
#[cfg_attr(any(target_os = "emscripten", target_os = "android"), allow(dead_code))]
pub(crate) fn clear_reactive_runtime() {
    drain_orphaned_ui_callbacks_for_current_thread();
    let has_root = RUNTIME.with(|runtime| !runtime.borrow().scopes.is_empty());
    if has_root {
        ReactiveScope { id: 0 }.dispose();
    }
    let old = RUNTIME.with(|runtime| std::mem::take(&mut *runtime.borrow_mut()));
    drop(old);
    let old_callbacks = UI_CALLBACKS.with(|pool| std::mem::take(&mut *pool.borrow_mut()));
    drop(old_callbacks);
}

fn ensure_root_scope(rt: &mut ReactiveRuntime) -> usize {
    if rt.scopes.is_empty() {
        rt.scopes.push(ScopeSlot {
            parent: None,
            children: Vec::new(),
            effects: Vec::new(),
            contexts: HashMap::new(),
            cleanups: Vec::new(),
            active: true,
        });
    }
    0
}

fn attach_effect_to_current_scope(rt: &mut ReactiveRuntime, effect_id: usize) {
    let scope_id = rt
        .scope_stack
        .last()
        .copied()
        .unwrap_or_else(|| ensure_root_scope(rt));
    if let Some(scope) = rt.scopes.get_mut(scope_id) {
        if scope.active && !scope.effects.contains(&effect_id) {
            scope.effects.push(effect_id);
        }
    }
    if let Some(effect) = rt.effects.get_mut(effect_id) {
        effect.scope_id = scope_id;
    }
}

fn record_dependency(signal_id: usize) {
    RUNTIME.with(|runtime| {
        let mut rt = runtime.borrow_mut();
        if let Some(&effect_id) = rt.tracking_stack.last() {
            if let Some(effect) = rt.effects.get_mut(effect_id) {
                if !effect.dependencies.contains(&signal_id) {
                    effect.dependencies.push(signal_id);
                }
            }
            if let Some(signal) = rt.signals.get_mut(signal_id) {
                if !signal.subscribers.contains(&effect_id) {
                    signal.subscribers.push(effect_id);
                }
            }
        }
    });
}

fn unlink_effect_dependencies(rt: &mut ReactiveRuntime, effect_id: usize) {
    let deps = if let Some(effect) = rt.effects.get_mut(effect_id) {
        std::mem::take(&mut effect.dependencies)
    } else {
        return;
    };
    for signal_id in deps {
        if let Some(signal) = rt.signals.get_mut(signal_id) {
            signal.subscribers.retain(|&sub| sub != effect_id);
        }
    }
}

fn enqueue_subscribers(signal_id: usize) {
    let should_flush = RUNTIME.with(|runtime| {
        let mut rt = runtime.borrow_mut();
        let subscribers = rt
            .signals
            .get(signal_id)
            .map(|s| s.subscribers.clone())
            .unwrap_or_default();
        for effect_id in subscribers {
            if let Some(effect) = rt.effects.get_mut(effect_id) {
                if effect.active && !effect.queued && !effect.running {
                    effect.queued = true;
                    rt.pending_effects.push_back(effect_id);
                }
            }
        }
        if rt.flushing {
            false
        } else {
            rt.flushing = true;
            true
        }
    });

    if should_flush {
        flush_pending_effects();
    }
}

fn flush_pending_effects() {
    loop {
        let next_effect = RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            let Some(effect_id) = rt.pending_effects.pop_front() else {
                rt.flushing = false;
                return None;
            };
            if let Some(effect) = rt.effects.get_mut(effect_id) {
                effect.queued = false;
                if !effect.active {
                    return Some((effect_id, None));
                }
                effect.running = true;
                let cb = effect.callback.take();
                Some((effect_id, cb))
            } else {
                Some((effect_id, None))
            }
        });

        let Some((effect_id, maybe_cb)) = next_effect else {
            break;
        };

        let Some(mut cb) = maybe_cb else {
            continue;
        };

        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            unlink_effect_dependencies(&mut rt, effect_id);
            rt.tracking_stack.push(effect_id);
            let scope_id = rt.effects[effect_id].scope_id;
            rt.scope_stack.push(scope_id);
        });

        let result = cb();

        let mut callback = Some(cb);
        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            rt.tracking_stack.pop();
            rt.scope_stack.pop();
            if let Some(effect) = rt.effects.get_mut(effect_id) {
                effect.running = false;
                if effect.active {
                    effect.callback = callback.take();
                }
            }
        });
        drop(callback);

        if let Err(err) = result {
            crate::runtime::report_ui_error("reactive effect failed", &err);
        }
    }
}

/// Thread-tagged internal signal identifier for `Send` registries that only
/// dereference signals on their owning UI thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawSignalId {
    id: usize,
    thread: ThreadId,
}

/// A `Copy`, fine-grained UI-thread reactive signal holding a value of type `T`.
///
/// `Signal<T>` is `!Send` and `!Sync` because its backing arena is thread-local
/// to the UI thread. To update a signal from a background thread, obtain a
/// [`SignalWriter<T>`] via [`Signal::writer`] or use [`AppScope::channel`].
///
/// Reading a `Signal` inside a [`Memo`], [`Effect`], or reactive view property
/// automatically subscribes to future changes.
pub struct Signal<T: 'static> {
    id: usize,
    _marker: PhantomData<*mut T>,
}

impl<T: 'static> Copy for Signal<T> {}

impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: 'static> Eq for Signal<T> {}

impl<T: 'static> fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signal").field("id", &self.id).finish()
    }
}

impl<T: 'static> Signal<T> {
    /// Creates a new reactive signal with `initial` value.
    pub fn new(initial: T) -> Self {
        let id = RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            let id = rt.signals.len();
            rt.signals.push(SignalSlot {
                value: Some(Box::new(initial)),
                subscribers: Vec::new(),
            });
            id
        });
        Self {
            id,
            _marker: PhantomData,
        }
    }

    pub(crate) fn new_scoped(initial: T) -> Self {
        let signal = Self::new(initial);
        on_scope_cleanup(move || signal.dispose_value());
        signal
    }

    pub(crate) fn dispose_value(self) {
        let value = RUNTIME.with(|runtime| {
            runtime
                .borrow_mut()
                .signals
                .get_mut(self.id)
                .and_then(|slot| slot.value.take())
        });
        drop(value);
    }

    pub(crate) fn is_alive(self) -> bool {
        RUNTIME.with(|runtime| {
            runtime
                .borrow()
                .signals
                .get(self.id)
                .is_some_and(|slot| slot.value.is_some())
        })
    }

    /// Reads and clones the current value, recording a reactive dependency if
    /// called inside a running [`Effect`] or [`Memo`].
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    /// Reads and clones the current value without recording a reactive dependency.
    pub fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        self.with_untracked(T::clone)
    }

    /// Borrows the current value and records a reactive dependency.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        record_dependency(self.id);
        self.with_untracked(f)
    }

    /// Borrows the current value without recording a reactive dependency.
    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        RUNTIME.with(|runtime| {
            let rt = runtime.borrow();
            let slot = rt
                .signals
                .get(self.id)
                .expect("Signal read after reactive runtime was cleared");
            let value = slot
                .value
                .as_ref()
                .expect("Signal read after its reactive scope was disposed")
                .downcast_ref::<T>()
                .expect("Signal type mismatch");
            f(value)
        })
    }

    /// Replaces the signal's value and notifies all subscribed effects and memos.
    pub fn set(&self, value: T) {
        let old = RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            if let Some(slot) = rt.signals.get_mut(self.id) {
                assert!(
                    slot.value.is_some(),
                    "Signal updated after its reactive scope was disposed"
                );
                slot.value.replace(Box::new(value))
            } else {
                None
            }
        });
        drop(old);
        enqueue_subscribers(self.id);
    }

    /// Replaces the signal's value only if `current != value`, notifying subscribers
    /// only when the value actually changed.
    pub fn set_if_changed(&self, value: T)
    where
        T: PartialEq,
    {
        let old = RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            let Some(slot) = rt.signals.get_mut(self.id) else {
                return None;
            };
            let current = slot
                .value
                .as_ref()
                .expect("Signal updated after its reactive scope was disposed")
                .downcast_ref::<T>()
                .expect("Signal type mismatch");
            if *current == value {
                return None;
            }
            slot.value.replace(Box::new(value))
        });
        if old.is_some() {
            drop(old);
            enqueue_subscribers(self.id);
        }
    }

    /// Mutates the signal's value in place and notifies all subscribed effects and memos.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            if let Some(slot) = rt.signals.get_mut(self.id) {
                let value = slot
                    .value
                    .as_mut()
                    .expect("Signal updated after its reactive scope was disposed")
                    .downcast_mut::<T>()
                    .expect("Signal type mismatch");
                f(value);
            }
        });
        enqueue_subscribers(self.id);
    }

    /// Wraps this signal for two-way binding to an interactive control property
    /// (such as `TextBox::text`, `CheckBox::checked`, `ToggleSwitch::checked`,
    /// `Slider::value`, or `ComboBox::selected_index`).
    pub fn two_way(&self) -> TwoWay<T> {
        TwoWay { signal: *self }
    }

    /// Creates a `Send + Sync + Clone` [`SignalWriter<T>`] that marshals updates
    /// from background threads onto the UI dispatcher.
    pub fn writer(&self, scope: &AppScope) -> SignalWriter<T>
    where
        T: Send,
    {
        SignalWriter {
            context: scope.clone_context(),
            raw: self.raw_id(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn raw_id(&self) -> RawSignalId {
        RawSignalId {
            id: self.id,
            thread: std::thread::current().id(),
        }
    }

    pub(crate) fn try_from_raw(raw: RawSignalId) -> Result<Self> {
        if std::thread::current().id() != raw.thread {
            return Err(Error::NoUiContext);
        }
        Ok(Self {
            id: raw.id,
            _marker: PhantomData,
        })
    }
}

/// A thread-safe (`Send + Sync + Clone`) writer handle for updating a [`Signal<T>`]
/// from background threads via [`AppContext::post`].
pub struct SignalWriter<T: Send + 'static> {
    context: AppContext,
    raw: RawSignalId,
    _marker: PhantomData<fn(T)>,
}

impl<T: Send + 'static> Clone for SignalWriter<T> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            raw: self.raw,
            _marker: PhantomData,
        }
    }
}

impl<T: Send + 'static> fmt::Debug for SignalWriter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignalWriter")
            .field("raw", &self.raw)
            .finish()
    }
}

impl<T: Send + 'static> SignalWriter<T> {
    /// Replaces the signal's value on the UI thread.
    pub fn set(&self, value: T) -> Result<()> {
        let raw = self.raw;
        if std::thread::current().id() == raw.thread {
            if let Ok(sig) = Signal::<T>::try_from_raw(raw) {
                if sig.is_alive() {
                    sig.set(value);
                }
            }
            return Ok(());
        }
        self.context.post(move || {
            if let Ok(sig) = Signal::<T>::try_from_raw(raw) {
                if sig.is_alive() {
                    sig.set(value);
                }
            }
        })
    }

    /// Replaces the signal's value on the UI thread only when `current != value`.
    pub fn set_if_changed(&self, value: T) -> Result<()>
    where
        T: PartialEq,
    {
        let raw = self.raw;
        if std::thread::current().id() == raw.thread {
            if let Ok(sig) = Signal::<T>::try_from_raw(raw) {
                if sig.is_alive() {
                    sig.set_if_changed(value);
                }
            }
            return Ok(());
        }
        self.context.post(move || {
            if let Ok(sig) = Signal::<T>::try_from_raw(raw) {
                if sig.is_alive() {
                    sig.set_if_changed(value);
                }
            }
        })
    }

    /// Mutates the signal's value in place on the UI thread.
    pub fn update(&self, f: impl FnOnce(&mut T) + Send + 'static) -> Result<()> {
        let raw = self.raw;
        if std::thread::current().id() == raw.thread {
            if let Ok(sig) = Signal::<T>::try_from_raw(raw) {
                if sig.is_alive() {
                    sig.update(f);
                }
            }
            return Ok(());
        }
        self.context.post(move || {
            if let Ok(sig) = Signal::<T>::try_from_raw(raw) {
                if sig.is_alive() {
                    sig.update(f);
                }
            }
        })
    }
}

/// Two-way binding descriptor created via [`Signal::two_way`].
pub struct TwoWay<T: 'static> {
    pub(crate) signal: Signal<T>,
}

impl<T: 'static> Copy for TwoWay<T> {}

impl<T: 'static> Clone for TwoWay<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> fmt::Debug for TwoWay<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TwoWay")
            .field("signal", &self.signal)
            .finish()
    }
}

impl<T: 'static> TwoWay<T> {
    pub fn signal(&self) -> Signal<T> {
        self.signal
    }
}

type PathReader<T> = dyn Fn() -> Result<T>;
type PathWriter<T> = dyn Fn(T) -> Result<()>;

/// An application-supplied path into a Rust model.
///
/// Signal reads made by the reader are tracked on every evaluation. When an
/// intermediate model changes, subsequent evaluations subscribe to the new
/// model and unsubscribe from the old one.
pub struct PropertyPath<T: 'static> {
    pub(crate) read: Rc<PathReader<T>>,
    pub(crate) write: Option<Rc<PathWriter<T>>>,
}

impl<T: 'static> Clone for PropertyPath<T> {
    fn clone(&self) -> Self {
        Self {
            read: self.read.clone(),
            write: self.write.clone(),
        }
    }
}

impl<T: 'static> fmt::Debug for PropertyPath<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PropertyPath")
            .field("writable", &self.write.is_some())
            .finish_non_exhaustive()
    }
}

impl<T: 'static> PropertyPath<T> {
    /// Creates a read-only path. Return an error if an intermediate value is
    /// unavailable; callers may instead choose an explicit fallback.
    pub fn read_only(read: impl Fn() -> Result<T> + 'static) -> Self {
        Self {
            read: Rc::new(read),
            write: None,
        }
    }

    /// Creates a path whose writer updates the currently selected model.
    pub fn writable(
        read: impl Fn() -> Result<T> + 'static,
        write: impl Fn(T) -> Result<()> + 'static,
    ) -> Self {
        Self {
            read: Rc::new(read),
            write: Some(Rc::new(write)),
        }
    }

    /// Reads the current value and records any signal dependencies.
    pub fn get(&self) -> Result<T> {
        (self.read)()
    }

    /// Writes through the current path, or reports that it is read-only.
    pub fn set(&self, value: T) -> Result<()> {
        self.write
            .as_ref()
            .ok_or(crate::Error::ReadOnlyBindingPath)?(value)
    }

    /// Uses this path as a two-way control binding.
    pub fn two_way(&self) -> PathTwoWay<T> {
        PathTwoWay(self.clone())
    }
}

/// A two-way descriptor created by [`PropertyPath::two_way`].
pub struct PathTwoWay<T: 'static>(pub(crate) PropertyPath<T>);

/// A derived, cached reactive value that automatically recomputes when any
/// [`Signal`] or [`Memo`] read inside its computation changes, and only notifies
/// downstream subscribers when the resulting value changes (`PartialEq`).
pub struct Memo<T: 'static> {
    signal: Signal<T>,
    effect: Effect,
}

impl<T: 'static> Copy for Memo<T> {}

impl<T: 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> fmt::Debug for Memo<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Memo")
            .field("signal", &self.signal)
            .field("effect", &self.effect)
            .finish()
    }
}

impl<T: Clone + PartialEq + 'static> Memo<T> {
    /// Creates a new memoized derived signal.
    pub fn new(mut compute: impl FnMut() -> T + 'static) -> Self {
        let effect_id = RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            let id = rt.effects.len();
            rt.effects.push(EffectSlot {
                callback: None,
                dependencies: Vec::new(),
                scope_id: 0,
                active: true,
                running: true,
                queued: false,
            });
            attach_effect_to_current_scope(&mut rt, id);
            rt.tracking_stack.push(id);
            id
        });

        let initial = compute();

        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            rt.tracking_stack.pop();
            if let Some(effect) = rt.effects.get_mut(effect_id) {
                effect.running = false;
            }
        });

        let signal = Signal::new(initial);
        let memo_signal = signal;
        let callback: EffectCallback = Box::new(move || {
            let next = compute();
            memo_signal.set_if_changed(next);
            Ok(())
        });

        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            if let Some(effect) = rt.effects.get_mut(effect_id) {
                effect.callback = Some(callback);
            }
        });

        Self {
            signal,
            effect: Effect { id: effect_id },
        }
    }

    /// Creates a memo whose cached value is released when the current reactive
    /// scope is disposed. Its signal must not be used after that point.
    pub fn new_scoped(compute: impl FnMut() -> T + 'static) -> Self {
        let memo = Self::new(compute);
        on_scope_cleanup(move || memo.signal.dispose_value());
        memo
    }

    pub fn get(&self) -> T {
        self.signal.get()
    }

    pub fn get_untracked(&self) -> T {
        self.signal.get_untracked()
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.signal.with(f)
    }

    pub fn as_signal(&self) -> Signal<T> {
        self.signal
    }
}

/// A reactive side-effect that runs immediately and re-runs whenever any
/// [`Signal`] or [`Memo`] read during its most recent execution changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effect {
    id: usize,
}

impl Effect {
    /// Creates and immediately runs a tracked reactive effect.
    pub fn new(mut f: impl FnMut() -> Result<()> + 'static) -> Result<Self> {
        let effect_id = RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            let id = rt.effects.len();
            rt.effects.push(EffectSlot {
                callback: None,
                dependencies: Vec::new(),
                scope_id: 0,
                active: true,
                running: true,
                queued: false,
            });
            attach_effect_to_current_scope(&mut rt, id);
            rt.tracking_stack.push(id);
            id
        });

        let initial_result = f();

        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            rt.tracking_stack.pop();
            if let Some(effect) = rt.effects.get_mut(effect_id) {
                effect.running = false;
                if initial_result.is_ok() && effect.active {
                    effect.callback = Some(Box::new(f));
                } else {
                    effect.active = false;
                    unlink_effect_dependencies(&mut rt, effect_id);
                }
            }
        });

        initial_result?;
        Ok(Self { id: effect_id })
    }

    /// Stops this effect from running on future signal updates.
    pub fn dispose(self) {
        let callback = RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            unlink_effect_dependencies(&mut rt, self.id);
            if let Some(effect) = rt.effects.get_mut(self.id) {
                effect.active = false;
                effect.callback.take()
            } else {
                None
            }
        });
        drop(callback);
    }
}

/// A `Copy` slot for capturing a materialized control reference from a declarative
/// [`El`](crate::view::El) or `rsx!` tree.
pub struct NodeRef<T: 'static> {
    slot: Signal<(u64, Option<T>)>,
}

impl<T: 'static> Copy for NodeRef<T> {}

impl<T: 'static> Clone for NodeRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Default for NodeRef<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> fmt::Debug for NodeRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeRef").field("slot", &self.slot).finish()
    }
}

impl<T: 'static> NodeRef<T> {
    pub fn new() -> Self {
        Self {
            slot: Signal::new((0, None)),
        }
    }

    /// Creates a reference whose storage is released with the current reactive scope.
    pub fn new_scoped() -> Self {
        Self {
            slot: Signal::new_scoped((0, None)),
        }
    }

    pub fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        if !self.slot.is_alive() {
            return None;
        }
        self.slot.with_untracked(|(_, value)| value.clone())
    }

    pub fn set(&self, value: T) {
        self.assign(Some(value));
    }

    /// Clears the captured value.
    pub fn clear(&self) {
        self.assign(None);
    }

    fn assign(&self, value: Option<T>) -> u64 {
        let generation = self
            .slot
            .with_untracked(|(generation, _)| generation.wrapping_add(1));
        self.slot.set((generation, value));
        generation
    }

    pub(crate) fn set_scoped(&self, value: T) {
        let generation = self.assign(Some(value));
        let slot = *self;
        on_scope_cleanup(move || {
            if slot
                .slot
                .with_untracked(|(current, _)| *current == generation)
            {
                slot.clear();
            }
        });
    }
}

/// A hierarchical ownership scope that tracks [`Effect`]s, [`Memo`]s, and
/// context values created within a UI subtree, disposing them deterministically
/// when the subtree unmounts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReactiveScope {
    id: usize,
}

impl Default for ReactiveScope {
    fn default() -> Self {
        Self::new()
    }
}

impl ReactiveScope {
    fn is_active(self) -> bool {
        RUNTIME.with(|runtime| {
            runtime
                .borrow()
                .scopes
                .get(self.id)
                .is_some_and(|scope| scope.active)
        })
    }

    pub(crate) fn current() -> Self {
        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            let id = rt
                .scope_stack
                .last()
                .copied()
                .unwrap_or_else(|| ensure_root_scope(&mut rt));
            Self { id }
        })
    }

    /// Creates a new child `ReactiveScope` parented to the currently active scope.
    pub fn new() -> Self {
        let id = RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            let parent = Some(
                rt.scope_stack
                    .last()
                    .copied()
                    .unwrap_or_else(|| ensure_root_scope(&mut rt)),
            );
            let id = rt.scopes.len();
            rt.scopes.push(ScopeSlot {
                parent,
                children: Vec::new(),
                effects: Vec::new(),
                contexts: HashMap::new(),
                cleanups: Vec::new(),
                active: true,
            });
            if let Some(parent_id) = parent {
                if let Some(parent_slot) = rt.scopes.get_mut(parent_id) {
                    parent_slot.children.push(id);
                }
            }
            id
        });
        Self { id }
    }

    /// Runs `f` with this scope active on the scope stack.
    pub fn run<R>(self, f: impl FnOnce() -> R) -> R {
        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            assert!(
                rt.scopes.get(self.id).is_some_and(|scope| scope.active),
                "cannot run a disposed reactive scope"
            );
            rt.scope_stack.push(self.id);
        });
        struct PopGuard;
        impl Drop for PopGuard {
            fn drop(&mut self) {
                RUNTIME.with(|runtime| {
                    runtime.borrow_mut().scope_stack.pop();
                });
            }
        }
        let _guard = PopGuard;
        f()
    }

    /// Creates a new `ReactiveScope`, runs `f` inside it, and returns both the
    /// result and the scope handle.
    pub fn with_new<R>(f: impl FnOnce() -> R) -> (R, Self) {
        let scope = Self::new();
        let out = scope.run(f);
        (out, scope)
    }

    /// Disposes this scope, all of its child scopes, and every [`Effect`] / [`Memo`]
    /// created within them.
    pub fn dispose(self) {
        let disposed = RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            let mut disposed = ScopeDisposals::default();
            dispose_scope_recursive(&mut rt, self.id, &mut disposed);
            disposed
        });
        drop(disposed.callbacks);
        for cleanup in disposed.cleanups {
            cleanup();
        }
        drop(disposed.contexts);
    }
}

#[derive(Default)]
struct ScopeDisposals {
    callbacks: Vec<EffectCallback>,
    contexts: Vec<Box<dyn Any>>,
    cleanups: Vec<ScopeCleanup>,
}

fn dispose_scope_recursive(
    rt: &mut ReactiveRuntime,
    scope_id: usize,
    disposed: &mut ScopeDisposals,
) {
    let (parent, children, effects, contexts, cleanups) =
        if let Some(scope) = rt.scopes.get_mut(scope_id) {
            if !scope.active {
                return;
            }
            scope.active = false;
            (
                scope.parent,
                std::mem::take(&mut scope.children),
                std::mem::take(&mut scope.effects),
                std::mem::take(&mut scope.contexts),
                std::mem::take(&mut scope.cleanups),
            )
        } else {
            return;
        };

    for child_id in children {
        dispose_scope_recursive(rt, child_id, disposed);
    }

    for effect_id in effects {
        unlink_effect_dependencies(rt, effect_id);
        if let Some(effect) = rt.effects.get_mut(effect_id) {
            effect.active = false;
            if let Some(callback) = effect.callback.take() {
                disposed.callbacks.push(callback);
            }
        }
    }
    disposed.contexts.extend(contexts.into_values());
    disposed.cleanups.extend(cleanups.into_iter().rev());
    if let Some(parent) = parent.and_then(|id| rt.scopes.get_mut(id)) {
        parent.children.retain(|child| *child != scope_id);
    }
}

pub(crate) fn on_scope_cleanup(cleanup: impl FnOnce() + 'static) {
    RUNTIME.with(|runtime| {
        let mut rt = runtime.borrow_mut();
        let scope_id = rt
            .scope_stack
            .last()
            .copied()
            .unwrap_or_else(|| ensure_root_scope(&mut rt));
        let scope = rt
            .scopes
            .get_mut(scope_id)
            .expect("reactive scope is no longer available");
        assert!(
            scope.active,
            "cannot attach cleanup to a disposed reactive scope"
        );
        scope.cleanups.push(Box::new(cleanup));
    });
}

/// Provides a typed context value to the current [`ReactiveScope`] and all of
/// its descendants.
pub fn provide_context<T: Clone + 'static>(value: T) {
    RUNTIME.with(|runtime| {
        let mut rt = runtime.borrow_mut();
        let scope_id = rt
            .scope_stack
            .last()
            .copied()
            .unwrap_or_else(|| ensure_root_scope(&mut rt));
        if let Some(scope) = rt.scopes.get_mut(scope_id) {
            scope.contexts.insert(TypeId::of::<T>(), Box::new(value));
        }
    });
}

/// Retrieves a typed context value from the nearest enclosing [`ReactiveScope`]
/// (walking up to the root application scope).
pub fn use_context<T: Clone + 'static>() -> Option<T> {
    RUNTIME.with(|runtime| {
        let rt = runtime.borrow();
        if rt.scopes.is_empty() {
            return None;
        }
        let mut cursor = Some(rt.scope_stack.last().copied().unwrap_or(0));
        while let Some(scope_id) = cursor {
            let scope = rt.scopes.get(scope_id)?;
            if let Some(boxed) = scope.contexts.get(&TypeId::of::<T>()) {
                if let Some(value) = boxed.downcast_ref::<T>() {
                    return Some(value.clone());
                }
            }
            cursor = scope.parent;
        }
        None
    })
}

/// The state of an asynchronous or derived [`Resource<T>`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceState<T> {
    Loading,
    Ready(T),
    Error(String),
}

impl<T> ResourceState<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            _ => None,
        }
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Error(message) => Some(message.as_str()),
            _ => None,
        }
    }
}

/// A `Copy` reactive handle representing an asynchronous or fallible data resource.
pub struct Resource<T: 'static> {
    state: Signal<ResourceState<T>>,
    generation: Signal<u64>,
}

impl<T: 'static> Copy for Resource<T> {}

impl<T: 'static> Clone for Resource<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> fmt::Debug for Resource<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resource")
            .field("state", &self.state)
            .field("generation", &self.generation)
            .finish()
    }
}

impl<T: 'static> Resource<T> {
    /// Creates a new resource initially in [`ResourceState::Loading`].
    pub fn pending() -> Self {
        Self {
            state: Signal::new(ResourceState::Loading),
            generation: Signal::new(0),
        }
    }

    /// Creates a reactive resource computed from `source` and `fetcher`.
    /// Whenever any signal read by `source` changes, `state` updates with the
    /// result of `fetcher`.
    pub fn new_derived<S>(
        mut source: impl FnMut() -> S + 'static,
        mut fetcher: impl FnMut(S) -> std::result::Result<T, String> + 'static,
    ) -> Result<Self>
    where
        T: PartialEq,
    {
        let resource = Self::pending();
        Effect::new(move || {
            let input = source();
            let next = match fetcher(input) {
                Ok(value) => ResourceState::Ready(value),
                Err(error) => ResourceState::Error(error),
            };
            resource.state.set_if_changed(next);
            Ok(())
        })?;
        Ok(resource)
    }

    pub fn get(&self) -> ResourceState<T>
    where
        T: Clone,
    {
        self.state.get()
    }

    pub fn get_untracked(&self) -> ResourceState<T>
    where
        T: Clone,
    {
        self.state.get_untracked()
    }

    pub fn signal(&self) -> Signal<ResourceState<T>> {
        self.state
    }

    pub fn set_loading(&self) {
        self.state.set(ResourceState::Loading);
    }

    pub fn set_ready(&self, value: T) {
        self.state.set(ResourceState::Ready(value));
    }

    pub fn set_error(&self, message: impl Into<String>) {
        self.state.set(ResourceState::Error(message.into()));
    }

    pub(crate) fn state_raw_id(&self) -> RawSignalId {
        self.state.raw_id()
    }

    pub(crate) fn generation_raw_id(&self) -> RawSignalId {
        self.generation.raw_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn scoped_node_refs_release_their_value_without_clearing_a_newer_mount() {
        let node = NodeRef::new();
        let first = ReactiveScope::new();
        let first_value = Rc::new(1);
        let first_weak = Rc::downgrade(&first_value);
        first.run(|| node.set_scoped(first_value));

        let second = ReactiveScope::new();
        let second_value = Rc::new(2);
        let second_weak = Rc::downgrade(&second_value);
        second.run(|| node.set_scoped(second_value));
        assert!(first_weak.upgrade().is_none());
        first.dispose();
        assert_eq!(*node.get().unwrap(), 2);
        second.dispose();
        assert!(node.get().is_none());
        assert!(second_weak.upgrade().is_none());
    }

    #[test]
    fn scoped_memos_release_captured_and_cached_values() {
        let scope = ReactiveScope::new();
        let value = Rc::new(String::from("owned by the view"));
        let weak = Rc::downgrade(&value);
        scope.run(|| {
            let _memo = Memo::new_scoped(move || value.clone());
        });
        assert!(weak.upgrade().is_some());
        scope.dispose();
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn scope_disposal_drops_callbacks_outside_the_runtime_borrow() {
        struct ReadsSignalOnDrop(Signal<i32>);
        impl Drop for ReadsSignalOnDrop {
            fn drop(&mut self) {
                assert_eq!(self.0.get_untracked(), 7);
            }
        }

        let signal = Signal::new(7);
        let scope = ReactiveScope::new();
        scope.run(|| {
            let guard = ReadsSignalOnDrop(signal);
            Effect::new(move || {
                let _ = &guard;
                Ok(())
            })
            .unwrap();
        });
        scope.dispose();
    }
}
