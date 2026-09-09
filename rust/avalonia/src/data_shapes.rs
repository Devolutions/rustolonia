//! Stage 30 building blocks shared by the generated view-model surfaces:
//! observable keyed maps, range-backed windows, tracked async cancellation and
//! exactly-once command completion.
//!
//! Everything here is deliberately independent of any application schema. The
//! generated code supplies IDs and named methods; this module supplies the
//! ownership, threading and idempotence rules.

use avalonia_sys as sys;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

/// A map key in its owned Rust form. The schema fixes which variant a given
/// map uses; the generated named APIs build it so application code never sees
/// the transport encoding.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapKey {
    Text(String),
    Integer(i64),
}

impl MapKey {
    pub(crate) fn to_wire(&self) -> sys::MapKey {
        match self {
            Self::Text(value) => sys::MapKey::Text(value.encode_utf16().chain(Some(0)).collect()),
            Self::Integer(value) => sys::MapKey::Integer(*value),
        }
    }
}

impl From<&str> for MapKey {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for MapKey {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<i64> for MapKey {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

/// One range request managed code posted for a windowed collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeRequest {
    pub collection_id: i32,
    pub offset: i64,
    pub length: i32,
    pub generation: i64,
}

/// The dataset identity of one windowed collection, readable without taking
/// the view-model lock. Managed code reads it from the UI thread through
/// `IAvnRustRangeSource::GetRangeState`, so it must never queue behind a Rust
/// worker.
#[derive(Debug, Default)]
pub(crate) struct RangeStates {
    states: Mutex<HashMap<i32, (i64, i64)>>,
}

impl RangeStates {
    pub(crate) fn set(&self, collection_id: i32, generation: i64, total_count: i64) {
        self.lock().insert(collection_id, (generation, total_count));
    }

    pub(crate) fn get(&self, collection_id: i32) -> Option<(i64, i64)> {
        self.lock().get(&collection_id).copied()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<i32, (i64, i64)>> {
        self.states.lock().expect("range state lock poisoned")
    }
}

/// Maximum number of outstanding range requests. Fast scrolling produces
/// requests far quicker than a dataset can answer them, so the queue is
/// bounded and the *oldest* request is dropped: the newest viewport is always
/// the one worth answering.
const MAX_PENDING_RANGE_REQUESTS: usize = 32;

struct RangeQueueState {
    queue: VecDeque<RangeRequest>,
    closed: bool,
    started: bool,
}

/// A bounded, coalescing request queue drained on a dedicated Rust thread.
///
/// Managed code enqueues from the UI thread and returns immediately; the
/// drain thread is the only place the application model is locked, so a range
/// request can never block the UI behind a running worker.
pub(crate) struct RangeQueue {
    state: Mutex<RangeQueueState>,
    signal: Condvar,
}

impl Default for RangeQueue {
    fn default() -> Self {
        Self {
            state: Mutex::new(RangeQueueState {
                queue: VecDeque::new(),
                closed: false,
                started: false,
            }),
            signal: Condvar::new(),
        }
    }
}

/// The outcome of enqueueing one range request.
pub(crate) struct RangeEnqueue {
    /// True when the caller must start the drain thread.
    pub(crate) start: bool,

    /// The collection ID and offset of a request the bounded queue evicted to
    /// make room, if any. A dropped request never produces a range batch, so
    /// its owning collection must be told to stop waiting for it -- and the
    /// queue is shared by every windowed collection on the model, so the ID
    /// matters as much as the offset.
    pub(crate) dropped: Option<(i32, i64)>,
}

impl RangeQueue {
    /// Enqueues a request, coalescing an identical outstanding one.
    pub(crate) fn enqueue(&self, request: RangeRequest) -> RangeEnqueue {
        let mut state = self.state.lock().expect("range queue lock poisoned");
        if state.closed {
            return RangeEnqueue {
                start: false,
                dropped: None,
            };
        }
        let mut dropped = None;
        if !state.queue.iter().any(|pending| *pending == request) {
            if state.queue.len() >= MAX_PENDING_RANGE_REQUESTS {
                dropped = state
                    .queue
                    .pop_front()
                    .map(|evicted| (evicted.collection_id, evicted.offset));
            }
            state.queue.push_back(request);
        }
        // `started` is latched by `mark_started` only after the caller has
        // actually spawned the drain thread, so a failed spawn cannot leave
        // the queue permanently unattended.
        let start = !state.started;
        drop(state);
        self.signal.notify_one();
        RangeEnqueue { start, dropped }
    }

