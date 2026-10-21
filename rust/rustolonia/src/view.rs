//! Declarative element builders (`El<T>`, `AnyEl`), polymorphic property conversions
//! (`IntoValue`, `IntoProp`), and pooled reactive `DataTemplate` support.

use crate::reactive::{
    provide_context, Effect, Memo, NodeRef, ReactiveScope, Resource, Signal, TwoWay,
};
use crate::{
    AppScope, AsControl, Border, Brush, Color, ContentControl, Control, CornerRadius, DataTemplate,
    Decorator, EventSubscription, ItemsControl, LinearGradientBrush, Paint, Panel, PixelPoint,
    Point, RadialGradientBrush, Rect, ResourceValue, Result, Size, StackPanel, StyledElement,
    TextBlock, ThemeVariant, Thickness, Upcast, Variant, Vector, Window,
};
use rustolonia_sys as sys;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Ergonomic value conversions into projected Avalonia property types.
pub trait IntoValue<T> {
    fn into_value(self) -> T;
}

impl<T> IntoValue<T> for T {
    fn into_value(self) -> T {
        self
    }
}

impl IntoValue<String> for &str {
    fn into_value(self) -> String {
        self.to_owned()
    }
}

impl IntoValue<String> for &String {
    fn into_value(self) -> String {
        self.clone()
    }
}

impl IntoValue<Option<String>> for &str {
    fn into_value(self) -> Option<String> {
        Some(self.to_owned())
    }
}

impl IntoValue<Option<String>> for String {
    fn into_value(self) -> Option<String> {
        Some(self)
    }
}

impl IntoValue<Option<String>> for &String {
    fn into_value(self) -> Option<String> {
        Some(self.clone())
    }
}

impl IntoValue<Option<bool>> for bool {
    fn into_value(self) -> Option<bool> {
        Some(self)
    }
}

impl IntoValue<Thickness> for f64 {
    fn into_value(self) -> Thickness {
        Thickness::uniform(self)
    }
}

impl IntoValue<Thickness> for (f64, f64) {
    fn into_value(self) -> Thickness {
        Thickness::symmetric(self.0, self.1)
    }
}

impl IntoValue<Thickness> for (f64, f64, f64, f64) {
    fn into_value(self) -> Thickness {
        Thickness::new(self.0, self.1, self.2, self.3)
    }
}

impl IntoValue<Thickness> for [f64; 4] {
    fn into_value(self) -> Thickness {
        Thickness::new(self[0], self[1], self[2], self[3])
    }
}

impl IntoValue<Option<Thickness>> for Thickness {
    fn into_value(self) -> Option<Thickness> {
        Some(self)
    }
}

impl IntoValue<Option<Thickness>> for f64 {
    fn into_value(self) -> Option<Thickness> {
        Some(Thickness::uniform(self))
    }
}

impl IntoValue<Option<Thickness>> for (f64, f64) {
    fn into_value(self) -> Option<Thickness> {
        Some(Thickness::symmetric(self.0, self.1))
    }
}

impl IntoValue<Option<Thickness>> for (f64, f64, f64, f64) {
    fn into_value(self) -> Option<Thickness> {
        Some(Thickness::new(self.0, self.1, self.2, self.3))
    }
}

impl IntoValue<CornerRadius> for f64 {
    fn into_value(self) -> CornerRadius {
        CornerRadius::uniform(self)
    }
}

impl IntoValue<CornerRadius> for (f64, f64, f64, f64) {
    fn into_value(self) -> CornerRadius {
        CornerRadius::new(self.0, self.1, self.2, self.3)
    }
}

impl IntoValue<CornerRadius> for [f64; 4] {
    fn into_value(self) -> CornerRadius {
        CornerRadius::new(self[0], self[1], self[2], self[3])
    }
}

impl IntoValue<Option<CornerRadius>> for CornerRadius {
    fn into_value(self) -> Option<CornerRadius> {
        Some(self)
    }
}

impl IntoValue<Option<CornerRadius>> for f64 {
    fn into_value(self) -> Option<CornerRadius> {
        Some(CornerRadius::uniform(self))
    }
}

impl IntoValue<Size> for (f64, f64) {
    fn into_value(self) -> Size {
        Size::new(self.0, self.1)
    }
}

impl IntoValue<Size> for [f64; 2] {
    fn into_value(self) -> Size {
        Size::new(self[0], self[1])
    }
}

impl IntoValue<Point> for (f64, f64) {
    fn into_value(self) -> Point {
        Point::new(self.0, self.1)
    }
}

impl IntoValue<Point> for [f64; 2] {
    fn into_value(self) -> Point {
        Point::new(self[0], self[1])
    }
}

impl IntoValue<Vector> for (f64, f64) {
    fn into_value(self) -> Vector {
        Vector::new(self.0, self.1)
    }
}

impl IntoValue<Vector> for [f64; 2] {
    fn into_value(self) -> Vector {
        Vector::new(self[0], self[1])
    }
}

impl IntoValue<Rect> for (f64, f64, f64, f64) {
    fn into_value(self) -> Rect {
        Rect::new(self.0, self.1, self.2, self.3)
    }
}

impl IntoValue<Rect> for [f64; 4] {
    fn into_value(self) -> Rect {
        Rect::new(self[0], self[1], self[2], self[3])
    }
}

impl IntoValue<Rect> for (Point, Size) {
    fn into_value(self) -> Rect {
        Rect::new(self.0.x, self.0.y, self.1.width, self.1.height)
    }
}

impl IntoValue<PixelPoint> for (i32, i32) {
    fn into_value(self) -> PixelPoint {
        PixelPoint::new(self.0, self.1)
    }
}

impl IntoValue<PixelPoint> for [i32; 2] {
    fn into_value(self) -> PixelPoint {
        PixelPoint::new(self[0], self[1])
    }
}

impl IntoValue<Color> for (u8, u8, u8) {
    fn into_value(self) -> Color {
        Color::rgb(self.0, self.1, self.2)
    }
}

impl IntoValue<Color> for (u8, u8, u8, u8) {
    fn into_value(self) -> Color {
        Color::new(self.0, self.1, self.2, self.3)
    }
}

impl IntoValue<Color> for u32 {
    fn into_value(self) -> Color {
        Color::from(self)
    }
}

impl IntoValue<Brush> for Color {
    fn into_value(self) -> Brush {
        Brush::solid(self)
    }
}

impl IntoValue<Brush> for (u8, u8, u8) {
    fn into_value(self) -> Brush {
        Brush::solid(Color::rgb(self.0, self.1, self.2))
    }
}

impl IntoValue<Brush> for (u8, u8, u8, u8) {
    fn into_value(self) -> Brush {
        Brush::solid(Color::new(self.0, self.1, self.2, self.3))
    }
}

impl IntoValue<Option<Brush>> for Brush {
    fn into_value(self) -> Option<Brush> {
        Some(self)
    }
}

impl IntoValue<Option<Brush>> for Color {
    fn into_value(self) -> Option<Brush> {
        Some(Brush::solid(self))
    }
}

impl IntoValue<Option<Brush>> for (u8, u8, u8) {
    fn into_value(self) -> Option<Brush> {
        Some(Brush::solid(Color::rgb(self.0, self.1, self.2)))
    }
}

