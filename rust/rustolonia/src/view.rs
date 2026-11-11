//! Declarative element builders (`El<T>`, `AnyEl`), polymorphic property conversions
//! (`IntoValue`, `IntoProp`), and pooled reactive `DataTemplate` support.

use crate::reactive::{
    on_scope_cleanup, provide_context, Effect, Memo, NodeRef, PathTwoWay, PropertyPath,
    ReactiveScope, Resource, ResourceState, Signal, TwoWay, UiCallbackHandle,
};
use crate::{
    AppScope, AsControl, Border, Brush, CivilDate, CivilTime, Color, ContentControl, Control,
    CornerRadius, DataTemplate, Decimal, Decorator, EventSubscription, GridLength, GridTrack,
    GridTracks, ItemsControl, LinearGradientBrush, Paint, Panel, PixelPoint, Point,
    RadialGradientBrush, Rect, ResourceValue, Result, Size, StackPanel, StyledElement, TextBlock,
    ThemeVariant, Thickness, Upcast, Variant, Vector, Window,
};
use rustolonia_sys as sys;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

impl IntoValue<Option<Paint>> for Paint {
    fn into_value(self) -> Option<Paint> {
        Some(self)
    }
}

impl IntoValue<Option<Paint>> for Brush {
    fn into_value(self) -> Option<Paint> {
        Some(Paint::Solid(self))
    }
}

impl IntoValue<Option<Paint>> for Option<Brush> {
    fn into_value(self) -> Option<Paint> {
        self.map(Paint::Solid)
    }
}

impl IntoValue<Option<Paint>> for Color {
    fn into_value(self) -> Option<Paint> {
        Some(Paint::Solid(Brush::solid(self)))
    }
}

impl IntoValue<Option<Paint>> for LinearGradientBrush {
    fn into_value(self) -> Option<Paint> {
        Some(Paint::LinearGradient(self))
    }
}

impl IntoValue<Option<Paint>> for RadialGradientBrush {
    fn into_value(self) -> Option<Paint> {
        Some(Paint::RadialGradient(self))
    }
}

impl IntoValue<Option<Paint>> for (u8, u8, u8) {
    fn into_value(self) -> Option<Paint> {
        Some(Paint::Solid(Brush::solid(Color::rgb(
            self.0, self.1, self.2,
        ))))
    }
}

impl IntoValue<Option<Command>> for Command {
    fn into_value(self) -> Option<Command> {
        Some(self)
    }
}

impl IntoValue<Option<Command>> for &Command {
    fn into_value(self) -> Option<Command> {
        Some(self.clone())
    }
}

impl IntoValue<Option<DataTemplate>> for DataTemplate {
    fn into_value(self) -> Option<DataTemplate> {
        Some(self)
    }
}

impl IntoValue<Option<DataTemplate>> for &DataTemplate {
    fn into_value(self) -> Option<DataTemplate> {
        Some(self.clone())
    }
}

/// Converts values accepted by grid track setters, reporting invalid strings.
pub trait TryIntoGridTracks {
    fn try_into_grid_tracks(&self) -> Result<GridTracks>;
}

pub trait GridTracksString: TryIntoGridTracks {}

impl TryIntoGridTracks for GridTracks {
    fn try_into_grid_tracks(&self) -> Result<GridTracks> {
        Ok(self.clone())
    }
}

impl TryIntoGridTracks for &str {
    fn try_into_grid_tracks(&self) -> Result<GridTracks> {
        self.parse()
    }
}

impl GridTracksString for &str {}

impl TryIntoGridTracks for String {
    fn try_into_grid_tracks(&self) -> Result<GridTracks> {
        self.parse()
    }
}

impl GridTracksString for String {}

impl TryIntoGridTracks for &String {
    fn try_into_grid_tracks(&self) -> Result<GridTracks> {
        self.parse()
    }
}

impl GridTracksString for &String {}

impl TryIntoGridTracks for Vec<GridTrack> {
    fn try_into_grid_tracks(&self) -> Result<GridTracks> {
        Ok(GridTracks(self.clone()))
    }
}

impl TryIntoGridTracks for &[GridTrack] {
    fn try_into_grid_tracks(&self) -> Result<GridTracks> {
        Ok(GridTracks(self.to_vec()))
    }
}

impl<const N: usize> TryIntoGridTracks for [GridTrack; N] {
    fn try_into_grid_tracks(&self) -> Result<GridTracks> {
        Ok(GridTracks::from(self.clone()))
    }
}

impl<const N: usize> TryIntoGridTracks for [GridLength; N] {
    fn try_into_grid_tracks(&self) -> Result<GridTracks> {
        Ok(GridTracks::from(*self))
    }
}

impl IntoProp<GridTracks, markers::GridTracksStringMarker> for &str {
    fn into_prop(self) -> PropBinding<GridTracks> {
        let value = self.to_owned();
        PropBinding::Fallible(Box::new(move || value.parse()))
    }
}

impl IntoProp<GridTracks, markers::GridTracksStringMarker> for String {
    fn into_prop(self) -> PropBinding<GridTracks> {
        PropBinding::Fallible(Box::new(move || self.parse()))
    }
}

impl IntoProp<GridTracks, markers::GridTracksStringMarker> for &String {
    fn into_prop(self) -> PropBinding<GridTracks> {
        let value = self.clone();
        PropBinding::Fallible(Box::new(move || value.parse()))
    }
}

impl<F, V> IntoProp<GridTracks, markers::GridTracksStringMarker> for F
where
    F: FnMut() -> V + 'static,
    V: GridTracksString + 'static,
{
    fn into_prop(mut self) -> PropBinding<GridTracks> {
        PropBinding::Fallible(Box::new(move || {
            let value = self();
            TryIntoGridTracks::try_into_grid_tracks(&value)
        }))
    }
}

impl IntoValue<GridTracks> for Vec<GridTrack> {
    fn into_value(self) -> GridTracks {
        GridTracks(self)
    }
}

impl IntoValue<GridTracks> for &[GridTrack] {
    fn into_value(self) -> GridTracks {
        GridTracks(self.to_vec())
    }
}

impl<const N: usize> IntoValue<GridTracks> for [GridTrack; N] {
    fn into_value(self) -> GridTracks {
        GridTracks::from(self)
    }
}

impl<const N: usize> IntoValue<GridTracks> for [GridLength; N] {
    fn into_value(self) -> GridTracks {
        GridTracks::from(self)
    }
}

impl IntoValue<String> for GridTracks {
    fn into_value(self) -> String {
        self.to_string()
    }
}

impl<const N: usize> IntoValue<String> for [GridTrack; N] {
    fn into_value(self) -> String {
        GridTracks::from(self).to_string()
    }
}

impl<const N: usize> IntoValue<String> for [GridLength; N] {
    fn into_value(self) -> String {
        GridTracks::from(self).to_string()
    }
}

impl IntoValue<Option<CivilDate>> for CivilDate {
    fn into_value(self) -> Option<CivilDate> {
        Some(self)
    }
}

impl IntoValue<Option<CivilTime>> for CivilTime {
    fn into_value(self) -> Option<CivilTime> {
        Some(self)
    }
}

impl IntoValue<Decimal> for i32 {
    fn into_value(self) -> Decimal {
        Decimal::from(self)
    }
}

impl IntoValue<Decimal> for i64 {
    fn into_value(self) -> Decimal {
        Decimal::from(self)
    }
}

impl IntoValue<Option<Decimal>> for Decimal {
    fn into_value(self) -> Option<Decimal> {
        Some(self)
    }
}

impl IntoValue<Option<Decimal>> for i32 {
    fn into_value(self) -> Option<Decimal> {
        Some(Decimal::from(self))
    }
}

impl IntoValue<Option<Decimal>> for i64 {
    fn into_value(self) -> Option<Decimal> {
        Some(Decimal::from(self))
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
    pub struct GridTracksStringMarker;
    pub struct SignalMarker;
    pub struct SignalOptionMarker;
    pub struct MemoMarker;
    pub struct MemoOptionMarker;
    pub struct ClosureMarker;
    pub struct PathMarker;
    pub struct PathTwoWayMarker;
    pub struct ConvertedMarker;
    pub struct TwoWayMarker;
    pub struct TwoWayOptionMarker;
}

type SetterFn<T> = Box<dyn Fn(T) -> Result<()>>;

struct ResetUpdating<'a>(&'a AtomicBool);

impl Drop for ResetUpdating<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(crate) fn set_unless_updating<T>(
    updating: &AtomicBool,
    setter: &dyn Fn(T) -> Result<()>,
    value: T,
) -> Result<()> {
    if updating.swap(true, Ordering::AcqRel) {
        return Ok(());
    }
    let _guard = ResetUpdating(updating);
    setter(value)
}

/// Type-erased adapter for two-way signal bindings.
pub struct TwoWayAdapter<T> {
    pub(crate) bind_to_control: Box<dyn FnOnce(SetterFn<T>, Arc<AtomicBool>) -> Result<()>>,
    pub(crate) set_from_control: Box<dyn Fn(T) -> Result<()> + 'static>,
}

