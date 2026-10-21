//! Single-threaded fine-grained reactive primitives (`Signal`, `Memo`, `Effect`, `NodeRef`)
//! for declarative and imperative UI authoring.
//!
//! Handles are `Copy` indices into the UI-thread reactive arena, so they can be
//! captured directly by multiple event closures (`on_click`, `on_value_changed`)
//! without cloning control handles or wrapping state in `Arc<Mutex<_>>`.

use crate::Result;
use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::marker::PhantomData;

struct SignalSlot {
    value: Box<dyn Any>,
    subscribers: Vec<usize>,
}

type EffectCallback = Box<dyn FnMut() -> Result<()>>;

struct EffectSlot {
    callback: Option<EffectCallback>,
    dependencies: Vec<usize>,
    active: bool,
    running: bool,
    queued: bool,
}

#[derive(Default)]
struct ReactiveRuntime {
    signals: Vec<SignalSlot>,
    effects: Vec<EffectSlot>,
    tracking_stack: Vec<usize>,
    pending_effects: VecDeque<usize>,
    flushing: bool,
}

thread_local! {
    static RUNTIME: RefCell<ReactiveRuntime> = RefCell::new(ReactiveRuntime::default());
}

pub(crate) fn clear_reactive_runtime() {
    RUNTIME.with(|runtime| {
        let mut rt = runtime.borrow_mut();
        for effect in &mut rt.effects {
            effect.active = false;
            effect.callback = None;
            effect.dependencies.clear();
        }
        rt.signals.clear();
        rt.effects.clear();
        rt.tracking_stack.clear();
        rt.pending_effects.clear();
        rt.flushing = false;
    });
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
        let _ = flush_pending_effects();
    }
}

fn flush_pending_effects() -> Result<()> {
    let mut first_error: Option<crate::Error> = None;
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
        });

        let result = cb();

        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            rt.tracking_stack.pop();
            if let Some(effect) = rt.effects.get_mut(effect_id) {
                effect.running = false;
                if effect.active {
                    effect.callback = Some(cb);
                }
            }
        });

        if let Err(err) = result {
            if first_error.is_none() {
                first_error = Some(err);
            }
        }
    }

    match first_error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

/// A `Copy`, fine-grained reactive signal holding a value of type `T`.
///
/// Reading a `Signal` inside a [`Memo`], [`Effect`], or reactive view property
/// automatically subscribes to future changes.
pub struct Signal<T: 'static> {
    id: usize,
    _marker: PhantomData<fn() -> T>,
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
                value: Box::new(initial),
                subscribers: Vec::new(),
            });
            id
        });
        Self {
            id,
            _marker: PhantomData,
        }
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
                .downcast_ref::<T>()
                .expect("Signal type mismatch");
            f(value)
        })
    }

    /// Replaces the signal's value and notifies all subscribed effects and memos.
    pub fn set(&self, value: T) {
        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            if let Some(slot) = rt.signals.get_mut(self.id) {
                slot.value = Box::new(value);
            }
        });
        enqueue_subscribers(self.id);
    }

    /// Replaces the signal's value only if `current != value`, notifying subscribers
    /// only when the value actually changed.
    pub fn set_if_changed(&self, value: T)
    where
        T: PartialEq,
    {
        let changed = RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            let Some(slot) = rt.signals.get_mut(self.id) else {
                return false;
            };
            let current = slot
                .value
                .downcast_ref::<T>()
                .expect("Signal type mismatch");
            if *current == value {
                return false;
            }
            slot.value = Box::new(value);
            true
        });
        if changed {
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
                active: true,
                running: true,
                queued: false,
            });
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
                active: true,
                running: true,
                queued: false,
            });
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
        RUNTIME.with(|runtime| {
            let mut rt = runtime.borrow_mut();
            unlink_effect_dependencies(&mut rt, self.id);
            if let Some(effect) = rt.effects.get_mut(self.id) {
                effect.active = false;
                effect.callback = None;
            }
        });
    }
}

/// A `Copy` slot for capturing a materialized control reference from a declarative
/// [`El`](crate::view::El) or `rsx!` tree.
pub struct NodeRef<T: 'static> {
    slot: Signal<Option<T>>,
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
            slot: Signal::new(None),
        }
    }

    pub fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.slot.get_untracked()
    }

    pub fn set(&self, value: T) {
        self.slot.set(Some(value));
    }
}