impl IntoValue<Option<Brush>> for (u8, u8, u8, u8) {
    fn into_value(self) -> Option<Brush> {
        Some(Brush::solid(Color::new(self.0, self.1, self.2, self.3)))
    }
}

impl IntoValue<Paint> for Brush {
    fn into_value(self) -> Paint {
        Paint::Solid(self)
    }
}

impl IntoValue<Paint> for Color {
    fn into_value(self) -> Paint {
        Paint::Solid(Brush::solid(self))
    }
}

impl IntoValue<Paint> for LinearGradientBrush {
    fn into_value(self) -> Paint {
        Paint::LinearGradient(self)
    }
}

impl IntoValue<Paint> for RadialGradientBrush {
    fn into_value(self) -> Paint {
        Paint::RadialGradient(self)
    }
}

impl IntoValue<Paint> for (u8, u8, u8) {
    fn into_value(self) -> Paint {
        Paint::Solid(Brush::solid(Color::rgb(self.0, self.1, self.2)))
    }
}

impl IntoValue<Paint> for (u8, u8, u8, u8) {
    fn into_value(self) -> Paint {
        Paint::Solid(Brush::solid(Color::new(self.0, self.1, self.2, self.3)))
    }
}

impl IntoValue<Variant> for &str {
    fn into_value(self) -> Variant {
        Variant::from(self)
    }
}

impl IntoValue<Variant> for String {
    fn into_value(self) -> Variant {
        Variant::from(self)
    }
}

impl IntoValue<Variant> for bool {
    fn into_value(self) -> Variant {
        Variant::from(self)
    }
}

impl IntoValue<Variant> for i32 {
    fn into_value(self) -> Variant {
        Variant::from(self)
    }
}

impl IntoValue<Variant> for f64 {
    fn into_value(self) -> Variant {
        Variant::from(self)
    }
}

/// Phantom markers used to disambiguate [`IntoProp`] implementations.
pub mod markers {
    pub struct ValueMarker;
    pub struct SignalMarker;
    pub struct SignalOptionMarker;
    pub struct MemoMarker;
    pub struct MemoOptionMarker;
    pub struct ClosureMarker;
    pub struct TwoWayMarker;
    pub struct TwoWayOptionMarker;
}

type SetterFn<T> = Box<dyn Fn(T) -> Result<()>>;

/// Type-erased adapter for two-way signal bindings.
pub struct TwoWayAdapter<T> {
    pub(crate) bind_to_control: Box<dyn FnOnce(SetterFn<T>, Arc<AtomicBool>) -> Result<()>>,
    pub(crate) set_from_control: Box<dyn Fn(T) + Send + 'static>,
}

/// A static, reactive, or two-way property binding.
pub enum PropBinding<T> {
    Static(T),
    Reactive(Box<dyn FnMut() -> T>),
    TwoWay(TwoWayAdapter<T>),
}

/// Polymorphic conversion into a [`PropBinding<T>`], accepting static values,
/// [`Signal<T>`], [`Memo<T>`], closures `FnMut() -> V`, and [`TwoWay<T>`].
pub trait IntoProp<T, Marker> {
    fn into_prop(self) -> PropBinding<T>;
}

impl<T, V: IntoValue<T>> IntoProp<T, markers::ValueMarker> for V {
    fn into_prop(self) -> PropBinding<T> {
        PropBinding::Static(self.into_value())
    }
}

impl<T: Clone + 'static> IntoProp<T, markers::SignalMarker> for Signal<T> {
    fn into_prop(self) -> PropBinding<T> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || signal.get()))
    }
}

impl IntoProp<Option<String>, markers::SignalOptionMarker> for Signal<String> {
    fn into_prop(self) -> PropBinding<Option<String>> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || Some(signal.get())))
    }
}

impl IntoProp<Option<bool>, markers::SignalOptionMarker> for Signal<bool> {
    fn into_prop(self) -> PropBinding<Option<bool>> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || Some(signal.get())))
    }
}

impl<T: Clone + PartialEq + 'static> IntoProp<T, markers::MemoMarker> for Memo<T> {
    fn into_prop(self) -> PropBinding<T> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || memo.get()))
    }
}

impl IntoProp<Option<String>, markers::MemoOptionMarker> for Memo<String> {
    fn into_prop(self) -> PropBinding<Option<String>> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || Some(memo.get())))
    }
}

impl IntoProp<Option<bool>, markers::MemoOptionMarker> for Memo<bool> {
    fn into_prop(self) -> PropBinding<Option<bool>> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || Some(memo.get())))
    }
}

impl<T, V, F> IntoProp<T, markers::ClosureMarker> for F
where
    F: FnMut() -> V + 'static,
    V: IntoValue<T>,
{
    fn into_prop(mut self) -> PropBinding<T> {
        PropBinding::Reactive(Box::new(move || (self)().into_value()))
    }
}