    /// Latches the drain thread as started. Called only after a successful spawn.
    pub(crate) fn mark_started(&self) {
        self.state
            .lock()
            .expect("range queue lock poisoned")
            .started = true;
    }

    /// Blocks until a request is available or the queue closes.
    pub(crate) fn take(&self) -> Option<RangeRequest> {
        let mut state = self.state.lock().expect("range queue lock poisoned");
        loop {
            if let Some(request) = state.queue.pop_front() {
                return Some(request);
            }
            if state.closed {
                return None;
            }
            state = self
                .signal
                .wait(state)
                .expect("range queue condvar poisoned");
        }
    }

    /// Closes the queue so the drain thread exits. Idempotent.
    pub(crate) fn close(&self) {
        let mut state = self.state.lock().expect("range queue lock poisoned");
        state.closed = true;
        state.queue.clear();
        drop(state);
        self.signal.notify_all();
    }

    #[cfg(test)]
    pub(crate) fn pending(&self) -> usize {
        self.state
            .lock()
            .expect("range queue lock poisoned")
            .queue
            .len()
    }
}

/// Closes a [`RangeQueue`] when the owning COM object is released, so the
/// drain thread never outlives the view model it serves.
pub(crate) struct RangeQueueGuard(pub(crate) Arc<RangeQueue>);

impl Drop for RangeQueueGuard {
    fn drop(&mut self) {
        self.0.close();
    }
}

/// A cooperative cancellation handle for one tracked async invocation.
///
/// The handle belongs to exactly one invocation and is never reused, so a
/// cancellation that arrives after the invocation already finished is dropped
/// rather than aborting its successor.
#[derive(Clone, Debug)]
pub struct CancellationToken {
    operation_id: i64,
    flag: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Creates a token that is never cancelled. Useful in tests and for
    /// producers that do not expose cancellation.
    pub fn none() -> Self {
        Self {
            operation_id: 0,
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// The never-reused invocation handle managed code cancels with.
    pub fn operation_id(&self) -> i64 {
        self.operation_id
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Cancels locally, for producers that abort their own work.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
    }
}

/// Tracks in-flight cancellation flags. Entries are held weakly so a finished
/// invocation's flag is reclaimed without any explicit deregistration.
#[derive(Debug, Default)]
pub(crate) struct CancellationRegistry {
    tokens: Mutex<HashMap<i64, std::sync::Weak<AtomicBool>>>,
}

impl CancellationRegistry {
    pub(crate) fn create(&self, operation_id: i64) -> CancellationToken {
        let flag = Arc::new(AtomicBool::new(false));
        let mut tokens = self.lock();
        tokens.retain(|_, weak| weak.strong_count() > 0);
        tokens.insert(operation_id, Arc::downgrade(&flag));
        CancellationToken { operation_id, flag }
    }

    pub(crate) fn cancel(&self, operation_id: i64) {
        let mut tokens = self.lock();
        if let Some(weak) = tokens.get(&operation_id) {
            if let Some(flag) = weak.upgrade() {
                flag.store(true, Ordering::Release);
            }
        }
        tokens.retain(|_, weak| weak.strong_count() > 0);
    }

    #[cfg(test)]
    pub(crate) fn tracked(&self) -> usize {
        self.lock().len()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<i64, std::sync::Weak<AtomicBool>>> {
        self.tokens.lock().expect("cancellation lock poisoned")
    }
}

/// Enforces "an async command completes exactly once" for a given invocation.
///
/// A worker may race its own cancellation, its timeout and its success path;
/// only the first of them publishes a terminal state.
#[derive(Debug, Default)]
pub(crate) struct CompletionSlots {
    finished: Mutex<HashMap<(i32, i64), ()>>,
}

impl CompletionSlots {
    /// Claims the terminal transition for one invocation. Returns false when
    /// something already claimed it.
    pub(crate) fn claim(&self, command_id: i32, operation_id: i64) -> bool {
        self.finished
            .lock()
            .expect("completion lock poisoned")
            .insert((command_id, operation_id), ())
            .is_none()
    }

    /// Forgets completed invocations for a command, bounding growth across a
    /// long-lived model.
    pub(crate) fn forget(&self, command_id: i32) {
        self.finished
            .lock()
            .expect("completion lock poisoned")
            .retain(|(id, _), ()| *id != command_id);
    }
}

/// Builds the elements of one realized range.
pub struct RangeBatch {
    pub(crate) collection_id: i32,
    pub(crate) generation: i64,
    pub(crate) total_count: i64,
    pub(crate) offset: i64,
    pub(crate) items: Vec<sys::RustVmRangeItem>,
}

impl RangeBatch {
    #[doc(hidden)]
    pub fn new(collection_id: i32, generation: i64, total_count: i64, offset: i64) -> Self {
        Self {
            collection_id,
            generation,
            total_count,
            offset,
            items: Vec::new(),
        }
    }

    #[doc(hidden)]
    pub fn push_text(&mut self, value: impl AsRef<str>) {
        self.items.push(sys::RustVmRangeItem {
            text: Some(value.as_ref().to_owned()),
            model: None,
        });
    }

    #[doc(hidden)]
    pub fn push_raw_model(&mut self, model: sys::ComPtr<sys::IAvnRustViewModel>) {
        self.items.push(sys::RustVmRangeItem {
            text: None,
            model: Some(model),
        });
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn into_parts(self) -> (i32, i64, i64, i64, Vec<sys::RustVmRangeItem>) {
        (
            self.collection_id,
            self.generation,
            self.total_count,
            self.offset,
            self.items,
        )
    }

    /// The dataset generation this range was produced from.
    pub fn generation(&self) -> i64 {
        self.generation
    }

    /// Index of the first element this range carries.
    pub fn offset(&self) -> i64 {
        self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_queue_coalesces_and_bounds_requests() {
        let queue = RangeQueue::default();
        let request = RangeRequest {
            collection_id: 1,
            offset: 0,
            length: 8,
            generation: 3,
        };
        assert!(queue.enqueue(request).start);
        // `started` is latched only after the caller spawns the drain thread,
        // so a failed spawn cannot leave the queue permanently unattended.
        assert!(queue.enqueue(request).start);
        queue.mark_started();
        assert!(!queue.enqueue(request).start);
        assert_eq!(queue.pending(), 1);
        let mut dropped = Vec::new();
        for index in 0..(MAX_PENDING_RANGE_REQUESTS as i64 + 4) {
            let outcome = queue.enqueue(RangeRequest {
                offset: (index + 1) * 8,
                ..request
            });
            dropped.extend(outcome.dropped);
        }
        assert_eq!(queue.pending(), MAX_PENDING_RANGE_REQUESTS);
        assert_eq!(
            vec![(1, 0), (1, 8), (1, 16), (1, 24), (1, 32)],
            dropped,
            "an evicted request must be reported with its collection so the right window stops waiting"
        );
    }

    #[test]
    fn range_queue_close_releases_waiter() {
        let queue = Arc::new(RangeQueue::default());
        let worker = queue.clone();
        let handle = std::thread::spawn(move || worker.take());
        queue.close();
        assert!(handle.join().expect("worker joins").is_none());
    }

    #[test]
    fn cancellation_is_scoped_to_one_operation() {
        let registry = CancellationRegistry::default();
        let first = registry.create(1);
        let second = registry.create(2);
        registry.cancel(1);
        assert!(first.is_cancelled());
        assert!(!second.is_cancelled());
        registry.cancel(9999);
        assert!(!second.is_cancelled());
    }

    #[test]
    fn dropped_tokens_are_reclaimed() {
        let registry = CancellationRegistry::default();
        {
            let _token = registry.create(1);
            assert_eq!(registry.tracked(), 1);
        }
        let _second = registry.create(2);
        assert_eq!(registry.tracked(), 1);
    }

    #[test]
    fn completion_is_claimed_once() {
        let slots = CompletionSlots::default();
        assert!(slots.claim(4, 7));
        assert!(!slots.claim(4, 7));
        assert!(slots.claim(4, 8));
        slots.forget(4);
        assert!(slots.claim(4, 7));
    }
}
