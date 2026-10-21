//! Declarative element builders (`El<T>`, `AnyEl`), polymorphic property conversions
//! (`IntoValue`, `IntoProp`), and pooled reactive `DataTemplate` support.

use crate::reactive::{Effect, Memo, NodeRef, Signal, TwoWay};
use crate::{
    AppScope, AsControl, Brush, Color, Control, CornerRadius, DataTemplate, EventSubscription,
    LinearGradientBrush, Paint, PixelPoint, Point, RadialGradientBrush, Rect, Result, Size,
    StyledElement, TextBlock, Thickness, Upcast, Variant, Vector, Window,
};
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

    /// Materializes the control and applies all deferred properties, children,
    /// and event subscriptions within `scope`.
    pub fn build(self, scope: &AppScope) -> Result<T> {
        let instance = (self.create)()?;
        for op in self.ops {
            op(scope, &instance)?;
        }
        Ok(instance)
    }
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
