//! Safe, UI-thread-friendly Rust wrappers for projected callback delegates and
//! notifications (`ItemFilter`, `TextFilter`, `ItemSelector`, `TextSelector`,
//! `PopupPlacementCallback`, `AsyncPopulator`, `Notification`, and `IntoContent`).

use crate::reactive::UiCallbackHandle;
use crate::runtime::{report_ui_error, AsControl, IntoEventResult};
use crate::{
    Control, Error, NotificationType, PopupAnchor, PopupGravity,
    PopupPositionerConstraintAdjustment, Rect, Result, Size, TextBlock, Variant, Vector,
};
use rustolonia_sys as sys;
use std::time::Duration;

/// Ergonomic content input for imperative `set_content` / `set_header` / `set_child`
/// calls on live control handles, accepting strings (`"Submit"`), control references
/// (`&icon`), owned `Option<Control>`, or bare `None` to clear content.
pub trait IntoContent {
    fn into_content_control(self) -> Result<Option<sys::ComPtr<sys::IAvnControl>>>;
}

impl IntoContent for &str {
    fn into_content_control(self) -> Result<Option<sys::ComPtr<sys::IAvnControl>>> {
        let tb = TextBlock::new()?;
        tb.set_text(self)?;
        Ok(Some(tb.as_control()?))
    }
}

impl IntoContent for String {
    fn into_content_control(self) -> Result<Option<sys::ComPtr<sys::IAvnControl>>> {
        self.as_str().into_content_control()
    }
}

impl IntoContent for &String {
    fn into_content_control(self) -> Result<Option<sys::ComPtr<sys::IAvnControl>>> {
        self.as_str().into_content_control()
    }
}

impl<T: AsControl + ?Sized> IntoContent for &T {
    fn into_content_control(self) -> Result<Option<sys::ComPtr<sys::IAvnControl>>> {
        Ok(Some(self.as_control()?))
    }
}

impl IntoContent for Option<Control> {
    fn into_content_control(self) -> Result<Option<sys::ComPtr<sys::IAvnControl>>> {
        self.map(|c| c.as_control()).transpose()
    }
}

/// Safe predicate wrapper for `AutoCompleteBox::item_filter`.
#[derive(Clone, Debug)]
pub struct ItemFilter {
    pub(crate) raw: sys::ComPtr<sys::IAvnItemFilter>,
}

impl ItemFilter {
    pub fn new(mut predicate: impl FnMut(&str, Variant) -> bool + 'static) -> Self {
        let cb = UiCallbackHandle::new(move |search: &str, item: Variant| predicate(search, item));
        let filter = sys::item_filter(move |search_ptr, variant_abi| {
            let search = unsafe { sys::clone_utf16(search_ptr) }.unwrap_or_default();
            let item = unsafe { Variant::from_borrowed_abi(variant_abi)? };
            cb.with(|f| f(&search, item))
                .map_err(|_| sys::Error(sys::E_FAIL))
        });
        Self {
            raw: filter.as_com_ptr().clone(),
        }
    }
}