/// A static, reactive, or two-way property binding.
pub enum PropBinding<T> {
    Static(T),
    Reactive(Box<dyn FnMut() -> T>),
    Fallible(Box<dyn FnMut() -> Result<T>>),
    FallibleOptional(Box<dyn FnMut() -> Result<Option<T>>>),
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

impl IntoProp<Option<CivilDate>, markers::SignalOptionMarker> for Signal<CivilDate> {
    fn into_prop(self) -> PropBinding<Option<CivilDate>> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || Some(signal.get())))
    }
}

impl IntoProp<Option<CivilTime>, markers::SignalOptionMarker> for Signal<CivilTime> {
    fn into_prop(self) -> PropBinding<Option<CivilTime>> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || Some(signal.get())))
    }
}

impl IntoProp<Option<Decimal>, markers::SignalOptionMarker> for Signal<Decimal> {
    fn into_prop(self) -> PropBinding<Option<Decimal>> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || Some(signal.get())))
    }
}

impl IntoProp<Option<Paint>, markers::SignalOptionMarker> for Signal<Option<Brush>> {
    fn into_prop(self) -> PropBinding<Option<Paint>> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || signal.get().map(Paint::Solid)))
    }
}

impl IntoProp<Option<Paint>, markers::SignalOptionMarker> for Signal<Brush> {
    fn into_prop(self) -> PropBinding<Option<Paint>> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || Some(Paint::Solid(signal.get()))))
    }
}

impl IntoProp<Option<Paint>, markers::SignalOptionMarker> for Signal<Paint> {
    fn into_prop(self) -> PropBinding<Option<Paint>> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || Some(signal.get())))
    }
}

impl IntoProp<Option<Paint>, markers::SignalOptionMarker> for Signal<Color> {
    fn into_prop(self) -> PropBinding<Option<Paint>> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || {
            Some(Paint::Solid(Brush::solid(signal.get())))
        }))
    }
}

impl IntoProp<Option<Command>, markers::SignalOptionMarker> for Signal<Command> {
    fn into_prop(self) -> PropBinding<Option<Command>> {
        let signal = self;
        PropBinding::Reactive(Box::new(move || Some(signal.get())))
    }
}

impl IntoProp<Option<DataTemplate>, markers::SignalOptionMarker> for Signal<DataTemplate> {
    fn into_prop(self) -> PropBinding<Option<DataTemplate>> {
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

impl IntoProp<Option<CivilDate>, markers::MemoOptionMarker> for Memo<CivilDate> {
    fn into_prop(self) -> PropBinding<Option<CivilDate>> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || Some(memo.get())))
    }
}

impl IntoProp<Option<CivilTime>, markers::MemoOptionMarker> for Memo<CivilTime> {
    fn into_prop(self) -> PropBinding<Option<CivilTime>> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || Some(memo.get())))
    }
}

impl IntoProp<Option<Decimal>, markers::MemoOptionMarker> for Memo<Decimal> {
    fn into_prop(self) -> PropBinding<Option<Decimal>> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || Some(memo.get())))
    }
}

impl IntoProp<Option<Paint>, markers::MemoOptionMarker> for Memo<Option<Brush>> {
    fn into_prop(self) -> PropBinding<Option<Paint>> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || memo.get().map(Paint::Solid)))
    }
}

impl IntoProp<Option<Paint>, markers::MemoOptionMarker> for Memo<Brush> {
    fn into_prop(self) -> PropBinding<Option<Paint>> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || Some(Paint::Solid(memo.get()))))
    }
}

impl IntoProp<Option<Paint>, markers::MemoOptionMarker> for Memo<Paint> {
    fn into_prop(self) -> PropBinding<Option<Paint>> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || Some(memo.get())))
    }
}

impl IntoProp<Option<Paint>, markers::MemoOptionMarker> for Memo<Color> {
    fn into_prop(self) -> PropBinding<Option<Paint>> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || {
            Some(Paint::Solid(Brush::solid(memo.get())))
        }))
    }
}

impl IntoProp<Option<Command>, markers::MemoOptionMarker> for Memo<Command> {
    fn into_prop(self) -> PropBinding<Option<Command>> {
        let memo = self;
        PropBinding::Reactive(Box::new(move || Some(memo.get())))
    }
}

