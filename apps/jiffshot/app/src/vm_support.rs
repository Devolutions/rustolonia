//! What [`crate::vm`] is built on: publishing values to signals from any
//! thread, keeping published values from echoing back into the model, model
//! cells, and collections of element models.

use rustolonia::{AppScope, Effect, Result, Signal};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;
use std::thread::ThreadId;

static UI: OnceLock<(ThreadId, AppScope)> = OnceLock::new();

thread_local! {
    /// Set while the model's values are written to signals.
    static PUBLISHING: Cell<u32> = const { Cell::new(0) };
}

/// Remembers the UI thread and how to reach it; call once, from it, before
/// any sink is used.
pub fn install(scope: &AppScope) {
    let _ = UI.set((std::thread::current().id(), scope.clone()));
}

pub fn on_ui_thread() -> bool {
    UI.get().is_some_and(|(id, _)| *id == std::thread::current().id())
}

/// Runs `f` on the UI thread: now when already there, otherwise posted.
pub fn on_ui(f: impl FnOnce() + Send + 'static) {
    if on_ui_thread() {
        f();
    } else if let Some((_, scope)) = UI.get() {
        let _ = scope.post(f);
    }
}

/// Whether the model is publishing right now: a control changed by that is
/// not a user's edit.
pub fn publishing() -> bool {
    PUBLISHING.with(|p| p.get() > 0)
}

/// Runs `f` as part of publishing (see [`publishing`]).
pub fn as_publisher<R>(f: impl FnOnce() -> R) -> R {
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            PUBLISHING.with(|p| p.set(p.get() - 1));
        }
    }
    PUBLISHING.with(|p| p.set(p.get() + 1));
    let _guard = Guard;
    f()
}

/// Sets `signal` to `value` when it differs, from any thread.
pub fn publish<T: PartialEq + Send + 'static>(signal: Signal<T>, value: T) -> Result<()> {
    on_ui(move || as_publisher(|| signal.set_if_changed(value)));
    Ok(())
}

/// Changes `signal`'s value in place, from any thread.
pub fn publish_update<T: Send + 'static>(signal: Signal<T>, f: impl FnOnce(&mut T) + Send + 'static) -> Result<()> {
    on_ui(move || as_publisher(|| signal.update(f)));
    Ok(())
}

/// A model shared by its window's controls. Calls that arrive while it is
/// already in a call (a control echoing what the model just published) are
/// dropped.
pub struct ModelCell<M: ?Sized>(Rc<RefCell<Box<M>>>);

impl<M: ?Sized> Clone for ModelCell<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<M: ?Sized> ModelCell<M> {
    pub fn new(model: Box<M>) -> Self {
        Self(Rc::new(RefCell::new(model)))
    }

    /// Runs `f` on the model unless it is busy.
    pub fn call(&self, f: impl FnOnce(&mut M) -> Result<()>) -> Result<()> {
        match self.0.try_borrow_mut() {
            Ok(mut model) => f(&mut model),
            Err(_) => Ok(()),
        }
    }

    /// Like [`ModelCell::call`], for control callbacks that cannot return an
    /// error: a failing call is logged.
    pub fn run(&self, f: impl FnOnce(&mut M) -> Result<()>) {
        if let Err(error) = self.call(f) {
            eprintln!("view-model call failed: {error:?}");
        }
    }
}

/// Calls the model's setter when a control changes `signal` (a two-way
/// binding), but not for values the model published.
pub fn watch<T, M: ?Sized + 'static>(
    signal: Signal<T>,
    model: &ModelCell<M>,
    set: impl Fn(&mut M, T) -> Result<()> + 'static,
) -> Result<()>
where
    T: Clone + 'static,
{
    let model = model.clone();
    let first = Cell::new(true);
    Effect::new(move || {
        let value = signal.get();
        if first.replace(false) || publishing() {
            return Ok(());
        }
        model.run(|m| set(m, value));
        Ok(())
    })?;
    Ok(())
}

// ---- element models --------------------------------------------------------

/// One element of a model collection (a library item, a quick style): the
/// values it published and its model. Views bind to [`Element::fields`] and
/// run commands through [`Element::model`].
pub struct Element<F: 'static, M: ?Sized> {
    pub fields: Signal<F>,
    pub model: ModelCell<M>,
    /// Stable for the element's life: a key for list views.
    pub key: u64,
    detach: Box<dyn Fn(&mut M) -> Result<()>>,
}

impl<F: 'static, M: ?Sized> Drop for Element<F, M> {
    fn drop(&mut self) {
        let detach = &self.detach;
        let _ = self.model.call(|m| detach(m));
        recycle(self.fields, self.key);
    }
}