impl<F> From<F> for ItemFilter
where
    F: FnMut(&str, Variant) -> bool + 'static,
{
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

/// Safe predicate wrapper for `AutoCompleteBox::text_filter`.
#[derive(Clone, Debug)]
pub struct TextFilter {
    pub(crate) raw: sys::ComPtr<sys::IAvnTextFilter>,
}

impl TextFilter {
    pub fn new(mut predicate: impl FnMut(&str, &str) -> bool + 'static) -> Self {
        let cb = UiCallbackHandle::new(move |search: &str, text: &str| predicate(search, text));
        let filter = sys::text_filter(move |search_ptr, text_ptr| {
            let search = unsafe { sys::clone_utf16(search_ptr) }.unwrap_or_default();
            let text = unsafe { sys::clone_utf16(text_ptr) }.unwrap_or_default();
            cb.with(|f| f(&search, &text))
                .map_err(|_| sys::Error(sys::E_FAIL))
        });
        Self {
            raw: filter.as_com_ptr().clone(),
        }
    }
}

impl<F> From<F> for TextFilter
where
    F: FnMut(&str, &str) -> bool + 'static,
{
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

/// Safe item-to-text projection wrapper for `AutoCompleteBox::item_selector`.
#[derive(Clone, Debug)]
pub struct ItemSelector {
    pub(crate) raw: sys::ComPtr<sys::IAvnItemSelector>,
}

impl ItemSelector {
    pub fn new(mut selector: impl FnMut(&str, Variant) -> String + 'static) -> Self {
        let cb = UiCallbackHandle::new(move |search: &str, item: Variant| selector(search, item));
        let sel = sys::item_selector(move |search_ptr, variant_abi| {
            let search = unsafe { sys::clone_utf16(search_ptr) }.unwrap_or_default();
            let item = unsafe { Variant::from_borrowed_abi(variant_abi)? };
            cb.with(|f| f(&search, item))
                .map_err(|_| sys::Error(sys::E_FAIL))
        });
        Self {
            raw: sel.as_com_ptr().clone(),
        }
    }
}

impl<F> From<F> for ItemSelector
where
    F: FnMut(&str, Variant) -> String + 'static,
{
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

/// Safe text-to-text projection wrapper for `AutoCompleteBox::text_selector`.
#[derive(Clone, Debug)]
pub struct TextSelector {
    pub(crate) raw: sys::ComPtr<sys::IAvnTextSelector>,
}

impl TextSelector {
    pub fn new(mut selector: impl FnMut(&str, &str) -> String + 'static) -> Self {
        let cb = UiCallbackHandle::new(move |search: &str, item: &str| selector(search, item));
        let sel = sys::text_selector(move |search_ptr, item_ptr| {
            let search = unsafe { sys::clone_utf16(search_ptr) }.unwrap_or_default();
            let item = unsafe { sys::clone_utf16(item_ptr) }.unwrap_or_default();
            cb.with(|f| f(&search, &item))
                .map_err(|_| sys::Error(sys::E_FAIL))
        });
        Self {
            raw: sel.as_com_ptr().clone(),
        }
    }
}

impl<F> From<F> for TextSelector
where
    F: FnMut(&str, &str) -> String + 'static,
{
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

/// Geometry input passed to a [`PopupPlacementCallback`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopupPlacementInput {
    pub popup_size: Size,
    pub anchor_rect: Rect,
}

/// Placement returned by a [`PopupPlacementCallback`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopupPlacementOutput {
    pub offset: Vector,
    pub anchor: PopupAnchor,
    pub gravity: PopupGravity,
    pub constraint_adjustment: PopupPositionerConstraintAdjustment,
}

/// Safe custom placement callback for `Popup::custom_popup_placement_callback`.
#[derive(Clone, Debug)]
pub struct PopupPlacementCallback {
    pub(crate) raw: sys::ComPtr<sys::IAvnPopupPlacementCallback>,
}

impl PopupPlacementCallback {
    pub fn new(
        mut callback: impl FnMut(PopupPlacementInput) -> PopupPlacementOutput + 'static,
    ) -> Self {
        let cb = UiCallbackHandle::new(move |input: PopupPlacementInput| callback(input));
        let placement = sys::popup_placement(move |pw, ph, ax, ay, aw, ah| {
            let input = PopupPlacementInput {
                popup_size: Size::new(pw, ph),
                anchor_rect: Rect::new(ax, ay, aw, ah),
            };
            let out = cb.with(|f| f(input)).map_err(|_| sys::Error(sys::E_FAIL))?;
            Ok(sys::PopupPlacementResult {
                offset_x: out.offset.x,
                offset_y: out.offset.y,
                anchor: out.anchor.bits(),
                gravity: out.gravity.bits(),
                constraint_adjustment: out.constraint_adjustment.bits(),
            })
        });
        Self {
            raw: placement.as_com_ptr().clone(),
        }
    }
}

impl<F> From<F> for PopupPlacementCallback
where
    F: FnMut(PopupPlacementInput) -> PopupPlacementOutput + 'static,
{
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

/// Completion handle passed to an [`AsyncPopulator`] request.
pub struct PopulateRequest {
    request_id: i64,
    inner: sys::PopulateCompletion,
}

impl PopulateRequest {
    pub fn request_id(&self) -> i64 {
        self.request_id
    }

    /// Completes the population request with no suggestions.
    pub fn complete_empty(self) -> Result<()> {
        Ok(self.inner.complete(self.request_id)?)
    }

    /// Completes the population request with Rust-owned string suggestions.
    pub fn complete_strings(self, suggestions: Vec<String>) -> Result<()> {
        Ok(self.inner.complete_strings(self.request_id, suggestions)?)
    }

    /// Reports a failure HRESULT to the host for this population request.
    pub fn fail(self) -> Result<()> {
        Ok(self.inner.complete_with(self.request_id, sys::E_FAIL)?)
    }
}

/// Safe asynchronous suggestion populator for `AutoCompleteBox::async_populator`.
#[derive(Clone, Debug)]
pub struct AsyncPopulator {
    pub(crate) raw: sys::ComPtr<sys::IAvnAsyncPopulator>,
}

impl AsyncPopulator {
    pub fn new<R: IntoEventResult>(
        mut callback: impl FnMut(&str, PopulateRequest) -> R + 'static,
    ) -> Self {
        let cb = UiCallbackHandle::new(move |search: &str, req: PopulateRequest| {
            callback(search, req).into_event_result()
        });
        let populator = sys::async_populator(move |request_id, search_ptr, completion| {
            let search = unsafe { sys::clone_utf16(search_ptr) }.unwrap_or_default();
            let req = PopulateRequest {
                request_id,
                inner: completion,
            };
            match cb.with(|f| f(&search, req)) {
                Ok(Ok(())) => Ok(()),
                Ok(Err(Error::Abi(err))) => Err(err),
                Ok(Err(err)) => {
                    report_ui_error("async populator callback failed", &err);
                    Err(sys::Error(sys::E_FAIL))
                }
                Err(_) => Err(sys::Error(sys::E_FAIL)),
            }
        });
        Self {
            raw: populator.as_com_ptr().clone(),
        }
    }
}

/// Safe, builder-style desktop/in-window notification for `WindowNotificationManager::show`.
#[derive(Clone, Debug)]
pub struct Notification {
    title: String,
    message: String,
    kind: NotificationType,
    expiration: Duration,
    on_click: Option<sys::ComPtr<sys::IAvnNotificationActionHandler>>,
    on_close: Option<sys::ComPtr<sys::IAvnNotificationActionHandler>>,
}

impl Notification {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            kind: NotificationType::Information,
            expiration: Duration::from_secs(5),
            on_click: None,
            on_close: None,
        }
    }

    pub fn kind(mut self, kind: NotificationType) -> Self {
        self.kind = kind;
        self
    }

    pub fn expiration(mut self, expiration: Duration) -> Self {
        self.expiration = expiration;
        self
    }

    pub fn on_click<R: IntoEventResult>(
        mut self,
        mut callback: impl FnMut() -> R + 'static,
    ) -> Self {
        let cb = UiCallbackHandle::new(move || {
            if let Err(err) = callback().into_event_result() {
                report_ui_error("notification click callback failed", &err);
            }
        });
        self.on_click = Some(sys::notification_action_handler(move || {
            let _ = cb.with(|f| f());
            Ok(())
        }));
        self
    }

    pub fn on_close<R: IntoEventResult>(
        mut self,
        mut callback: impl FnMut() -> R + 'static,
    ) -> Self {
        let cb = UiCallbackHandle::new(move || {
            if let Err(err) = callback().into_event_result() {
                report_ui_error("notification close callback failed", &err);
            }
        });
        self.on_close = Some(sys::notification_action_handler(move || {
            let _ = cb.with(|f| f());
            Ok(())
        }));
        self
    }

    pub(crate) fn to_raw(&self) -> sys::ComPtr<sys::IAvnNotification> {
        let ticks = (self.expiration.as_nanos() / 100).clamp(0, i64::MAX as u128) as i64;
        sys::notification(sys::NotificationSpec {
            title: self.title.clone(),
            message: self.message.clone(),
            kind: self.kind as i32,
            expiration_ticks: ticks,
            on_click: self.on_click.clone(),
            on_close: self.on_close.clone(),
        })
        .as_com_ptr()
        .clone()
    }
}