impl IntoProp<Option<DataTemplate>, markers::MemoOptionMarker> for Memo<DataTemplate> {
    fn into_prop(self) -> PropBinding<Option<DataTemplate>> {
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

impl<T: 'static> IntoProp<T, markers::PathMarker> for PropertyPath<T> {
    fn into_prop(self) -> PropBinding<T> {
        PropBinding::Fallible(Box::new(move || self.get()))
    }
}

impl IntoProp<Option<Paint>, markers::PathMarker> for PropertyPath<Option<Brush>> {
    fn into_prop(self) -> PropBinding<Option<Paint>> {
        PropBinding::Fallible(Box::new(move || Ok(self.get()?.map(Paint::from))))
    }
}

impl IntoProp<Paint, markers::PathMarker> for PropertyPath<Brush> {
    fn into_prop(self) -> PropBinding<Paint> {
        PropBinding::Fallible(Box::new(move || Ok(Paint::from(self.get()?))))
    }
}

impl<T: 'static> IntoProp<T, markers::PathTwoWayMarker> for PathTwoWay<T> {
    fn into_prop(self) -> PropBinding<T> {
        let reader = self.0.read;
        let writer = self.0.write;
        let is_writable = writer.is_some();
        PropBinding::TwoWay(TwoWayAdapter {
            bind_to_control: Box::new(move |setter, updating| {
                if !is_writable {
                    return Err(crate::Error::ReadOnlyBindingPath);
                }
                Effect::new(move || {
                    let value = reader()?;
                    set_unless_updating(&updating, &*setter, value)
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value| {
                writer.as_ref().ok_or(crate::Error::ReadOnlyBindingPath)?(value)
            }),
        })
    }
}

impl<T: Clone + PartialEq + 'static> IntoProp<T, markers::TwoWayMarker> for TwoWay<T> {
    fn into_prop(self) -> PropBinding<T> {
        let signal = self.signal;
        PropBinding::TwoWay(TwoWayAdapter {
            bind_to_control: Box::new(move |setter, updating| {
                Effect::new(move || {
                    let value = signal.get();
                    set_unless_updating(&updating, &*setter, value)
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value| {
                signal.set_if_changed(value);
                Ok(())
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
                    set_unless_updating(&updating, &*setter, Some(value))
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value: Option<bool>| {
                signal.set_if_changed(value.unwrap_or(false));
                Ok(())
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
                    set_unless_updating(&updating, &*setter, value)
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value: String| {
                signal.set_if_changed(Some(value));
                Ok(())
            }),
        })
    }
}

impl IntoProp<Option<CivilDate>, markers::TwoWayOptionMarker> for TwoWay<CivilDate> {
    fn into_prop(self) -> PropBinding<Option<CivilDate>> {
        let signal = self.signal;
        PropBinding::TwoWay(TwoWayAdapter {
            bind_to_control: Box::new(move |setter, updating| {
                Effect::new(move || {
                    let value = signal.get();
                    set_unless_updating(&updating, &*setter, Some(value))
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value: Option<CivilDate>| {
                if let Some(d) = value {
                    signal.set_if_changed(d);
                }
                Ok(())
            }),
        })
    }
}

impl IntoProp<Option<CivilTime>, markers::TwoWayOptionMarker> for TwoWay<CivilTime> {
    fn into_prop(self) -> PropBinding<Option<CivilTime>> {
        let signal = self.signal;
        PropBinding::TwoWay(TwoWayAdapter {
            bind_to_control: Box::new(move |setter, updating| {
                Effect::new(move || {
                    let value = signal.get();
                    set_unless_updating(&updating, &*setter, Some(value))
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value: Option<CivilTime>| {
                if let Some(t) = value {
                    signal.set_if_changed(t);
                }
                Ok(())
            }),
        })
    }
}

impl IntoProp<Option<Decimal>, markers::TwoWayOptionMarker> for TwoWay<Decimal> {
    fn into_prop(self) -> PropBinding<Option<Decimal>> {
        let signal = self.signal;
        PropBinding::TwoWay(TwoWayAdapter {
            bind_to_control: Box::new(move |setter, updating| {
                Effect::new(move || {
                    let value = signal.get();
                    set_unless_updating(&updating, &*setter, Some(value))
                })
                .map(|_| ())
            }),
            set_from_control: Box::new(move |value: Option<Decimal>| {
                if let Some(d) = value {
                    signal.set_if_changed(d);
                }
                Ok(())
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
        PropBinding::Fallible(mut compute) => {
            let control = control.clone();
            Effect::new(move || set(&control, compute()?)).map(|_| ())
        }
        PropBinding::FallibleOptional(mut compute) => {
            let control = control.clone();
            Effect::new(move || match compute()? {
                Some(value) => set(&control, value),
                None => Ok(()),
            })
            .map(|_| ())
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
    get: impl Fn(&C) -> Result<T> + 'static,
    subscribe: impl FnOnce(&C, Box<dyn FnMut() + 'static>) -> Result<EventSubscription>,
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
        PropBinding::Fallible(mut compute) => {
            let control = control.clone();
            Effect::new(move || set(&control, compute()?)).map(|_| ())
        }
        PropBinding::FallibleOptional(mut compute) => {
            let control = control.clone();
            Effect::new(move || match compute()? {
                Some(value) => set(&control, value),
                None => Ok(()),
            })
            .map(|_| ())
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
                    if updating.swap(true, Ordering::AcqRel) {
                        return;
                    }
                    let _guard = ResetUpdating(&updating);
                    match get(&control_for_get) {
                        Ok(value) => {
                            if let Err(error) = set_from_control(value) {
                                crate::runtime::report_ui_error(
                                    "two-way binding update failed",
                                    &error,
                                );
                            }
                        }
                        Err(error) => {
                            crate::runtime::report_ui_error("two-way binding read failed", &error)
                        }
                    }
                }),
            )?;
            scope.retain_subscription(subscription);
            Ok(())
        }
    }
}

pub(crate) fn apply_two_way_prop_dual<C, T>(
    scope: &AppScope,
    control: &C,
    binding: PropBinding<T>,
    set: impl Fn(&C, T) -> Result<()> + 'static,
    get: impl Fn(&C) -> Result<T> + Clone + 'static,
    subscribe_a: impl FnOnce(&C, Box<dyn FnMut() + 'static>) -> Result<EventSubscription>,
    subscribe_b: impl FnOnce(&C, Box<dyn FnMut() + 'static>) -> Result<EventSubscription>,
) -> Result<()>
where
    C: Clone + 'static,
    T: 'static,
{
    match binding {
        PropBinding::TwoWay(adapter) => {
            let updating = Arc::new(AtomicBool::new(false));
            let control_for_set = control.clone();
            (adapter.bind_to_control)(
                Box::new(move |value| set(&control_for_set, value)),
                updating.clone(),
            )?;

            let set_from_control: Rc<dyn Fn(T) -> Result<()>> = Rc::from(adapter.set_from_control);
            fn make_listener<C: 'static, T: 'static, G: Fn(&C) -> Result<T> + 'static>(
                ctrl: C,
                getter: G,
                upd: Arc<AtomicBool>,
                sink: Rc<dyn Fn(T) -> Result<()>>,
            ) -> Box<dyn FnMut() + 'static> {
                Box::new(move || {
                    if upd.swap(true, Ordering::AcqRel) {
                        return;
                    }
                    let _guard = ResetUpdating(&upd);
                    match getter(&ctrl) {
                        Ok(value) => {
                            if let Err(error) = sink(value) {
                                crate::runtime::report_ui_error(
                                    "two-way binding update failed",
                                    &error,
                                );
                            }
                        }
                        Err(error) => {
                            crate::runtime::report_ui_error("two-way binding read failed", &error)
                        }
                    }
                })
            }

            let sub_a = subscribe_a(
                control,
                make_listener(
                    control.clone(),
                    get.clone(),
                    updating.clone(),
                    set_from_control.clone(),
                ),
            )?;
            scope.retain_subscription(sub_a);
            let sub_b = subscribe_b(
                control,
                make_listener(control.clone(), get, updating, set_from_control),
            )?;
            scope.retain_subscription(sub_b);
            Ok(())
        }
        other => apply_prop(control, other, set),
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
    panel_child_slots: Rc<RefCell<Vec<Option<i64>>>>,
}

impl<T: 'static> El<T> {
    pub fn new(create: impl FnOnce() -> Result<T> + 'static) -> Self {
        Self {
            create: Box::new(create),
            ops: Vec::new(),
            panel_child_slots: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn with_op(mut self, op: impl FnOnce(&AppScope, &T) -> Result<()> + 'static) -> Self {
        self.ops.push(Box::new(op));
        self
    }

    pub fn with_panel_child_op(
        mut self,
        op: impl FnOnce(&AppScope, &T, PanelChildSlot) -> Result<()> + 'static,
    ) -> Self {
        let index = {
            let mut slots = self.panel_child_slots.borrow_mut();
            let index = slots.len();
            slots.push(None);
            index
        };
        let slot = PanelChildSlot {
            slots: self.panel_child_slots.clone(),
            index,
        };
        self.ops
            .push(Box::new(move |scope, panel| op(scope, panel, slot)));
        self
    }

    /// Captures a clone of the materialized control into `slot` when built.
    pub fn node_ref(self, slot: NodeRef<T>) -> Self
    where
        T: Clone,
    {
        self.with_op(move |_, control| {
            slot.set_scoped(control.clone());
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

    /// Subscribes to incoming file/folder drags directly on this declarative element.
    pub fn on_file_drop<R: crate::IntoEventResult + 'static>(
        self,
        accepted_effects: crate::DragDropEffects,
        mut callback: impl FnMut(crate::FileDropEvent) -> R + 'static,
    ) -> Self
    where
        T: AsControl,
    {
        self.with_op(move |scope, control| {
            scope.on_file_drop(control, accepted_effects, move |ev| callback(ev))
        })
    }

    /// Materializes the control and applies all deferred properties, children,
    /// and event subscriptions within `scope`.
    pub fn build(self, scope: &AppScope) -> Result<T> {
        crate::runtime::with_view_subscription_scope(|| {
            let instance = (self.create)()?;
            if let Some(window) = (&instance as &dyn std::any::Any).downcast_ref::<Window>() {
                let win_clone = window.clone();
                let mut ops = self.ops;
                let window_scope = ReactiveScope::new();
                let inherited_theme = crate::reactive::use_context::<AmbientThemeSignal>();
                let result = window_scope.run(|| {
                    provide_context(win_clone);
                    let theme = if let Some(ambient) =
                        inherited_theme.as_ref().filter(|ambient| !ambient.1.get())
                    {
                        let signal = Signal::<ThemeVariant>::try_from_raw(ambient.0)?;
                        let app_scope = scope.clone();
                        let signal_for_event = signal;
                        let subscription =
                            window.subscribe_actual_theme_variant_changed(move |_| {
                                if let Ok(variant) = app_scope.actual_theme_variant() {
                                    signal_for_event.set_if_changed(variant);
                                }
                            })?;
                        scope.retain_subscription(subscription);
                        ambient.1.set(true);
                        signal
                    } else {
                        scope.theme_variant_signal(window)?
                    };
                    provide_context(AmbientThemeSignal(
                        theme.raw_id(),
                        Rc::new(std::cell::Cell::new(true)),
                    ));
                    for op in ops.drain(..) {
                        op(scope, &instance)?;
                    }
                    Ok::<(), crate::Error>(())
                });
                if let Err(error) = result {
                    window_scope.dispose();
                    return Err(error);
                }
                return Ok(instance);
            }
            for op in self.ops {
                op(scope, &instance)?;
            }
            Ok(instance)
        })
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
        host.upcast_ref().set_content(&built)?;
        Ok(built)
    }

    /// Materializes this element and sets it as the single child of a [`Decorator`]
    /// (such as [`Border`] or `Viewbox`).
    pub fn mount_into_decorator(self, scope: &AppScope, host: &impl Upcast<Decorator>) -> Result<T>
    where
        T: AsControl,
    {
        let built = self.build(scope)?;
        host.upcast_ref().set_child(&built)?;
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

    /// Mounts `then_view` directly into this [`Panel`]'s `children()` collection
    /// without an intermediate `Border` wrapper when `condition` is `true`, and
    /// unmounts/disposes it when `false` (preserving `Grid.row`, `DockPanel.dock`,
    /// and `WrapPanel` sibling layout).
    pub fn child_when<M, F, E>(
        self,
        condition: impl IntoProp<bool, M> + 'static,
        mut then_view: F,
    ) -> Self
    where
        T: Upcast<Panel> + Clone + 'static,
        F: FnMut() -> E + 'static,
        E: IntoChildEl + 'static,
    {
        let binding = condition.into_prop();
        self.with_panel_child_op(move |scope, panel_owner, slot| {
            let panel: Panel = panel_owner.upcast();
            let scope = scope.clone();
            let mounted: Rc<RefCell<Option<(Control, ReactiveScope)>>> =
                Rc::new(RefCell::new(None));

            let mut update = move |show: bool| -> Result<()> {
                if show {
                    if mounted.borrow().is_some() {
                        return Ok(());
                    }
                    let branch_scope = ReactiveScope::new();
                    let ctrl = match branch_scope.run(|| then_view().into_child_el().build(&scope))
                    {
                        Ok(ctrl) => ctrl,
                        Err(err) => {
                            branch_scope.dispose();
                            return Err(err);
                        }
                    };
                    if let Err(err) = slot.mount(&panel, &ctrl) {
                        branch_scope.dispose();
                        return Err(err);
                    }
                    *mounted.borrow_mut() = Some((ctrl, branch_scope));
                } else if let Some((ctrl, prev_scope)) = mounted.borrow_mut().take() {
                    slot.unmount(&panel, &ctrl)?;
                    prev_scope.dispose();
                }
                Ok(())
            };

            match binding {
                PropBinding::Static(state) => update(state),
                PropBinding::Reactive(mut compute) => {
                    Effect::new(move || update(compute())).map(|_| ())
                }
                PropBinding::Fallible(mut compute) => {
                    Effect::new(move || update(compute()?)).map(|_| ())
                }
                PropBinding::FallibleOptional(mut compute) => Effect::new(move || {
                    if let Some(state) = compute()? {
                        update(state)?;
                    }
                    Ok(())
                })
                .map(|_| ()),
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
                            let previous = (0..list.len()?)
                                .map(|index| list.get(index))
                                .collect::<Result<Vec<_>>>()?;
                            reconcile_collection(
                                &previous,
                                controls,
                                |c| c.object_id(),
                                || list.clear(),
                                |control| list.add(control),
                                |idx, control| list.insert(idx, control),
                                |idx| list.remove(idx),
                            )
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
                PropBinding::Fallible(mut compute) => {
                    Effect::new(move || apply(compute()?, &mut key_fn, &mut view_fn)).map(|_| ())
                }
                PropBinding::FallibleOptional(mut compute) => Effect::new(move || {
                    if let Some(items) = compute()? {
                        apply(items, &mut key_fn, &mut view_fn)?;
                    }
                    Ok(())
                })
                .map(|_| ()),
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
                            let previous = (0..list.len()?)
                                .map(|index| list.get(index))
                                .collect::<Result<Vec<_>>>()?;
                            reconcile_collection(
                                &previous,
                                controls,
                                |c| c.object_id(),
                                || list.clear(),
                                |control| list.add(control),
                                |idx, control| list.insert(idx, control),
                                |idx| list.remove(idx),
                            )
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
                PropBinding::Fallible(mut compute) => {
                    Effect::new(move || apply(compute()?, &mut key_fn, &mut view_fn)).map(|_| ())
                }
                PropBinding::FallibleOptional(mut compute) => Effect::new(move || {
                    if let Some(items) = compute()? {
                        apply(items, &mut key_fn, &mut view_fn)?;
                    }
                    Ok(())
                })
                .map(|_| ()),
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

fn reconcile_collection<V>(
    previous: &[V],
    next: &[V],
    id_of: impl Fn(&V) -> Result<i64>,
    mut clear: impl FnMut() -> Result<()>,
    mut add: impl FnMut(&V) -> Result<()>,
    mut insert: impl FnMut(usize, &V) -> Result<()>,
    mut remove: impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    let apply_diff = |add: &mut dyn FnMut(&V) -> Result<()>,
                      insert: &mut dyn FnMut(usize, &V) -> Result<()>,
                      remove: &mut dyn FnMut(usize) -> Result<()>|
     -> Result<()> {
        let mut working: Vec<i64> = previous.iter().map(&id_of).collect::<Result<Vec<_>>>()?;
        let next_ids: Vec<i64> = next.iter().map(&id_of).collect::<Result<Vec<_>>>()?;

        for idx in (0..working.len()).rev() {
            if !next_ids.contains(&working[idx]) {
                remove(idx)?;
                working.remove(idx);
            }
        }

        for (target_idx, (&want_id, item)) in next_ids.iter().zip(next.iter()).enumerate() {
            if working.get(target_idx) == Some(&want_id) {
                continue;
            }
            if let Some(rel) = working[target_idx..].iter().position(|&id| id == want_id) {
                let from_idx = target_idx + rel;
                remove(from_idx)?;
                working.remove(from_idx);
            }
            if target_idx == working.len() {
                add(item)?;
                working.push(want_id);
            } else {
                insert(target_idx, item)?;
                working.insert(target_idx, want_id);
            }
        }
        Ok(())
    };

    match apply_diff(&mut add, &mut insert, &mut remove) {
        Ok(()) => Ok(()),
        Err(update) => {
            let mut restore = || -> Result<()> {
                clear()?;
                for value in previous {
                    add(value)?;
                }
                Ok(())
            };
            match restore() {
                Ok(()) => Err(update),
                Err(rollback) => Err(crate::Error::CollectionUpdateRollback {
                    update: Box::new(update),
                    rollback: Box::new(rollback),
                }),
            }
        }
    }
}

#[cfg(test)]
fn replace_collection<V>(
    previous: &[V],
    next: &[V],
    mut clear: impl FnMut() -> Result<()>,
    mut add: impl FnMut(&V) -> Result<()>,
) -> Result<()> {
    fn apply<V>(
        values: &[V],
        clear: &mut impl FnMut() -> Result<()>,
        add: &mut impl FnMut(&V) -> Result<()>,
    ) -> Result<()> {
        clear()?;
        for value in values {
            add(value)?;
        }
        Ok(())
    }

    match apply(next, &mut clear, &mut add) {
        Ok(()) => Ok(()),
        Err(update) => match apply(previous, &mut clear, &mut add) {
            Ok(()) => Err(update),
            Err(rollback) => Err(crate::Error::CollectionUpdateRollback {
                update: Box::new(update),
                rollback: Box::new(rollback),
            }),
        },
    }
}

/// Keyed list reconciler that reuses existing realized view handles `V` and
/// updates their per-item [`Signal<T>`] in place when items are added, removed,
/// reordered, or modified.
pub struct KeyedListReconciler<T: 'static, K, V> {
    entries: Vec<(K, V, Signal<T>, ReactiveScope)>,
}

/// Rust references for each live instance of a named item template.
///
/// Entries are removed when their item scope is disposed. Their order reflects
/// creation, not the current visual order; use the item signal to identify an
/// entry. Do not read a saved item signal after that item is removed. This does
/// not create an Avalonia `NameScope`.
pub struct ItemNameScope<T: 'static, R: Copy + 'static> {
    entries: Rc<RefCell<Vec<(Signal<T>, R)>>>,
}

impl<T: 'static, R: Copy + 'static> Clone for ItemNameScope<T, R> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
        }
    }
}

impl<T: 'static, R: Copy + 'static> std::fmt::Debug for ItemNameScope<T, R> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ItemNameScope")
            .field("len", &self.entries.borrow().len())
            .finish()
    }
}

impl<T: 'static, R: Copy + 'static> Default for ItemNameScope<T, R> {
    fn default() -> Self {
        Self {
            entries: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl<T: 'static, R: Copy + 'static> ItemNameScope<T, R> {
    /// Returns the currently mounted item signals and their named references.
    pub fn entries(&self) -> Vec<(Signal<T>, R)> {
        self.entries.borrow().clone()
    }

    /// Registers one item instance for the lifetime of the current scope.
    #[doc(hidden)]
    pub fn register(&self, item: Signal<T>, refs: R) {
        let mut entries = self.entries.borrow_mut();
        assert!(
            !entries.iter().any(|(registered, _)| *registered == item),
            "item names were already registered for this signal"
        );
        entries.push((item, refs));
        drop(entries);
        let entries = self.entries.clone();
        on_scope_cleanup(move || {
            entries
                .borrow_mut()
                .retain(|(registered, _)| *registered != item)
        });
    }
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
    /// item's [`ReactiveScope`] is automatically disposed. Duplicate keys are
    /// rejected, and a failed build or `apply_list` leaves the reconciler's
    /// previous entries intact. `apply_list` is responsible for rolling back
    /// changes to its own collection if it fails.
    pub fn reconcile(
        &mut self,
        next_items: Vec<T>,
        key_fn: &mut impl FnMut(&T) -> K,
        build_view: &mut impl FnMut(Signal<T>) -> Result<V>,
        apply_list: impl FnOnce(&[V]) -> Result<()>,
    ) -> Result<bool> {
        let old_keys: Vec<K> = self.entries.iter().map(|(k, _, _, _)| k.clone()).collect();
        let mut desired = Vec::with_capacity(next_items.len());
        for item in next_items {
            let key = key_fn(&item);
            if desired.iter().any(|(existing, _)| *existing == key) {
                return Err(crate::Error::DuplicateItemKey);
            }
            desired.push((key, item));
        }

        let mut next_entries = Vec::with_capacity(desired.len());
        let mut next_keys = Vec::with_capacity(desired.len());
        let mut updates = Vec::new();
        let mut created_scopes = Vec::new();

        for (key, item) in desired {
            next_keys.push(key.clone());
            if let Some((existing_key, view, signal, item_scope)) = self
                .entries
                .iter()
                .find(|(existing, _, _, _)| *existing == key)
            {
                updates.push((*signal, item));
                next_entries.push((existing_key.clone(), view.clone(), *signal, *item_scope));
            } else {
                let item_scope = ReactiveScope::new();
                created_scopes.push(item_scope);
                let (view, signal) = match item_scope.run(|| {
                    let signal = Signal::new_scoped(item);
                    build_view(signal).map(|view| (view, signal))
                }) {
                    Ok(built) => built,
                    Err(error) => {
                        for scope in created_scopes {
                            scope.dispose();
                        }
                        return Err(error);
                    }
                };
                next_entries.push((key, view, signal, item_scope));
            }
        }

        let keys_changed = old_keys != next_keys;
        if keys_changed {
            let views: Vec<V> = next_entries.iter().map(|(_, v, _, _)| v.clone()).collect();
            if let Err(error) = apply_list(&views) {
                for scope in created_scopes {
                    scope.dispose();
                }
                return Err(error);
            }
        }
        for (key, _, _, scope) in &self.entries {
            if !next_keys.contains(key) {
                scope.dispose();
            }
        }
        self.entries = next_entries;
        for (signal, item) in updates {
            signal.set_if_changed(item);
        }
        Ok(keys_changed)
    }
}

#[cfg(test)]
mod collection_tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn two_way_update_guard_prevents_reentry_and_resets_after_errors() {
        let updating = AtomicBool::new(false);
        let calls = Cell::new(0);
        set_unless_updating(
            &updating,
            &|()| {
                calls.set(calls.get() + 1);
                set_unless_updating(
                    &updating,
                    &|()| {
                        calls.set(calls.get() + 1);
                        Ok(())
                    },
                    (),
                )
            },
            (),
        )
        .unwrap();
        assert_eq!(calls.get(), 1);
        assert!(!updating.load(Ordering::Acquire));

        assert!(set_unless_updating(
            &updating,
            &|()| Err(crate::Error::Binding("rejected".into())),
            ()
        )
        .is_err());
        assert!(!updating.load(Ordering::Acquire));

        assert!(std::panic::catch_unwind(|| {
            let _ = set_unless_updating(&updating, &|()| panic!("setter panicked"), ());
        })
        .is_err());
        assert!(!updating.load(Ordering::Acquire));
    }

    #[test]
    fn two_way_paths_surface_read_only_and_validation_errors() {
        let read_only = PropertyPath::read_only(|| Ok(String::from("read only")));
        let binding: PropBinding<String> = read_only.two_way().into_prop();
        let PropBinding::TwoWay(read_only) = binding else {
            panic!("expected two-way binding");
        };
        assert!(matches!(
            (read_only.bind_to_control)(Box::new(|_| Ok(())), Arc::new(AtomicBool::new(false))),
            Err(crate::Error::ReadOnlyBindingPath)
        ));

        let value = Signal::new(String::from("initial"));
        let path = PropertyPath::writable(
            move || Ok(value.get()),
            move |next: String| {
                if next.is_empty() {
                    return Err(crate::Error::Binding("value is required".into()));
                }
                value.set_if_changed(next);
                Ok(())
            },
        );
        let scope = ReactiveScope::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        scope.run(|| {
            let binding: PropBinding<String> = path.two_way().into_prop();
            let PropBinding::TwoWay(adapter) = binding else {
                panic!("expected two-way binding");
            };
            let seen = seen.clone();
            (adapter.bind_to_control)(
                Box::new(move |next| {
                    seen.borrow_mut().push(next);
                    Ok(())
                }),
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap();
            assert!((adapter.set_from_control)(String::new()).is_err());
            (adapter.set_from_control)("accepted".into()).unwrap();
        });
        assert_eq!(*seen.borrow(), ["initial", "accepted"]);
        scope.dispose();
    }

    #[test]
    fn named_item_references_are_isolated_and_removed_with_each_scope() {
        let names = ItemNameScope::<u32, NodeRef<String>>::default();
        let first = ReactiveScope::new();
        let first_ref = first.run(|| {
            let item = Signal::new_scoped(1);
            let node = NodeRef::new_scoped();
            node.set("first".to_string());
            names.register(item, node);
            node
        });
        let second = ReactiveScope::new();
        let second_ref = second.run(|| {
            let item = Signal::new_scoped(2);
            let node = NodeRef::new_scoped();
            node.set("second".to_string());
            names.register(item, node);
            node
        });

        assert_eq!(names.entries().len(), 2);
        assert_eq!(first_ref.get().as_deref(), Some("first"));
        assert_eq!(second_ref.get().as_deref(), Some("second"));

        first.dispose();
        assert_eq!(names.entries().len(), 1);
        assert_eq!(names.entries()[0].0.get(), 2);
        assert_eq!(first_ref.get(), None);
        assert_eq!(second_ref.get().as_deref(), Some("second"));

        second.dispose();
        assert!(names.entries().is_empty());
        assert_eq!(second_ref.get(), None);
    }

    #[test]
    fn failed_list_update_removes_names_for_rolled_back_items() {
        let names = ItemNameScope::<u32, NodeRef<String>>::default();
        let parent = ReactiveScope::new();
        parent.run(|| {
            let mut reconciler = KeyedListReconciler::<u32, u32, ()>::new();
            let mut key = |item: &u32| *item;
            let mut build = |item: Signal<u32>| {
                names.register(item, NodeRef::new_scoped());
                Ok(())
            };
            reconciler
                .reconcile(vec![1], &mut key, &mut build, |_| Ok(()))
                .expect("first item should mount");
            let result = reconciler.reconcile(vec![1, 2], &mut key, &mut build, |_| {
                Err(crate::Error::Load("update failed".into()))
            });
            assert!(result.is_err());
            assert_eq!(names.entries().len(), 1);
            assert_eq!(names.entries()[0].0.get(), 1);
        });
        parent.dispose();
        assert!(names.entries().is_empty());
    }

    #[test]
    fn failed_collection_update_restores_the_original_entries() {
        let values = RefCell::new(vec![1, 2]);
        let fail_once = Cell::new(true);
        let result = replace_collection(
            &[1, 2],
            &[2, 3],
            || {
                values.borrow_mut().clear();
                Ok(())
            },
            |value| {
                if *value == 3 && fail_once.replace(false) {
                    return Err(crate::Error::Load("update failed".into()));
                }
                values.borrow_mut().push(*value);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(*values.borrow(), vec![1, 2]);
    }

    #[test]
    fn collection_update_reports_a_failed_rollback() {
        let values = RefCell::new(vec![1, 2]);
        let calls = Cell::new(0);
        let result = replace_collection(
            &[1, 2],
            &[2, 3],
            || {
                values.borrow_mut().clear();
                Ok(())
            },
            |value| {
                let call = calls.get() + 1;
                calls.set(call);
                if call == 2 || call == 3 {
                    return Err(crate::Error::Load(format!("add {call} failed")));
                }
                values.borrow_mut().push(*value);
                Ok(())
            },
        );
        let error = result.expect_err("the update and rollback both fail");
        assert!(matches!(
            error,
            crate::Error::CollectionUpdateRollback { .. }
        ));
        assert!(error.to_string().contains("add 2 failed"));
        assert!(error.to_string().contains("add 3 failed"));
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
                border.set_child(built)
            };

            match binding {
                PropBinding::Static(state) => update(state),
                PropBinding::Reactive(mut compute) => {
                    Effect::new(move || update(compute())).map(|_| ())
                }
                PropBinding::Fallible(mut compute) => {
                    Effect::new(move || update(compute()?)).map(|_| ())
                }
                PropBinding::FallibleOptional(mut compute) => Effect::new(move || {
                    if let Some(state) = compute()? {
                        update(state)?;
                    }
                    Ok(())
                })
                .map(|_| ()),
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

    /// Renders a reactive keyed list inside a caller-chosen [`Panel`] (for example
    /// `WrapPanel::el()`, `UniformGrid::el()`, or a horizontal `StackPanel`).
    pub fn in_panel<P, Item, K, M, KF, VF, E>(
        panel: El<P>,
        items: impl IntoProp<Vec<Item>, M> + 'static,
        key_fn: KF,
        view_fn: VF,
    ) -> El<P>
    where
        P: Upcast<Panel> + Clone + 'static,
        Item: Clone + PartialEq + 'static,
        K: Eq + Clone + 'static,
        KF: FnMut(&Item) -> K + 'static,
        VF: FnMut(Signal<Item>) -> E + 'static,
        E: IntoChildEl + 'static,
    {
        panel.children_for(items, key_fn, view_fn)
    }
}

/// A `Copy` slot for dynamically mounting, replacing, or clearing a declarative
/// [`El`] / `rsx!` subtree inside an existing view hierarchy at runtime.
#[derive(Clone, Copy, Debug, Default)]
pub struct ControlSlot {
    host: NodeRef<Border>,
    owner_scope: NodeRef<ReactiveScope>,
    active_scope: NodeRef<ReactiveScope>,
}

impl ControlSlot {
    pub fn new() -> Self {
        Self {
            host: NodeRef::new(),
            owner_scope: NodeRef::new(),
            active_scope: NodeRef::new(),
        }
    }

    /// Returns the host [`El<Border>`] to place inside a parent `rsx!` or `El` tree.
    pub fn host(&self) -> El<Border> {
        let slot = *self;
        Border::el()
            .with_op(move |_, _| {
                if slot.host.get().is_some() {
                    return Err(crate::Error::ControlSlotAlreadyMounted);
                }
                slot.owner_scope.set_scoped(ReactiveScope::current());
                on_scope_cleanup(move || {
                    if let Some(active) = slot.active_scope.get() {
                        active.dispose();
                        slot.active_scope.clear();
                    }
                });
                Ok(())
            })
            .node_ref(self.host)
    }

    /// Returns the underlying host [`Border`] if it has been materialized.
    pub fn host_control(&self) -> Option<Border> {
        self.host.get()
    }

    /// Materializes `view` inside a new [`ReactiveScope`] and mounts it into this
    /// slot's host container, disposing any previous subtree's scope.
    pub fn mount(&self, scope: &AppScope, view: impl IntoContentEl) -> Result<()> {
        let border = self.host.get().ok_or(crate::Error::ControlSlotNotMounted)?;
        let owner = self
            .owner_scope
            .get()
            .ok_or(crate::Error::ControlSlotNotMounted)?;
        let next_scope = owner.run(ReactiveScope::new);
        let built =
            match next_scope.run(|| view.into_content_el().map(|el| el.build(scope)).transpose()) {
                Ok(built) => built,
                Err(error) => {
                    next_scope.dispose();
                    return Err(error);
                }
            };
        if let Err(error) = border.set_child(built) {
            next_scope.dispose();
            return Err(error);
        }
        if let Some(prev) = self.active_scope.get() {
            prev.dispose();
        }
        self.active_scope.set(next_scope);
        Ok(())
    }

    /// Clears any currently mounted child and disposes its [`ReactiveScope`].
    pub fn clear(&self) -> Result<()> {
        let border = self.host.get().ok_or(crate::Error::ControlSlotNotMounted)?;
        border.set_child(None)?;
        if let Some(prev) = self.active_scope.get() {
            prev.dispose();
        }
        self.active_scope.clear();
        Ok(())
    }

    /// Reactively selects and mounts a view for an optional Rust model.
    ///
    /// A model update with the same registry key updates the existing view's
    /// signal. A different key replaces the view and disposes its old scope.
    pub fn bind<T, K>(
        &self,
        scope: &AppScope,
        model: Signal<Option<T>>,
        registry: ViewRegistry<T, K>,
    ) -> Result<Effect>
    where
        T: Clone + PartialEq + 'static,
        K: Clone + Eq + 'static,
    {
        let owner = self
            .owner_scope
            .get()
            .filter(|_| self.host.get().is_some())
            .ok_or(crate::Error::ControlSlotNotMounted)?;
        let slot = *self;
        let scope = scope.clone();
        let mut current: Option<(K, Signal<T>, ReactiveScope)> = None;
        let mut initialized = false;
        owner.run(|| {
            Effect::new(move || {
                match model.get() {
                    Some(value) => {
                        let key = registry.key_for(&value);
                        if let Some((current_key, signal, mounted_scope)) = &current {
                            if *current_key == key
                                && slot.active_scope.get() == Some(*mounted_scope)
                            {
                                signal.set_if_changed(value);
                                return Ok(());
                            }
                        }
                        let signal = Signal::new(value);
                        if let Err(error) = slot.mount(&scope, registry.deferred(signal)) {
                            signal.dispose_value();
                            return Err(error);
                        }
                        let mounted_scope = slot
                            .active_scope
                            .get()
                            .expect("a successful mount stores its scope");
                        current = Some((key, signal, mounted_scope));
                    }
                    None if !initialized || current.is_some() => {
                        slot.clear()?;
                        current = None;
                    }
                    None => {}
                }
                initialized = true;
                Ok(())
            })
        })
    }
}

type ViewBuilder<T> = Rc<dyn Fn(Signal<T>) -> AnyEl>;

/// Selects Rust views for model variants without requiring a managed ViewLocator.
pub struct ViewRegistry<T: 'static, K> {
    selector: Rc<dyn Fn(&T) -> K>,
    builders: Rc<RefCell<Vec<(K, ViewBuilder<T>)>>>,
}

impl<T: 'static, K> Clone for ViewRegistry<T, K> {
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            builders: self.builders.clone(),
        }
    }
}

impl<T: 'static, K> std::fmt::Debug for ViewRegistry<T, K> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ViewRegistry")
            .finish_non_exhaustive()
    }
}

impl<T: Clone + 'static, K: Clone + Eq + 'static> ViewRegistry<T, K> {
    /// Creates a registry whose selector identifies the view kind for a model.
    pub fn new(selector: impl Fn(&T) -> K + 'static) -> Self {
        Self {
            selector: Rc::new(selector),
            builders: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Registers one view factory for `key`; duplicate registrations are rejected.
    pub fn register<E: IntoChildEl + 'static>(
        &self,
        key: K,
        factory: impl Fn(Signal<T>) -> E + 'static,
    ) -> Result<()> {
        let mut builders = self.builders.borrow_mut();
        if builders.iter().any(|(existing, _)| *existing == key) {
            return Err(crate::Error::InvalidViewRegistry(
                "a view factory is already registered for this key".to_string(),
            ));
        }
        builders.push((key, Rc::new(move |item| factory(item).into_child_el())));
        Ok(())
    }

    /// Returns the selector key for a model.
    pub fn key_for(&self, model: &T) -> K {
        (self.selector)(model)
    }

    /// Creates the selected view, reporting an error for an unregistered key.
    pub fn create(&self, model: Signal<T>) -> Result<AnyEl> {
        let key = self.key_for(&model.get_untracked());
        let factory = self
            .builders
            .borrow()
            .iter()
            .find(|(registered, _)| *registered == key)
            .map(|(_, factory)| factory.clone())
            .ok_or_else(|| {
                crate::Error::InvalidViewRegistry(format!(
                    "no view factory is registered for model {}",
                    std::any::type_name::<T>()
                ))
            })?;
        Ok(factory(model))
    }

    fn deferred(&self, model: Signal<T>) -> AnyEl {
        let registry = self.clone();
        AnyEl {
            build: Box::new(move |scope| {
                on_scope_cleanup(move || model.dispose_value());
                registry.create(model)?.build(scope)
            }),
        }
    }
}

/// A type-erased, reactive item source for an AXAML `ItemsControl`.
#[derive(Clone)]
pub struct ItemsView {
    attach: Rc<dyn Fn(&AppScope, &ItemsControl) -> Result<()>>,
}

impl std::fmt::Debug for ItemsView {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ItemsView").finish_non_exhaustive()
    }
}

impl ItemsView {
    /// Creates a keyed item source with an explicit Rust view factory.
    pub fn new<Item, K, KF, VF, E>(items: Signal<Vec<Item>>, key_fn: KF, view_fn: VF) -> Self
    where
        Item: Clone + PartialEq + 'static,
        K: Eq + Clone + 'static,
        KF: FnMut(&Item) -> K + 'static,
        VF: FnMut(Signal<Item>) -> E + 'static,
        E: IntoChildEl + 'static,
    {
        let key_fn = Rc::new(RefCell::new(key_fn));
        let view_fn = Rc::new(RefCell::new(view_fn));
        Self {
            attach: Rc::new(move |scope, owner| {
                let owner = owner.clone();
                let key_fn = key_fn.clone();
                let view_fn = view_fn.clone();
                El::new(move || Ok(owner))
                    .items_for(
                        items,
                        move |item| (key_fn.borrow_mut())(item),
                        move |item| (view_fn.borrow_mut())(item),
                    )
                    .build(scope)?;
                Ok(())
            }),
        }
    }

    /// Creates a keyed item source using a Rust view registry.
    pub fn from_registry<Item, K, Kind>(
        items: Signal<Vec<Item>>,
        key_fn: impl FnMut(&Item) -> K + 'static,
        registry: ViewRegistry<Item, Kind>,
    ) -> Self
    where
        Item: Clone + PartialEq + 'static,
        K: Eq + Clone + 'static,
        Kind: Eq + Clone + 'static,
    {
        Self::new(items, key_fn, move |item| {
            let slot = ControlSlot::new();
            let registry = registry.clone();
            slot.host().with_op(move |scope, _| {
                let model = Memo::new_scoped(move || Some(item.get()));
                slot.bind(scope, model.as_signal(), registry)?;
                Ok(())
            })
        })
    }

    /// Attaches this source to an already materialized items control.
    pub fn attach(&self, scope: &AppScope, owner: &impl Upcast<ItemsControl>) -> Result<()> {
        (self.attach)(scope, owner.upcast_ref())
    }
}

/// A safe, reactive wrapper around `IAvnCommand` that supports automatic
/// `CanExecuteChanged` notifications driven by [`Signal<bool>`], [`Memo<bool>`],
/// or reactive closures.
#[derive(Clone, Debug)]
pub struct Command {
    pub(crate) ptr: sys::ComPtr<sys::IAvnCommand>,
    owned: Option<sys::Command>,
    can_execute_flag: Arc<AtomicBool>,
    _effect: Option<Effect>,
}

fn command_callback_error(error: crate::Error) -> sys::Error {
    match error {
        crate::Error::Abi(error) => error,
        error => {
            crate::runtime::report_ui_error("command callback failed", &error);
            sys::Error(sys::E_FAIL)
        }
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.ptr.as_raw(), other.ptr.as_raw())
    }
}
impl Eq for Command {}

impl Command {
    /// Creates an always-enabled command that invokes `execute` on the UI thread
    /// with the command parameter.
    pub fn new(execute: impl FnMut(Variant) -> Result<()> + 'static) -> Self {
        let can_execute_flag = Arc::new(AtomicBool::new(true));
        let flag_for_query = can_execute_flag.clone();
        let exec_cb = UiCallbackHandle::new(execute);
        let raw = sys::command(
            move |variant| {
                let variant = unsafe { Variant::from_borrowed_abi(variant)? };
                exec_cb
                    .with(|cb| cb(variant))
                    .map_err(command_callback_error)?
                    .map_err(command_callback_error)
            },
            move |_| Ok(flag_for_query.load(Ordering::Acquire)),
        );
        let ptr = raw.as_com_ptr().clone();
        Self {
            ptr,
            owned: Some(raw),
            can_execute_flag,
            _effect: None,
        }
    }

    /// Creates a command whose `CanExecute` state is reactively driven by `can_execute`
    /// (a `bool`, [`Signal<bool>`], [`Memo<bool>`], or closure), automatically firing
    /// `CanExecuteChanged` when the state changes.
    pub fn with_can_execute<M>(
        can_execute: impl IntoProp<bool, M> + 'static,
        execute: impl FnMut(Variant) -> Result<()> + 'static,
    ) -> Result<Self> {
        Self::with_can_execute_parameter(can_execute, |_| Ok(true), execute)
    }

    /// Creates a command with a reactive global gate and an additional
    /// parameter-specific `CanExecute` predicate. Call [`Command::notify`]
    /// when predicate dependencies other than the global gate change.
    pub fn with_can_execute_parameter<M>(
        can_execute: impl IntoProp<bool, M> + 'static,
        predicate: impl FnMut(Variant) -> Result<bool> + 'static,
        execute: impl FnMut(Variant) -> Result<()> + 'static,
    ) -> Result<Self> {
        let can_execute_flag = Arc::new(AtomicBool::new(true));
        let flag_for_query = can_execute_flag.clone();
        let exec_cb = UiCallbackHandle::new(execute);
        let pred_cb = UiCallbackHandle::new(predicate);
        let raw = sys::command(
            move |variant| {
                let variant = unsafe { Variant::from_borrowed_abi(variant)? };
                exec_cb
                    .with(|cb| cb(variant))
                    .map_err(command_callback_error)?
                    .map_err(command_callback_error)
            },
            move |variant| {
                if !flag_for_query.load(Ordering::Acquire) {
                    return Ok(false);
                }
                let variant = unsafe { Variant::from_borrowed_abi(variant)? };
                pred_cb
                    .with(|cb| cb(variant))
                    .map_err(command_callback_error)?
                    .map_err(command_callback_error)
            },
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
                        raw_for_effect.try_notify()?;
                    }
                    Ok(())
                })?)
            }
            PropBinding::Fallible(mut compute) => {
                let flag_for_effect = can_execute_flag.clone();
                let raw_for_effect = raw.clone();
                Some(Effect::new(move || {
                    let next = compute()?;
                    let prev = flag_for_effect.swap(next, Ordering::AcqRel);
                    if prev != next {
                        raw_for_effect.try_notify()?;
                    }
                    Ok(())
                })?)
            }
            PropBinding::FallibleOptional(mut compute) => {
                let flag_for_effect = can_execute_flag.clone();
                let raw_for_effect = raw.clone();
                Some(Effect::new(move || {
                    if let Some(next) = compute()? {
                        let prev = flag_for_effect.swap(next, Ordering::AcqRel);
                        if prev != next {
                            raw_for_effect.try_notify()?;
                        }
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
                            raw_for_effect.try_notify()?;
                        }
                        Ok(())
                    }),
                    updating,
                )?;
                None
            }
        };

        let ptr = raw.as_com_ptr().clone();
        Ok(Self {
            ptr,
            owned: Some(raw),
            can_execute_flag,
            _effect: effect,
        })
    }

    /// Returns the reactive global gate. A parameter-specific predicate may
    /// further restrict execution; use [`Command::can_execute`] to query it.
    pub fn is_enabled(&self) -> bool {
        self.can_execute_flag.load(Ordering::Acquire)
    }

    /// Manually fires `CanExecuteChanged` notifications to all subscribed controls.
    ///
    /// Notification failures are written to stderr. Use [`Command::try_notify`]
    /// to handle them directly.
    pub fn notify(&self) {
        if let Some(owned) = &self.owned {
            owned.notify();
        }
    }

    /// Manually fires `CanExecuteChanged` notifications and returns the first
    /// notification failure after attempting all subscribed controls.
    pub fn try_notify(&self) -> Result<()> {
        if let Some(owned) = &self.owned {
            owned.try_notify()?;
        }
        Ok(())
    }

    /// Invokes the command directly from Rust with `parameter`.
    pub fn execute(&self, parameter: impl Into<Variant>) -> Result<()> {
        Self::execute_interface(&self.ptr, parameter)
    }

    /// Invokes a command interface obtained from a control property.
    pub fn execute_interface(
        command: &sys::ComPtr<sys::IAvnCommand>,
        parameter: impl Into<Variant>,
    ) -> Result<()> {
        let abi = parameter.into().to_abi()?.into_owned_abi();
        Ok(command.execute(abi)?)
    }

    /// Queries the command for a particular parameter.
    pub fn can_execute(&self, parameter: impl Into<Variant>) -> Result<bool> {
        Self::can_execute_interface(&self.ptr, parameter)
    }

    /// Creates an asynchronous UI command whose `CanExecute` state automatically
    /// disables while an invocation is in flight and re-enables on completion.
    pub fn new_async<Fut, R>(
        scope: &AppScope,
        mut execute: impl FnMut(Variant) -> Fut + 'static,
    ) -> Result<Self>
    where
        Fut: Future<Output = R> + 'static,
        R: crate::IntoEventResult + 'static,
    {
        let idle = Signal::new(true);
        let scope = scope.clone();
        Self::with_can_execute(idle, move |param| {
            if !idle.get_untracked() {
                return Ok(());
            }
            idle.set(false);
            struct IdleReset(crate::reactive::RawSignalId);
            impl Drop for IdleReset {
                fn drop(&mut self) {
                    if let Ok(sig) = Signal::<bool>::try_from_raw(self.0) {
                        if sig.is_alive() {
                            sig.set(true);
                        }
                    }
                }
            }
            let reset = IdleReset(idle.raw_id());
            let fut = execute(param);
            scope.spawn_local(async move {
                let _guard = reset;
                fut.await
            })
        })
    }

    /// Queries a command interface obtained from a control property.
    pub fn can_execute_interface(
        command: &sys::ComPtr<sys::IAvnCommand>,
        parameter: impl Into<Variant>,
    ) -> Result<bool> {
        let abi = parameter.into().to_abi()?.into_owned_abi();
        Ok(command.can_execute(abi)?)
    }

    pub fn as_com_ptr(&self) -> &sys::ComPtr<sys::IAvnCommand> {
        &self.ptr
    }

    pub(crate) fn from_com_ptr(ptr: sys::ComPtr<sys::IAvnCommand>) -> Self {
        Self {
            ptr,
            owned: None,
            can_execute_flag: Arc::new(AtomicBool::new(true)),
            _effect: None,
        }
    }
}

/// Polymorphic conversion into an optional `IAvnCommand` pointer for control setters.
pub trait IntoCommandProp {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>>;
}

impl IntoCommandProp for Command {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>> {
        Some(self.ptr)
    }
}

impl IntoCommandProp for &Command {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>> {
        Some(self.ptr.clone())
    }
}

impl IntoCommandProp for Option<Command> {
    fn into_command_prop(self) -> Option<sys::ComPtr<sys::IAvnCommand>> {
        self.map(|cmd| cmd.ptr)
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

    /// Creates a deferred [`AnyEl`] whose builder closure runs when mounted into
    /// an [`AppScope`] (preserving enclosing reactive scope & context).
    pub fn from_deferred(build: impl FnOnce(&AppScope) -> Result<Control> + 'static) -> Self {
        Self {
            build: Box::new(build),
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
    mut matches: impl FnMut(&Variant) -> bool + 'static,
    mut build_view: impl FnMut(Signal<Variant>) -> E + 'static,
) -> Result<DataTemplate>
where
    E: Into<AnyEl>,
{
    let scope = scope.clone();
    let last_item: Rc<RefCell<Variant>> = Rc::new(RefCell::new(Variant::None));
    let last_item_for_match = last_item.clone();
    let pool: Rc<RefCell<Vec<(Control, Signal<Variant>)>>> = Rc::new(RefCell::new(Vec::new()));

    crate::data_template(
        move |variant| {
            let matched = matches(&variant);
            if matched {
                *last_item_for_match.borrow_mut() = variant;
            }
            matched
        },
        move || {
            let item = last_item.borrow().clone();

            if let Some((control, signal)) = pool.borrow_mut().pop() {
                signal.set(item);
                return Ok(control);
            }

            let signal = Signal::new(item);
            let control = build_view(signal).into().build(&scope)?;
            let pool_for_detach = pool.clone();
            let control_for_detach = control.clone();
            let subscription = control.subscribe_detached_from_logical_tree(move |_| {
                pool_for_detach
                    .borrow_mut()
                    .push((control_for_detach.clone(), signal));
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

    /// Creates an asynchronous reactive [`Resource<T>`] that runs `fetcher`
    /// whenever the value returned by `source` changes.
    ///
    /// When the `tokio` Cargo feature is enabled and a Tokio runtime is active,
    /// `fetcher` is spawned onto that Tokio runtime; otherwise it is polled via
    /// [`AppScope::spawn_local`]. For CPU-bound or synchronous I/O work without
    /// Tokio, prefer [`AppScope::create_blocking_resource`].
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
        let raw_state = resource.state_raw_id();
        let raw_gen = resource.generation_raw_id();
        let scope = self.clone();
        let last_source: Rc<RefCell<Option<S>>> = Rc::new(RefCell::new(None));

        Effect::new(move || {
            let next_source = source();
            if last_source.borrow().as_ref() == Some(&next_source) {
                return Ok(());
            }
            *last_source.borrow_mut() = Some(next_source.clone());

            let gen = Signal::<u64>::try_from_raw(raw_gen)?
                .get_untracked()
                .wrapping_add(1);
            Signal::<u64>::try_from_raw(raw_gen)?.set(gen);
            Signal::<ResourceState<T>>::try_from_raw(raw_state)?.set(ResourceState::Loading);

            let fut = fetcher(next_source);
            let apply_outcome = move |outcome: std::result::Result<T, String>| {
                if let (Ok(gen_sig), Ok(state_sig)) = (
                    Signal::<u64>::try_from_raw(raw_gen),
                    Signal::<ResourceState<T>>::try_from_raw(raw_state),
                ) {
                    if gen_sig.is_alive() && state_sig.is_alive() && gen_sig.get_untracked() == gen
                    {
                        match outcome {
                            Ok(value) => state_sig.set(ResourceState::Ready(value)),
                            Err(error) => state_sig.set(ResourceState::Error(error)),
                        }
                    }
                }
            };

            #[cfg(feature = "tokio")]
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let scope_for_post = scope.clone();
                handle.spawn(async move {
                    let outcome = fut.await;
                    let _ = scope_for_post.post(move || apply_outcome(outcome));
                });
                return Ok(());
            }

            scope.spawn_local(async move {
                let outcome = fut.await;
                apply_outcome(outcome);
            })
        })?;

        Ok(resource)
    }

    /// Creates a reactive [`Resource<T>`] whose `fetcher` runs on a background
    /// OS thread via [`AppScope::run_blocking`] whenever `source` changes.
    pub fn create_blocking_resource<S, T>(
        &self,
        mut source: impl FnMut() -> S + 'static,
        fetcher: impl Fn(S) -> Result<T> + Send + Sync + 'static,
    ) -> Result<Resource<T>>
    where
        S: PartialEq + Clone + Send + 'static,
        T: Clone + Send + 'static,
    {
        let fetcher = Arc::new(fetcher);
        let resource = Resource::<T>::pending();
        let raw_state = resource.state_raw_id();
        let raw_gen = resource.generation_raw_id();
        let scope = self.clone();
        let last_source: Rc<RefCell<Option<S>>> = Rc::new(RefCell::new(None));

        Effect::new(move || {
            let next_source = source();
            if last_source.borrow().as_ref() == Some(&next_source) {
                return Ok(());
            }
            *last_source.borrow_mut() = Some(next_source.clone());

            let gen = Signal::<u64>::try_from_raw(raw_gen)?
                .get_untracked()
                .wrapping_add(1);
            Signal::<u64>::try_from_raw(raw_gen)?.set(gen);
            Signal::<ResourceState<T>>::try_from_raw(raw_state)?.set(ResourceState::Loading);

            let f = fetcher.clone();
            let op = scope.run_blocking(move || f(next_source));
            scope.spawn_local(async move {
                let outcome = op.await;
                if let (Ok(gen_sig), Ok(state_sig)) = (
                    Signal::<u64>::try_from_raw(raw_gen),
                    Signal::<ResourceState<T>>::try_from_raw(raw_state),
                ) {
                    if gen_sig.is_alive() && state_sig.is_alive() && gen_sig.get_untracked() == gen
                    {
                        match outcome {
                            Ok(value) => state_sig.set(ResourceState::Ready(value)),
                            Err(error) => state_sig.set(ResourceState::Error(error.to_string())),
                        }
                    }
                }
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

/// Resolves the enclosing [`Window`] from reactive scope context, falling back
/// to [`AppScope::main_window`].
pub fn use_window(scope: &AppScope) -> Result<Window> {
    scope.resolve_window()
}

#[derive(Clone)]
struct AmbientThemeSignal(crate::reactive::RawSignalId, Rc<std::cell::Cell<bool>>);

/// Returns a reactive [`Signal<ThemeVariant>`] tracking the active theme of the
/// enclosing [`Window`] (or application theme when no window is mounted yet),
/// without requiring callers to thread `&Window` manually.
pub fn use_theme(scope: &AppScope) -> Result<Signal<ThemeVariant>> {
    if let Some(cached) = crate::reactive::use_context::<AmbientThemeSignal>() {
        if let Ok(sig) = Signal::<ThemeVariant>::try_from_raw(cached.0) {
            if sig.is_alive() {
                return Ok(sig);
            }
        }
    }
    let (sig, attached) = if let Some(window) = crate::reactive::use_context::<Window>() {
        (scope.theme_variant_signal(&window)?, true)
    } else if let Ok(window) = scope.resolve_window() {
        (scope.theme_variant_signal(&window)?, true)
    } else {
        (Signal::new(scope.actual_theme_variant()?), false)
    };
    provide_context(AmbientThemeSignal(
        sig.raw_id(),
        Rc::new(std::cell::Cell::new(attached)),
    ));
    Ok(sig)
}

#[derive(Clone)]
pub struct PanelChildSlot {
    slots: Rc<RefCell<Vec<Option<i64>>>>,
    index: usize,
}

impl PanelChildSlot {
    pub fn mount(&self, panel_owner: &impl Upcast<Panel>, child: &impl AsControl) -> Result<()> {
        let panel = panel_owner.upcast_ref();
        let child_id = child.as_control()?.object_id()?;
        let mut slots = self.slots.borrow_mut();
        if slots[self.index] == Some(child_id) {
            return Ok(());
        }
        let insert_at = slots[..self.index]
            .iter()
            .filter(|slot| slot.is_some())
            .count();
        let children = panel.children()?;
        if insert_at < children.len()? {
            children.insert(insert_at, child)?;
        } else {
            children.add(child)?;
        }
        slots[self.index] = Some(child_id);
        Ok(())
    }

    pub fn unmount(&self, panel_owner: &impl Upcast<Panel>, child: &impl AsControl) -> Result<()> {
        let child_id = child.as_control()?.object_id()?;
        let mut slots = self.slots.borrow_mut();
        if slots.get(self.index).copied().flatten() != Some(child_id) {
            return Ok(());
        }
        let children = panel_owner.upcast_ref().children()?;
        for index in 0..children.len()? {
            if children.get(index)?.object_id()? == child_id {
                children.remove(index)?;
                slots[self.index] = None;
                return Ok(());
            }
        }
        slots[self.index] = None;
        Ok(())
    }
}

impl El<crate::Button> {
    /// Attaches an asynchronous click handler that automatically disables the
    /// button while `handler`'s future is running and re-enables it upon completion.
    pub fn on_click_async<Fut, R>(self, mut handler: impl FnMut() -> Fut + 'static) -> Self
    where
        Fut: Future<Output = R> + 'static,
        R: crate::IntoEventResult + 'static,
    {
        self.with_op(move |scope, button| {
            let scope_for_click = scope.clone();
            let button_for_click = button.clone();
            let busy = Rc::new(std::cell::Cell::new(false));
            let sub = button.subscribe_click(move |_| {
                if busy.get() {
                    return;
                }
                busy.set(true);
                let _ = button_for_click.set_enabled(false);
                struct ReenableGuard {
                    button: crate::Button,
                    busy: Rc<std::cell::Cell<bool>>,
                }
                impl Drop for ReenableGuard {
                    fn drop(&mut self) {
                        self.busy.set(false);
                        let _ = self.button.set_enabled(true);
                    }
                }
                let guard = ReenableGuard {
                    button: button_for_click.clone(),
                    busy: busy.clone(),
                };
                let fut = handler();
                let _ = scope_for_click.spawn_local(async move {
                    let _guard = guard;
                    fut.await
                });
            })?;
            scope.retain_subscription(sub);
            Ok(())
        })
    }
}