impl<T: Clone + PartialEq + 'static> IntoProp<T, markers::TwoWayMarker> for TwoWay<T> {
    fn into_prop(self) -> PropBinding<T> {
        let signal = self.signal;
        PropBinding::TwoWay(TwoWayAdapter {
            bind_to_control: Box::new(move |setter, updating| {
                Effect::new(move || {
                    let value = signal.get();
                    if !updating.load(Ordering::Acquire) {
                        setter(value)?;
                    }
                    Ok(())
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value| {
                signal.set_if_changed(value);
            }),
        })
    }
}

impl IntoProp<Option<bool>, markers::TwoWayOptionMarker> for TwoWay<bool> {
    fn into_prop(self) -> PropBinding<Option<bool>> {
        let signal = self.signal;
        PropBinding::TwoWay(TwoWayAdapter {
            bind_to_control: Box::new(move |setter, updating| {
                Effect::new(move || {
                    let value = signal.get();
                    if !updating.load(Ordering::Acquire) {
                        setter(Some(value))?;
                    }
                    Ok(())
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value: Option<bool>| {
                signal.set_if_changed(value.unwrap_or(false));
            }),
        })
    }
}

impl IntoProp<String, markers::TwoWayOptionMarker> for TwoWay<Option<String>> {
    fn into_prop(self) -> PropBinding<String> {
        let signal = self.signal;
        PropBinding::TwoWay(TwoWayAdapter {
            bind_to_control: Box::new(move |setter, updating| {
                Effect::new(move || {
                    let value = signal.get().unwrap_or_default();
                    if !updating.load(Ordering::Acquire) {
                        setter(value)?;
                    }
                    Ok(())
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value: String| {
                signal.set_if_changed(Some(value));
            }),
        })
    }
}

pub(crate) fn apply_prop<C, T>(
    control: &C,
    binding: PropBinding<T>,
    set: impl Fn(&C, T) -> Result<()> + 'static,
) -> Result<()>
where
    C: Clone + 'static,
    T: 'static,
{
    match binding {
        PropBinding::Static(value) => set(control, value),
        PropBinding::Reactive(mut compute) => {
            let control = control.clone();
            Effect::new(move || set(&control, compute())).map(|_| ())
        }
        PropBinding::TwoWay(adapter) => {
            let control = control.clone();
            let updating = Arc::new(AtomicBool::new(false));
            (adapter.bind_to_control)(Box::new(move |value| set(&control, value)), updating)
        }
    }
}

pub(crate) fn apply_two_way_prop<C, T>(
    scope: &AppScope,
    control: &C,
    binding: PropBinding<T>,
    set: impl Fn(&C, T) -> Result<()> + 'static,
    get: impl Fn(&C) -> Result<T> + Send + 'static,
    subscribe: impl FnOnce(&C, Box<dyn FnMut() + Send + 'static>) -> Result<EventSubscription>,
) -> Result<()>
where
    C: Clone + Send + 'static,
    T: 'static,
{
    match binding {
        PropBinding::Static(value) => set(control, value),
        PropBinding::Reactive(mut compute) => {
            let control = control.clone();
            Effect::new(move || set(&control, compute())).map(|_| ())
        }
        PropBinding::TwoWay(adapter) => {
            let updating = Arc::new(AtomicBool::new(false));
            let control_for_set = control.clone();
            (adapter.bind_to_control)(
                Box::new(move |value| set(&control_for_set, value)),
                updating.clone(),
            )?;

            let control_for_get = control.clone();
            let set_from_control = adapter.set_from_control;
            let subscription = subscribe(
                control,
                Box::new(move || {
                    if let Ok(value) = get(&control_for_get) {
                        updating.store(true, Ordering::Release);
                        set_from_control(value);
                        updating.store(false, Ordering::Release);
                    }
                }),
            )?;
            scope.retain_subscription(subscription);
            Ok(())
        }
    }
}

type ElOp<T> = Box<dyn FnOnce(&AppScope, &T) -> Result<()>>;

/// An infallible, deferred declarative builder for a projected Avalonia type `T`.
///
/// Construct via `<ControlType>::el()` or `rsx!`, chain properties, attached
/// properties, children, and event handlers without intermediate `?`, and
/// materialize via [`El::build`] or [`AppScope::mount_view`].
pub struct El<T> {
    create: Box<dyn FnOnce() -> Result<T>>,
    ops: Vec<ElOp<T>>,
}

impl<T: 'static> El<T> {
    pub fn new(create: impl FnOnce() -> Result<T> + 'static) -> Self {
        Self {
            create: Box::new(create),
            ops: Vec::new(),
        }
    }

    pub fn with_op(mut self, op: impl FnOnce(&AppScope, &T) -> Result<()> + 'static) -> Self {
        self.ops.push(Box::new(op));
        self
    }

    /// Captures a clone of the materialized control into `slot` when built.
    pub fn node_ref(self, slot: NodeRef<T>) -> Self
    where
        T: Clone,
    {
        self.with_op(move |_, control| {
            slot.set(control.clone());
            Ok(())
        })
    }

    /// Adds one or more space-separated style classes to a [`StyledElement`].
    pub fn class(self, classes: impl AsRef<str>) -> Self
    where
        T: Upcast<StyledElement>,
    {
        let names: Vec<String> = classes
            .as_ref()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        self.with_op(move |_, control| {
            let styled: &StyledElement = control.upcast_ref();
            let class_list = styled.classes()?;
            for name in names {
                class_list.add(name)?;
            }
            Ok(())
        })
    }

    /// Provides a typed context value to this element and all of its descendants
    /// during [`El::build`].
    pub fn provide_context<C: Clone + 'static>(mut self, value: C) -> Self {
        let mut prev_ops = std::mem::take(&mut self.ops);
        self.ops.push(Box::new(move |scope, control| {
            let child_scope = ReactiveScope::new();
            child_scope.run(|| {
                provide_context(value);
                for op in prev_ops.drain(..) {
                    op(scope, control)?;
                }
                Ok(())
            })
        }));
        self
    }

    /// Materializes the control and applies all deferred properties, children,
    /// and event subscriptions within `scope`.
    pub fn build(self, scope: &AppScope) -> Result<T> {
        let instance = (self.create)()?;
        for op in self.ops {
            op(scope, &instance)?;
        }
        Ok(instance)
    }

    /// Materializes this element and sets it as the content of `host`.
    pub fn mount_into_content(
        self,
        scope: &AppScope,
        host: &impl Upcast<ContentControl>,
    ) -> Result<T>
    where
        T: AsControl,
    {
        let built = self.build(scope)?;
        host.upcast_ref().set_content(Some(&built))?;
        Ok(built)
    }

    /// Materializes this element and sets it as the single child of a [`Decorator`]
    /// (such as [`Border`] or `Viewbox`).
    pub fn mount_into_decorator(self, scope: &AppScope, host: &impl Upcast<Decorator>) -> Result<T>
    where
        T: AsControl,
    {
        let built = self.build(scope)?;
        host.upcast_ref().set_child(Some(&built))?;
        Ok(built)
    }

    /// Materializes this element and appends it to `host.children()`.
    pub fn mount_into_panel(self, scope: &AppScope, host: &impl Upcast<Panel>) -> Result<T>
    where
        T: AsControl,
    {
        let built = self.build(scope)?;
        host.upcast_ref().children()?.add(&built)?;
        Ok(built)
    }

    /// Reactively binds a keyed list of items to a [`Panel`]'s `children()` collection.
    ///
    /// When the bound signal/closure updates, existing keys keep their realized
    /// [`Control`] instances and update their per-item [`Signal<Item>`] in place.
    pub fn children_for<Item, K, M, KF, VF, E>(
        self,
        items: impl IntoProp<Vec<Item>, M> + 'static,
        mut key_fn: KF,
        mut view_fn: VF,
    ) -> Self
    where
        T: Upcast<Panel> + Clone + 'static,
        Item: Clone + PartialEq + 'static,
        K: Eq + Clone + 'static,
        KF: FnMut(&Item) -> K + 'static,
        VF: FnMut(Signal<Item>) -> E + 'static,
        E: IntoChildEl + 'static,
    {
        let binding = items.into_prop();
        self.with_op(move |scope, panel_owner| {
            let panel: Panel = panel_owner.upcast();
            let scope = scope.clone();
            let reconciler = Rc::new(RefCell::new(KeyedListReconciler::<Item, K, Control>::new()));

            let apply =
                move |next_items: Vec<Item>, key_fn: &mut KF, view_fn: &mut VF| -> Result<()> {
                    let scope_for_build = scope.clone();
                    let panel_for_apply = panel.clone();
                    reconciler.borrow_mut().reconcile(
                        next_items,
                        key_fn,
                        &mut |sig| view_fn(sig).into_child_el().build(&scope_for_build),
                        |controls| {
                            let list = panel_for_apply.children()?;
                            list.clear()?;
                            for ctrl in controls {
                                list.add(ctrl)?;
                            }
                            Ok(())
                        },
                    )?;
                    Ok(())
                };

            match binding {
                PropBinding::Static(initial) => {
                    let mut key_fn = key_fn;
                    let mut view_fn = view_fn;
                    apply(initial, &mut key_fn, &mut view_fn)
                }
                PropBinding::Reactive(mut compute) => {
                    Effect::new(move || apply(compute(), &mut key_fn, &mut view_fn)).map(|_| ())
                }
                PropBinding::TwoWay(adapter) => {
                    let updating = Arc::new(AtomicBool::new(false));
                    let mut key_fn = key_fn;
                    let mut view_fn = view_fn;
                    let apply_cell = Rc::new(RefCell::new(move |next: Vec<Item>| {
                        apply(next, &mut key_fn, &mut view_fn)
                    }));
                    (adapter.bind_to_control)(
                        Box::new(move |next| (apply_cell.borrow_mut())(next)),
                        updating,
                    )
                }
            }
        })
    }

    /// Reactively binds a keyed list of items to an [`ItemsControl`]'s `items()` collection.
    pub fn items_for<Item, K, M, KF, VF, E>(
        self,
        items: impl IntoProp<Vec<Item>, M> + 'static,
        mut key_fn: KF,
        mut view_fn: VF,
    ) -> Self
    where
        T: Upcast<ItemsControl> + Clone + 'static,
        Item: Clone + PartialEq + 'static,
        K: Eq + Clone + 'static,
        KF: FnMut(&Item) -> K + 'static,
        VF: FnMut(Signal<Item>) -> E + 'static,
        E: IntoChildEl + 'static,
    {
        let binding = items.into_prop();
        self.with_op(move |scope, items_owner| {
            let items_control: ItemsControl = items_owner.upcast();
            let scope = scope.clone();
            let reconciler = Rc::new(RefCell::new(KeyedListReconciler::<Item, K, Control>::new()));

            let apply =
                move |next_items: Vec<Item>, key_fn: &mut KF, view_fn: &mut VF| -> Result<()> {
                    let scope_for_build = scope.clone();
                    let ic_for_apply = items_control.clone();
                    reconciler.borrow_mut().reconcile(
                        next_items,
                        key_fn,
                        &mut |sig| view_fn(sig).into_child_el().build(&scope_for_build),
                        |controls| {
                            let list = ic_for_apply.items()?;
                            list.clear()?;
                            for ctrl in controls {
                                list.add(ctrl)?;
                            }
                            Ok(())
                        },
                    )?;
                    Ok(())
                };

            match binding {
                PropBinding::Static(initial) => {
                    let mut key_fn = key_fn;
                    let mut view_fn = view_fn;
                    apply(initial, &mut key_fn, &mut view_fn)
                }
                PropBinding::Reactive(mut compute) => {
                    Effect::new(move || apply(compute(), &mut key_fn, &mut view_fn)).map(|_| ())
                }
                PropBinding::TwoWay(adapter) => {
                    let updating = Arc::new(AtomicBool::new(false));
                    let mut key_fn = key_fn;
                    let mut view_fn = view_fn;
                    let apply_cell = Rc::new(RefCell::new(move |next: Vec<Item>| {
                        apply(next, &mut key_fn, &mut view_fn)
                    }));
                    (adapter.bind_to_control)(
                        Box::new(move |next| (apply_cell.borrow_mut())(next)),
                        updating,
                    )
                }
            }
        })
    }
}

/// Keyed list reconciler that reuses existing realized view handles `V` and
/// updates their per-item [`Signal<T>`] in place when items are added, removed,
/// reordered, or modified.
pub struct KeyedListReconciler<T: 'static, K, V> {
    entries: Vec<(K, V, Signal<T>, ReactiveScope)>,
}

impl<T: 'static, K, V> Default for KeyedListReconciler<T, K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static, K, V> KeyedListReconciler<T, K, V> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<T: 'static, K, V> KeyedListReconciler<T, K, V>
where
    T: Clone + PartialEq,
    K: Eq + Clone,
    V: Clone,
{
    /// Reconciles `next_items` against the currently realized entries.
    ///
    /// Returns `Ok(true)` if the structural key sequence changed (and `apply_list`
    /// was invoked), or `Ok(false)` if all keys remained in the exact same order
    /// (in which case only per-item signals were updated in place). Any removed
    /// item's [`ReactiveScope`] is automatically disposed.
    pub fn reconcile(
        &mut self,
        next_items: Vec<T>,
        key_fn: &mut impl FnMut(&T) -> K,
        build_view: &mut impl FnMut(Signal<T>) -> Result<V>,
        apply_list: impl FnOnce(&[V]) -> Result<()>,
    ) -> Result<bool> {
        let old_keys: Vec<K> = self.entries.iter().map(|(k, _, _, _)| k.clone()).collect();
        let mut old_entries: Vec<Option<(K, V, Signal<T>, ReactiveScope)>> =
            self.entries.drain(..).map(Some).collect();
        let mut next_entries = Vec::with_capacity(next_items.len());
        let mut next_keys = Vec::with_capacity(next_items.len());

        for item in next_items {
            let key = key_fn(&item);
            next_keys.push(key.clone());
            let found = old_entries
                .iter_mut()
                .find(|slot| slot.as_ref().is_some_and(|(k, _, _, _)| *k == key))
                .and_then(Option::take);

            if let Some((existing_key, view, signal, item_scope)) = found {
                signal.set_if_changed(item);
                next_entries.push((existing_key, view, signal, item_scope));
            } else {
                let signal = Signal::new(item);
                let item_scope = ReactiveScope::new();
                let view = item_scope.run(|| build_view(signal))?;
                next_entries.push((key, view, signal, item_scope));
            }
        }

        // Dispose scopes for any removed items.
        for (_, _, _, removed_scope) in old_entries.into_iter().flatten() {
            removed_scope.dispose();
        }

        let keys_changed = old_keys != next_keys;
        if keys_changed {
            let views: Vec<V> = next_entries.iter().map(|(_, v, _, _)| v.clone()).collect();
            apply_list(&views)?;
        }
        self.entries = next_entries;
        Ok(keys_changed)
    }
}

/// Reactive conditional view primitive (`Show::when` / `Show::when_else`).
pub struct Show;

impl Show {
    /// Renders `then_view` inside a lightweight [`Border`] container when
    /// `condition` evaluates to `true`, and clears the container when `false`.
    pub fn when<M, F, E>(
        condition: impl IntoProp<bool, M> + 'static,
        mut then_view: F,
    ) -> El<Border>
    where
        F: FnMut() -> E + 'static,
        E: IntoContentEl + 'static,
    {
        Self::when_else(
            condition,
            move || then_view().into_content_el(),
            || None::<AnyEl>,
        )
    }

    /// Renders `then_view` when `condition` is `true` and `else_view` when `false`,
    /// swapping the mounted child only when the boolean condition flips.
    pub fn when_else<M, FT, ET, FE, EE>(
        condition: impl IntoProp<bool, M> + 'static,
        mut then_view: FT,
        mut else_view: FE,
    ) -> El<Border>
    where
        FT: FnMut() -> ET + 'static,
        ET: IntoContentEl + 'static,
        FE: FnMut() -> EE + 'static,
        EE: IntoContentEl + 'static,
    {
        let binding = condition.into_prop();
        Border::el().with_op(move |scope, border| {
            let scope = scope.clone();
            let border = border.clone();
            let last_state: Rc<RefCell<Option<bool>>> = Rc::new(RefCell::new(None));
            let active_scope: Rc<RefCell<Option<ReactiveScope>>> = Rc::new(RefCell::new(None));

            let mut update = move |state: bool| -> Result<()> {
                if *last_state.borrow() == Some(state) {
                    return Ok(());
                }
                *last_state.borrow_mut() = Some(state);
                if let Some(prev_scope) = active_scope.borrow_mut().take() {
                    prev_scope.dispose();
                }
                let branch_scope = ReactiveScope::new();
                let built = branch_scope.run(|| {
                    let next_el = if state {
                        then_view().into_content_el()
                    } else {
                        else_view().into_content_el()
                    };
                    next_el.map(|el| el.build(&scope)).transpose()
                })?;
                *active_scope.borrow_mut() = Some(branch_scope);
                border.set_child(built.as_ref().map(|c| c as &dyn AsControl))
            };

            match binding {
                PropBinding::Static(state) => update(state),
                PropBinding::Reactive(mut compute) => {
                    Effect::new(move || update(compute())).map(|_| ())
                }
                PropBinding::TwoWay(adapter) => {
                    let updating = Arc::new(AtomicBool::new(false));
                    let update_cell = Rc::new(RefCell::new(update));
                    (adapter.bind_to_control)(
                        Box::new(move |state| (update_cell.borrow_mut())(state)),
                        updating,
                    )
                }
            }
        })
    }
}

/// Reactive keyed list view primitive (`For::each`).
pub struct For;

impl For {
    /// Renders a reactive keyed list inside a [`StackPanel`], reusing realized
    /// controls and updating per-item [`Signal<Item>`] handles in place.
    pub fn each<Item, K, M, KF, VF, E>(
        items: impl IntoProp<Vec<Item>, M> + 'static,
        key_fn: KF,
        view_fn: VF,
    ) -> El<StackPanel>
    where
        Item: Clone + PartialEq + 'static,
        K: Eq + Clone + 'static,
        KF: FnMut(&Item) -> K + 'static,
        VF: FnMut(Signal<Item>) -> E + 'static,
        E: IntoChildEl + 'static,
    {
        StackPanel::el().children_for(items, key_fn, view_fn)
    }
}

/// A `Copy` slot for dynamically mounting, replacing, or clearing a declarative
/// [`El`] / `rsx!` subtree inside an existing view hierarchy at runtime.
#[derive(Clone, Copy, Debug, Default)]
pub struct ControlSlot {
    host: NodeRef<Border>,
    active_scope: NodeRef<ReactiveScope>,
}

impl ControlSlot {
    pub fn new() -> Self {
        Self {
            host: NodeRef::new(),
            active_scope: NodeRef::new(),
        }
    }

    /// Returns the host [`El<Border>`] to place inside a parent `rsx!` or `El` tree.
    pub fn host(&self) -> El<Border> {
        Border::el().node_ref(self.host)
    }

    /// Returns the underlying host [`Border`] if it has been materialized.
    pub fn host_control(&self) -> Option<Border> {
        self.host.get()
    }

    /// Materializes `view` inside a new [`ReactiveScope`] and mounts it into this
    /// slot's host container, disposing any previous subtree's scope.
    pub fn mount(&self, scope: &AppScope, view: impl IntoContentEl) -> Result<()> {
        if let Some(prev) = self.active_scope.get() {
            prev.dispose();
        }
        let next_scope = ReactiveScope::new();
        self.active_scope.set(next_scope);
        let Some(border) = self.host.get() else {
            return Ok(());
        };
        let built =
            next_scope.run(|| view.into_content_el().map(|el| el.build(scope)).transpose())?;
        border.set_child(built.as_ref().map(|c| c as &dyn AsControl))
    }

    /// Clears any currently mounted child and disposes its [`ReactiveScope`].
    pub fn clear(&self) -> Result<()> {
        if let Some(prev) = self.active_scope.get() {
            prev.dispose();
        }
        let Some(border) = self.host.get() else {
            return Ok(());
        };
        border.set_child(None)
    }
}

/// A safe, reactive wrapper around `IAvnCommand` that supports automatic
/// `CanExecuteChanged` notifications driven by [`Signal<bool>`], [`Memo<bool>`],
/// or reactive closures.
#[derive(Clone, Debug)]
pub struct Command {
    pub(crate) raw: sys::Command,
    can_execute_flag: Arc<AtomicBool>,
    _effect: Option<Effect>,
}

impl Command {
    /// Creates an always-enabled command that invokes `execute` with the command parameter.
    pub fn new(mut execute: impl FnMut(Variant) -> Result<()> + Send + 'static) -> Self {
        let can_execute_flag = Arc::new(AtomicBool::new(true));
        let flag_for_query = can_execute_flag.clone();
        let raw = sys::command(
            move |variant| {
                execute(Variant::from_abi(variant)).map_err(|error| match error {
                    crate::Error::Abi(error) => error,
                    _ => sys::Error(sys::E_FAIL),
                })
            },
            move |_| Ok(flag_for_query.load(Ordering::Acquire)),
        );
        Self {
            raw,
            can_execute_flag,
            _effect: None,
        }
    }

    /// Creates a command whose `CanExecute` state is reactively driven by `can_execute`
    /// (a `bool`, [`Signal<bool>`], [`Memo<bool>`], or closure), automatically firing
    /// `CanExecuteChanged` when the state changes.
    pub fn with_can_execute<M>(
        can_execute: impl IntoProp<bool, M> + 'static,
        mut execute: impl FnMut(Variant) -> Result<()> + Send + 'static,
    ) -> Result<Self> {
        let can_execute_flag = Arc::new(AtomicBool::new(true));
        let flag_for_query = can_execute_flag.clone();
        let raw = sys::command(
            move |variant| {
                execute(Variant::from_abi(variant)).map_err(|error| match error {
                    crate::Error::Abi(error) => error,
                    _ => sys::Error(sys::E_FAIL),
                })
            },
            move |_| Ok(flag_for_query.load(Ordering::Acquire)),
        );

        let effect = match can_execute.into_prop() {
            PropBinding::Static(initial) => {
                can_execute_flag.store(initial, Ordering::Release);
                None
            }
            PropBinding::Reactive(mut compute) => {
                let flag_for_effect = can_execute_flag.clone();
                let raw_for_effect = raw.clone();
                Some(Effect::new(move || {
                    let next = compute();
                    let prev = flag_for_effect.swap(next, Ordering::AcqRel);
                    if prev != next {
                        raw_for_effect.notify();
                    }
                    Ok(())
                })?)
            }
            PropBinding::TwoWay(adapter) => {
                let flag_for_effect = can_execute_flag.clone();
                let raw_for_effect = raw.clone();
                let updating = Arc::new(AtomicBool::new(false));
                (adapter.bind_to_control)(
                    Box::new(move |next| {
                        let prev = flag_for_effect.swap(next, Ordering::AcqRel);
                        if prev != next {
                            raw_for_effect.notify();
                        }
                        Ok(())
                    }),
                    updating,
                )?;
                None
            }
        };

        Ok(Self {
            raw,
            can_execute_flag,
            _effect: effect,
        })
    }

    /// Returns whether this command is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.can_execute_flag.load(Ordering::Acquire)
    }

    /// Manually fires `CanExecuteChanged` notifications to all subscribed controls.
    pub fn notify(&self) {
        self.raw.notify();
    }

    /// Invokes the command directly from Rust with `parameter`.
    pub fn execute(&self, parameter: impl Into<Variant>) -> Result<()> {
        let abi = parameter.into().to_abi()?;
        Ok(self.raw.as_com_ptr().execute(*abi)?)
    }

    pub fn as_com_ptr(&self) -> &sys::ComPtr<sys::IAvnCommand> {
        self.raw.as_com_ptr()
    }
}

/// Polymorphic conversion into an optional `IAvnCommand` pointer for control setters.
pub trait IntoCommandProp {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>>;
}

impl IntoCommandProp for Command {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>> {
        Some(self.raw.as_com_ptr().clone())
    }
}

impl IntoCommandProp for &Command {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>> {
        Some(self.raw.as_com_ptr().clone())
    }
}

impl IntoCommandProp for Option<Command> {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>> {
        self.map(|cmd| cmd.raw.as_com_ptr().clone())
    }
}

impl IntoCommandProp for sys::ComPtr<sys::IAvnCommand> {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>> {
        Some(self)
    }
}

impl IntoCommandProp for Option<sys::ComPtr<sys::IAvnCommand>> {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>> {
        self
    }
}

impl IntoCommandProp for Option<&sys::ComPtr<sys::IAvnCommand>> {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>> {
        self.cloned()
    }
}

/// Returns a reactive [`Memo<Option<Brush>>`] that switches between `light` and
/// `dark` whenever `theme` changes.
pub fn theme_brush(
    theme: Signal<ThemeVariant>,
    light: impl Into<Brush>,
    dark: impl Into<Brush>,
) -> Memo<Option<Brush>> {
    let light = light.into();
    let dark = dark.into();
    Memo::new(move || {
        Some(match theme.get() {
            ThemeVariant::Dark => dark,
            ThemeVariant::Light | ThemeVariant::Default => light,
        })
    })
}

/// A type-erased declarative element that materializes into a [`Control`].
pub struct AnyEl {
    build: Box<dyn FnOnce(&AppScope) -> Result<Control>>,
}

impl AnyEl {
    pub fn new<T: AsControl + 'static>(el: El<T>) -> Self {
        Self {
            build: Box::new(move |scope| {
                let built = el.build(scope)?;
                Ok(Control {
                    raw: built.as_control()?,
                })
            }),
        }
    }

    pub fn from_control<T: AsControl + 'static>(control: T) -> Self {
        Self {
            build: Box::new(move |_| {
                Ok(Control {
                    raw: control.as_control()?,
                })
            }),
        }
    }

    pub fn build(self, scope: &AppScope) -> Result<Control> {
        (self.build)(scope)
    }
}

impl<T: AsControl + 'static> From<El<T>> for AnyEl {
    fn from(el: El<T>) -> Self {
        Self::new(el)
    }
}

impl From<&str> for AnyEl {
    fn from(text: &str) -> Self {
        let text = text.to_owned();
        Self::new(TextBlock::el().text(text))
    }
}

impl From<String> for AnyEl {
    fn from(text: String) -> Self {
        Self::new(TextBlock::el().text(text))
    }
}

impl From<&String> for AnyEl {
    fn from(text: &String) -> Self {
        Self::new(TextBlock::el().text(text.clone()))
    }
}

impl From<Signal<String>> for AnyEl {
    fn from(signal: Signal<String>) -> Self {
        Self::new(TextBlock::el().text(signal))
    }
}

impl From<Memo<String>> for AnyEl {
    fn from(memo: Memo<String>) -> Self {
        Self::new(TextBlock::el().text(memo))
    }
}

/// Conversion into a child [`AnyEl`] for panel/items containers.
pub trait IntoChildEl {
    fn into_child_el(self) -> AnyEl;
}

impl<T: Into<AnyEl>> IntoChildEl for T {
    fn into_child_el(self) -> AnyEl {
        self.into()
    }
}

/// Conversion into an optional [`AnyEl`] for `Content`, `Header`, and `Decorator::Child` properties.
pub trait IntoContentEl {
    fn into_content_el(self) -> Option<AnyEl>;
}

impl IntoContentEl for Option<AnyEl> {
    fn into_content_el(self) -> Option<AnyEl> {
        self
    }
}

impl<T: AsControl + 'static> IntoContentEl for Option<El<T>> {
    fn into_content_el(self) -> Option<AnyEl> {
        self.map(AnyEl::new)
    }
}

impl<T: Into<AnyEl>> IntoContentEl for T {
    fn into_content_el(self) -> Option<AnyEl> {
        Some(self.into())
    }
}

/// Builds a reactive, container-pooling [`DataTemplate`] for virtualized controls
/// (`ListBox`, `TreeView`, `ContentControl`).
///
/// When a realized container leaves the logical tree during scrolling, its
/// `(Control, Signal<Variant>)` pair is returned to an internal pool; the next
/// item scrolled into view reuses the pooled control and updates its
/// `Signal<Variant>` in place without allocating new COM controls.
pub fn data_template_reactive<E>(
    scope: &AppScope,
    mut matches: impl FnMut(&Variant) -> bool + Send + 'static,
    mut build_view: impl FnMut(Signal<Variant>) -> E + Send + 'static,
) -> Result<DataTemplate>
where
    E: Into<AnyEl>,
{
    let scope = scope.clone();
    let last_item: Arc<Mutex<Variant>> = Arc::new(Mutex::new(Variant::None));
    let last_item_for_match = last_item.clone();
    let pool: Arc<Mutex<Vec<(Control, Signal<Variant>)>>> = Arc::new(Mutex::new(Vec::new()));

    crate::data_template(
        move |variant| {
            let matched = matches(&variant);
            if matched {
                if let Ok(mut slot) = last_item_for_match.lock() {
                    *slot = variant;
                }
            }
            matched
        },
        move || {
            let item = last_item
                .lock()
                .map(|slot| slot.clone())
                .unwrap_or(Variant::None);

            if let Ok(mut pooled) = pool.lock() {
                if let Some((control, signal)) = pooled.pop() {
                    signal.set(item);
                    return Ok(control);
                }
            }

            let signal = Signal::new(item);
            let control = build_view(signal).into().build(&scope)?;
            let pool_for_detach = pool.clone();
            let control_for_detach = control.clone();
            let subscription = control.subscribe_detached_from_logical_tree(move |_| {
                if let Ok(mut pooled) = pool_for_detach.lock() {
                    pooled.push((control_for_detach.clone(), signal));
                }
            })?;
            scope.retain_subscription(subscription);
            Ok(control)
        },
    )
}

impl AppScope {
    /// Creates a fine-grained reactive [`Signal`] tied to this application scope.
    pub fn signal<T: 'static>(&self, initial: T) -> Signal<T> {
        Signal::new(initial)
    }

    /// Creates a derived [`Memo`] tied to this application scope.
    pub fn memo<T: Clone + PartialEq + 'static>(
        &self,
        compute: impl FnMut() -> T + 'static,
    ) -> Memo<T> {
        Memo::new(compute)
    }

    /// Registers and immediately runs a reactive [`Effect`] tied to this application scope.
    pub fn effect(&self, f: impl FnMut() -> Result<()> + 'static) -> Result<Effect> {
        Effect::new(f)
    }

    /// Creates a [`NodeRef`] slot for capturing a materialized control from `El<T>` or `rsx!`.
    pub fn node_ref<T: 'static>(&self) -> NodeRef<T> {
        NodeRef::new()
    }

    /// Creates a [`ControlSlot`] for dynamically mounting or swapping `rsx!` subtrees.
    pub fn control_slot(&self) -> ControlSlot {
        ControlSlot::new()
    }

    /// Creates an asynchronous reactive [`Resource<T>`] that runs `fetcher` on a
    /// background task whenever the value returned by `source` changes.
    ///
    /// If `source` changes again while a previous fetch is still in flight, the
    /// stale result is discarded automatically.
    pub fn create_resource<S, T, Fut>(
        &self,
        mut source: impl FnMut() -> S + 'static,
        fetcher: impl Fn(S) -> Fut + 'static,
    ) -> Result<Resource<T>>
    where
        S: PartialEq + Clone + Send + 'static,
        T: Clone + Send + 'static,
        Fut: Future<Output = std::result::Result<T, String>> + Send + 'static,
    {
        let resource = Resource::<T>::pending();
        let scope = self.clone();
        let last_source: Rc<RefCell<Option<S>>> = Rc::new(RefCell::new(None));

        Effect::new(move || {
            let next_source = source();
            if last_source.borrow().as_ref() == Some(&next_source) {
                return Ok(());
            }
            *last_source.borrow_mut() = Some(next_source.clone());

            let gen = resource.next_generation();
            resource.set_loading();
            let fut = fetcher(next_source);
            let scope_for_post = scope.clone();
            scope.spawn(async move {
                let outcome = fut.await;
                let _ = scope_for_post.post(move || {
                    if resource.current_generation() == gen {
                        match outcome {
                            Ok(value) => resource.set_ready(value),
                            Err(error) => resource.set_error(error),
                        }
                    }
                });
            })
        })?;

        Ok(resource)
    }

    /// Creates a reactive [`Signal<ThemeVariant>`] that tracks `ActualThemeVariantChanged`
    /// on `host` (for example a [`Window`] or root [`Control`]).
    pub fn theme_variant_signal(
        &self,
        host: &impl Upcast<StyledElement>,
    ) -> Result<Signal<ThemeVariant>> {
        let initial = self.actual_theme_variant()?;
        let signal = Signal::new(initial);
        let app_scope = self.clone();
        let subscription = host
            .upcast_ref()
            .subscribe_actual_theme_variant_changed(move |_| {
                if let Ok(variant) = app_scope.actual_theme_variant() {
                    signal.set_if_changed(variant);
                }
            })?;
        self.retain_subscription(subscription);
        Ok(signal)
    }

    /// Creates a reactive [`Signal<Option<ResourceValue>>`] that resolves `key`
    /// and updates automatically when `ActualThemeVariantChanged` or
    /// `ResourcesChanged` fires on `host`.
    pub fn resource_signal(
        &self,
        host: &impl Upcast<StyledElement>,
        key: impl Into<String>,
    ) -> Result<Signal<Option<ResourceValue>>> {
        let key = key.into();
        let theme = self.actual_theme_variant()?;
        let initial = self.find_resource(&key, theme)?;
        let signal = Signal::new(initial);

        let styled = host.upcast_ref();
        let scope_for_theme = self.clone();
        let key_for_theme = key.clone();
        let theme_sub = styled.subscribe_actual_theme_variant_changed(move |_| {
            if let Ok(variant) = scope_for_theme.actual_theme_variant() {
                if let Ok(val) = scope_for_theme.find_resource(&key_for_theme, variant) {
                    signal.set_if_changed(val);
                }
            }
        })?;
        self.retain_subscription(theme_sub);

        let scope_for_res = self.clone();
        let res_sub = styled.subscribe_resources_changed(move |_| {
            if let Ok(variant) = scope_for_res.actual_theme_variant() {
                if let Ok(val) = scope_for_res.find_resource(&key, variant) {
                    signal.set_if_changed(val);
                }
            }
        })?;
        self.retain_subscription(res_sub);

        Ok(signal)
    }

    /// Materializes a declarative [`El<Window>`] and mounts it into this application scope.
    pub fn mount_view(&self, window: El<Window>) -> Result<Window> {
        let built = window.build(self)?;
        self.mount(built.clone())?;
        Ok(built)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __rsx_attached {
    ($el:ident, Canvas, left, $val:expr) => {
        $el.canvas_left($val)
    };
    ($el:ident, Canvas, Left, $val:expr) => {
        $el.canvas_left($val)
    };
    ($el:ident, Canvas, top, $val:expr) => {
        $el.canvas_top($val)
    };
    ($el:ident, Canvas, Top, $val:expr) => {
        $el.canvas_top($val)
    };
    ($el:ident, Canvas, right, $val:expr) => {
        $el.canvas_right($val)
    };
    ($el:ident, Canvas, Right, $val:expr) => {
        $el.canvas_right($val)
    };
    ($el:ident, Canvas, bottom, $val:expr) => {
        $el.canvas_bottom($val)
    };
    ($el:ident, Canvas, Bottom, $val:expr) => {
        $el.canvas_bottom($val)
    };
    ($el:ident, DockPanel, dock, $val:expr) => {
        $el.dock_panel_dock($val)
    };
    ($el:ident, DockPanel, Dock, $val:expr) => {
        $el.dock_panel_dock($val)
    };
    ($el:ident, Grid, column, $val:expr) => {
        $el.grid_column($val)
    };
    ($el:ident, Grid, Column, $val:expr) => {
        $el.grid_column($val)
    };
    ($el:ident, Grid, row, $val:expr) => {
        $el.grid_row($val)
    };
    ($el:ident, Grid, Row, $val:expr) => {
        $el.grid_row($val)
    };
    ($el:ident, Grid, column_span, $val:expr) => {
        $el.grid_column_span($val)
    };
    ($el:ident, Grid, ColumnSpan, $val:expr) => {
        $el.grid_column_span($val)
    };
    ($el:ident, Grid, row_span, $val:expr) => {
        $el.grid_row_span($val)
    };
    ($el:ident, Grid, RowSpan, $val:expr) => {
        $el.grid_row_span($val)
    };
    ($el:ident, Grid, is_shared_size_scope, $val:expr) => {
        $el.grid_is_shared_size_scope($val)
    };
    ($el:ident, Grid, IsSharedSizeScope, $val:expr) => {
        $el.grid_is_shared_size_scope($val)
    };
    ($el:ident, RelativePanel, align_left_with_panel, $val:expr) => {
        $el.relative_panel_align_left_with_panel($val)
    };
    ($el:ident, RelativePanel, AlignLeftWithPanel, $val:expr) => {
        $el.relative_panel_align_left_with_panel($val)
    };
    ($el:ident, RelativePanel, align_right_with_panel, $val:expr) => {
        $el.relative_panel_align_right_with_panel($val)
    };
    ($el:ident, RelativePanel, AlignRightWithPanel, $val:expr) => {
        $el.relative_panel_align_right_with_panel($val)
    };
    ($el:ident, RelativePanel, align_top_with_panel, $val:expr) => {
        $el.relative_panel_align_top_with_panel($val)
    };
    ($el:ident, RelativePanel, AlignTopWithPanel, $val:expr) => {
        $el.relative_panel_align_top_with_panel($val)
    };
    ($el:ident, RelativePanel, align_bottom_with_panel, $val:expr) => {
        $el.relative_panel_align_bottom_with_panel($val)
    };
    ($el:ident, RelativePanel, AlignBottomWithPanel, $val:expr) => {
        $el.relative_panel_align_bottom_with_panel($val)
    };
    ($el:ident, RelativePanel, align_horizontal_center_with_panel, $val:expr) => {
        $el.relative_panel_align_horizontal_center_with_panel($val)
    };
    ($el:ident, RelativePanel, AlignHorizontalCenterWithPanel, $val:expr) => {
        $el.relative_panel_align_horizontal_center_with_panel($val)
    };
    ($el:ident, RelativePanel, align_vertical_center_with_panel, $val:expr) => {
        $el.relative_panel_align_vertical_center_with_panel($val)
    };
    ($el:ident, RelativePanel, AlignVerticalCenterWithPanel, $val:expr) => {
        $el.relative_panel_align_vertical_center_with_panel($val)
    };
    ($el:ident, ToolTip, tip, $val:expr) => {
        $el.tool_tip_tip($val)
    };
    ($el:ident, ToolTip, Tip, $val:expr) => {
        $el.tool_tip_tip($val)
    };
    ($el:ident, ToolTip, is_open, $val:expr) => {
        $el.tool_tip_is_open($val)
    };
    ($el:ident, ToolTip, IsOpen, $val:expr) => {
        $el.tool_tip_is_open($val)
    };
    ($el:ident, ToolTip, placement, $val:expr) => {
        $el.tool_tip_placement($val)
    };
    ($el:ident, ToolTip, Placement, $val:expr) => {
        $el.tool_tip_placement($val)
    };
    ($el:ident, ToolTip, horizontal_offset, $val:expr) => {
        $el.tool_tip_horizontal_offset($val)
    };
    ($el:ident, ToolTip, HorizontalOffset, $val:expr) => {
        $el.tool_tip_horizontal_offset($val)
    };
    ($el:ident, ToolTip, vertical_offset, $val:expr) => {
        $el.tool_tip_vertical_offset($val)
    };
    ($el:ident, ToolTip, VerticalOffset, $val:expr) => {
        $el.tool_tip_vertical_offset($val)
    };
    ($el:ident, ToolTip, show_delay, $val:expr) => {
        $el.tool_tip_show_delay($val)
    };
    ($el:ident, ToolTip, ShowDelay, $val:expr) => {
        $el.tool_tip_show_delay($val)
    };
    ($el:ident, ToolTip, between_show_delay, $val:expr) => {
        $el.tool_tip_between_show_delay($val)
    };
    ($el:ident, ToolTip, BetweenShowDelay, $val:expr) => {
        $el.tool_tip_between_show_delay($val)
    };
    ($el:ident, ToolTip, show_on_disabled, $val:expr) => {
        $el.tool_tip_show_on_disabled($val)
    };
    ($el:ident, ToolTip, ShowOnDisabled, $val:expr) => {
        $el.tool_tip_show_on_disabled($val)
    };
    ($el:ident, ToolTip, service_enabled, $val:expr) => {
        $el.tool_tip_service_enabled($val)
    };
    ($el:ident, ToolTip, ServiceEnabled, $val:expr) => {
        $el.tool_tip_service_enabled($val)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __rsx_body {
    ($el:ident $(,)*) => {
        $el
    };
    ($el:ident, ref : $val:expr , $($rest:tt)*) => {{
        let $el = $el.node_ref($val);
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, ref : $val:expr) => {{
        $el.node_ref($val)
    }};
    ($el:ident, $owner:ident . $prop:ident : $val:expr , $($rest:tt)*) => {{
        let $el = $crate::__rsx_attached!($el, $owner, $prop, $val);
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, $owner:ident . $prop:ident : $val:expr) => {{
        $crate::__rsx_attached!($el, $owner, $prop, $val)
    }};
    ($el:ident, $prop:ident : $val:expr , $($rest:tt)*) => {{
        let $el = $el.$prop($val);
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, $prop:ident : $val:expr) => {{
        $el.$prop($val)
    }};
    ($el:ident, if ($cond:expr) { $($then_body:tt)* } else { $($else_body:tt)* } $(,)? $($rest:tt)*) => {{
        let $el = if $cond {
            $crate::__rsx_body!($el, $($then_body)*)
        } else {
            $crate::__rsx_body!($el, $($else_body)*)
        };
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, if $cond:ident { $($then_body:tt)* } else { $($else_body:tt)* } $(,)? $($rest:tt)*) => {{
        let $el = if $cond {
            $crate::__rsx_body!($el, $($then_body)*)
        } else {
            $crate::__rsx_body!($el, $($else_body)*)
        };
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, if ($cond:expr) { $($then_body:tt)* } $(,)? $($rest:tt)*) => {{
        let $el = if $cond {
            $crate::__rsx_body!($el, $($then_body)*)
        } else {
            $el
        };
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, if $cond:ident { $($then_body:tt)* } $(,)? $($rest:tt)*) => {{
        let $el = if $cond {
            $crate::__rsx_body!($el, $($then_body)*)
        } else {
            $el
        };
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, for $pat:pat in ($iter:expr) { $($loop_body:tt)* } $(,)? $($rest:tt)*) => {{
        let mut __acc = $el;
        for $pat in $iter {
            __acc = $crate::__rsx_body!(__acc, $($loop_body)*);
        }
        let $el = __acc;
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, for $pat:pat in $iter:ident { $($loop_body:tt)* } $(,)? $($rest:tt)*) => {{
        let mut __acc = $el;
        for $pat in $iter {
            __acc = $crate::__rsx_body!(__acc, $($loop_body)*);
        }
        let $el = __acc;
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, $ns:ident :: $child:ident { $($child_body:tt)* } $(,)? $($rest:tt)*) => {{
        let $el = $el.rsx_child($crate::rsx!($ns::$child { $($child_body)* }));
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, $child:ident { $($child_body:tt)* } $(,)? $($rest:tt)*) => {{
        let $el = $el.rsx_child($crate::rsx!($child { $($child_body)* }));
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, $lit:literal $(,)? $($rest:tt)*) => {{
        let $el = $el.rsx_child($lit);
        $crate::__rsx_body!($el, $($rest)*)
    }};
    ($el:ident, { $expr:expr } $(,)? $($rest:tt)*) => {{
        let $el = $el.rsx_child($expr);
        $crate::__rsx_body!($el, $($rest)*)
    }};
}

/// Declarative XAML-like macro for building [`El<T>`](crate::view::El) trees in pure Rust.
///
/// Supports properties (`width: 100.0`), attached properties (`Grid.row: 1`,
/// `DockPanel.dock: Dock::Top`), event handlers (`on_click: move |_| ...`),
/// node references (`ref: my_ref`), style classes (`class: "primary"`),
/// string/expression children (`"Submit"`, `{ expr }`), and inline `if`/`for` blocks.
#[macro_export]
macro_rules! rsx {
    ($control:ident { $($body:tt)* }) => {{
        let __el = $control::el();
        $crate::__rsx_body!(__el, $($body)*)
    }};
    ($ns:ident :: $control:ident { $($body:tt)* }) => {{
        let __el = $ns::$control::el();
        $crate::__rsx_body!(__el, $($body)*)
    }};
}