/// An element's sink: usable from any thread, and harmless once the element
/// is gone (its signal may already serve another element).
pub struct ElementSink<F: 'static> {
    fields: Signal<F>,
    key: u64,
}

impl<F: 'static> Clone for ElementSink<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F: 'static> Copy for ElementSink<F> {}

impl<F: 'static> std::fmt::Debug for ElementSink<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElementSink").field("key", &self.key).finish()
    }
}

impl<F: Clone + PartialEq + 'static> ElementSink<F> {
    /// Changes the element's values, when the element still exists.
    pub fn update(&self, f: impl FnOnce(&mut F) + Send + 'static) -> Result<()> {
        let (fields, key) = (self.fields, self.key);
        on_ui(move || {
            if owner_of(fields) == Some(key) {
                let mut value = fields.get_untracked();
                f(&mut value);
                as_publisher(|| fields.set_if_changed(value));
            }
        });
        Ok(())
    }
}

thread_local! {
    /// For each fields type: every element signal and the element it serves
    /// now (0 when free). Signals are never freed, so a library refreshed all
    /// day reuses them instead of growing.
    static SLOTS: RefCell<HashMap<std::any::TypeId, Box<dyn std::any::Any>>> = RefCell::new(HashMap::new());
    static NEXT_KEY: Cell<u64> = const { Cell::new(1) };
}

fn with_slots<F: 'static, R>(f: impl FnOnce(&mut Vec<(Signal<F>, u64)>) -> R) -> R {
    SLOTS.with(|slots| {
        let mut slots = slots.borrow_mut();
        let entry = slots
            .entry(std::any::TypeId::of::<F>())
            .or_insert_with(|| Box::new(Vec::<(Signal<F>, u64)>::new()));
        f(entry.downcast_mut().expect("slot type"))
    })
}

fn owner_of<F: 'static>(fields: Signal<F>) -> Option<u64> {
    with_slots(|slots: &mut Vec<(Signal<F>, u64)>| slots.iter().find(|(s, _)| *s == fields).map(|(_, k)| *k))
}

fn recycle<F: 'static>(fields: Signal<F>, key: u64) {
    with_slots(|slots: &mut Vec<(Signal<F>, u64)>| {
        if let Some(slot) = slots.iter_mut().find(|(s, k)| *s == fields && *k == key) {
            slot.1 = 0;
        }
    });
}

fn fresh<F: Default + 'static>() -> (Signal<F>, u64) {
    let key = NEXT_KEY.with(|k| k.replace(k.get() + 1));
    let reused = with_slots(|slots: &mut Vec<(Signal<F>, u64)>| {
        slots.iter_mut().find(|(_, k)| *k == 0).map(|slot| {
            slot.1 = key;
            slot.0
        })
    });
    let fields = match reused {
        Some(signal) => {
            signal.set(F::default());
            signal
        }
        None => {
            let signal = Signal::new(F::default());
            with_slots(|slots| slots.push((signal, key)));
            signal
        }
    };
    (fields, key)
}

type Attach<M, F> = fn(&mut M, ElementSink<F>) -> Result<()>;

impl<F: Clone + Default + PartialEq + 'static, M: ?Sized + 'static> Element<F, M> {
    fn make<V: Send + 'static>(
        value: V,
        boxed: fn(V) -> Box<M>,
        attach: Attach<M, F>,
        detach: fn(&mut M) -> Result<()>,
    ) -> Rc<Self> {
        let (fields, key) = fresh::<F>();
        let model = ModelCell::new(boxed(value));
        if let Err(error) = model.call(|m| attach(m, ElementSink { fields, key })) {
            eprintln!("attaching an element failed: {error:?}");
        }
        Rc::new(Self {
            fields,
            model,
            key,
            detach: Box::new(detach),
        })
    }

    pub fn add<V: Send + 'static>(
        list: Signal<Vec<Rc<Self>>>,
        value: V,
        boxed: fn(V) -> Box<M>,
        attach: Attach<M, F>,
        detach: fn(&mut M) -> Result<()>,
    ) -> Result<()> {
        on_ui(move || {
            let element = Self::make(value, boxed, attach, detach);
            as_publisher(|| list.update(|l| l.push(element)));
        });
        Ok(())
    }

    pub fn clear(list: Signal<Vec<Rc<Self>>>) -> Result<()> {
        on_ui(move || {
            let mut old = Vec::new();
            as_publisher(|| list.update(|l| old = std::mem::take(l)));
            drop(old);
        });
        Ok(())
    }
}
