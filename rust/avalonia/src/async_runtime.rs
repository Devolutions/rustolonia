use crate::{Error, Result};
use avalonia_sys as sys;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll, Wake, Waker};

#[derive(Debug)]
pub enum AsyncValue {
    None,
    Boolean(bool),
    Integer(i64),
    Double(f64),
    String(Option<String>),
}

#[derive(Debug)]
pub(crate) struct AsyncFailure {
    pub(crate) hresult: i32,
    pub(crate) message: String,
}

/// The shared, executor-neutral half of every host-started operation: exactly
/// one result, plus the waker registered by whichever executor is polling.
#[derive(Debug)]
pub(crate) struct CompletionSlot<T> {
    completed: bool,
    result: Option<std::result::Result<T, AsyncFailure>>,
    waker: Option<Waker>,
}

impl<T> Default for CompletionSlot<T> {
    fn default() -> Self {
        Self {
            completed: false,
            result: None,
            waker: None,
        }
    }
}

impl<T> CompletionSlot<T> {
    /// Records the single completion. A second completion is rejected with
    /// `E_FAIL` instead of overwriting the first, so a misbehaving host cannot
    /// resolve the same operation twice.
    pub(crate) fn publish(
        state: &Arc<Mutex<Self>>,
        result: std::result::Result<T, AsyncFailure>,
    ) -> sys::Result<()> {
        let waker = {
            let mut state = state.lock().expect("async operation state lock poisoned");
            if state.completed {
                return Err(sys::Error(sys::E_FAIL));
            }
            state.completed = true;
            state.result = Some(result);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
        Ok(())
    }

    pub(crate) fn poll(state: &Arc<Mutex<Self>>, context: &mut Context<'_>) -> Poll<Result<T>> {
        let mut state = state.lock().expect("async operation state lock poisoned");
        match state.result.take() {
            Some(Ok(value)) => Poll::Ready(Ok(value)),
            Some(Err(error)) => Poll::Ready(Err(Error::Async {
                hresult: error.hresult,
                message: error.message,
            })),
            None => {
                assert!(!state.completed, "completed operation polled again");
                state.waker = Some(context.waker().clone());
                Poll::Pending
            }
        }
    }

    pub(crate) fn is_pending(state: &Arc<Mutex<Self>>) -> bool {
        !state
            .lock()
            .expect("async operation state lock poisoned")
            .completed
    }
}

pub struct AsyncOperation<T> {
    application: sys::ComPtr<sys::IAvnApplication>,
    operation_id: i64,
    _completion: sys::ComPtr<sys::IAvnAsyncCompletion>,
    state: Arc<Mutex<CompletionSlot<AsyncValue>>>,
    decode: fn(AsyncValue) -> Result<T>,
    _result: PhantomData<T>,
}

impl<T> AsyncOperation<T> {
    pub(crate) fn start(
        application: sys::ComPtr<sys::IAvnApplication>,
        start: impl FnOnce(&sys::ComPtr<sys::IAvnAsyncCompletion>) -> sys::Result<i64>,
        decode: fn(AsyncValue) -> Result<T>,
    ) -> Result<Self> {
        let state = Arc::new(Mutex::new(CompletionSlot::default()));
        let completion_state = state.clone();
        let completion = sys::async_completion(move |arguments| {
            CompletionSlot::publish(&completion_state, decode_completion(arguments))
        });
        let operation_id = start(&completion)?;
        Ok(Self {
            application,
            operation_id,
            _completion: completion,
            state,
            decode,
            _result: PhantomData,
        })
    }
}

impl<T> Future for AsyncOperation<T> {
    type Output = Result<T>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match CompletionSlot::poll(&self.state, context) {
            Poll::Ready(Ok(value)) => Poll::Ready((self.decode)(value)),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> Drop for AsyncOperation<T> {
    fn drop(&mut self) {
        if CompletionSlot::is_pending(&self.state) {
            let _ = self.application.cancel_async_operation(self.operation_id);
        }
    }
}

pub(crate) fn decode_none(value: AsyncValue) -> Result<()> {
    match value {
        AsyncValue::None => Ok(()),
        _ => Err(Error::InvalidAsyncValue),
    }
}

pub(crate) fn decode_string(value: AsyncValue) -> Result<Option<String>> {
    match value {
        AsyncValue::String(value) => Ok(value),
        _ => Err(Error::InvalidAsyncValue),
    }
}

fn decode_completion(
    arguments: &sys::AsyncCompletionArgs,
) -> std::result::Result<AsyncValue, AsyncFailure> {
    if arguments.hresult < 0 {
        return Err(AsyncFailure {
            hresult: arguments.hresult,
            message: arguments
                .error
                .clone()
                .unwrap_or_else(|| format!("async operation failed: 0x{:08X}", arguments.hresult)),
        });
    }
    Ok(match arguments.value_kind {
        0 => AsyncValue::None,
        1 => AsyncValue::Boolean(arguments.integer_value != 0),
        2 => AsyncValue::Integer(arguments.integer_value),
        3 => AsyncValue::Double(arguments.double_value),
        4 => AsyncValue::String(arguments.string_value.clone()),
        _ => {
            return Err(AsyncFailure {
                hresult: sys::E_FAIL,
                message: format!("invalid async value kind {}", arguments.value_kind),
            });
        }
    })
}

#[derive(Default)]
pub(crate) struct TaskScope {
    state: Mutex<ScopeTasks>,
}

#[derive(Default)]
struct ScopeTasks {
    closed: bool,
    tasks: Vec<Arc<ScopedTask>>,
}

impl TaskScope {
    pub(crate) fn spawn(
        self: &Arc<Self>,
        dispatcher: sys::ComPtr<sys::IAvnDispatcher>,
        future: impl Future<Output = ()> + Send + 'static,
    ) -> Result<()> {
        let task = Arc::new(ScopedTask {
            state: Mutex::new(TaskState {
                future: Some(Box::pin(future)),
                polling: false,
                scheduled: false,
                notified: false,
                finished: false,
            }),
            dispatcher,
            scope: Arc::downgrade(self),
        });
        {
            let mut state = self.state.lock().expect("task scope lock poisoned");
            if state.closed {
                return Err(sys::Error(sys::E_FAIL).into());
            }
            state.tasks.push(task.clone());
        }
        task.schedule()
    }

    pub(crate) fn clear(&self) {
        let tasks = {
            let mut state = self.state.lock().expect("task scope lock poisoned");
            state.closed = true;
            std::mem::take(&mut state.tasks)
        };
        for task in tasks {
            task.cancel();
        }
    }
}

struct TaskState {
    future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
    polling: bool,
    scheduled: bool,
    notified: bool,
    finished: bool,
}

struct ScopedTask {
    state: Mutex<TaskState>,
    dispatcher: sys::ComPtr<sys::IAvnDispatcher>,
    scope: Weak<TaskScope>,
}

impl ScopedTask {
    fn unregister(&self) {
        if let Some(scope) = self.scope.upgrade() {
            let removed = {
                let mut state = scope.state.lock().expect("task scope lock poisoned");
                state
                    .tasks
                    .iter()
                    .position(|task| std::ptr::eq(task.as_ref(), self))
                    .map(|index| state.tasks.swap_remove(index))
            };
            drop(removed);
        }
    }

    fn cancel(&self) {
        let future = {
            let mut state = self.state.lock().expect("scoped task lock poisoned");
            state.finished = true;
            state.future.take()
        };
        self.unregister();
        drop(future);
    }

    fn schedule(self: Arc<Self>) -> Result<()> {
        {
            let mut state = self.state.lock().expect("scoped task lock poisoned");
            if state.finished {
                return Ok(());
            }
            state.notified = true;
            if state.polling || state.scheduled {
                return Ok(());
            }
            state.scheduled = true;
        }
        let dispatcher = self.dispatcher.clone();
        let task = self.clone();
        let action = sys::action(move || {
            task.poll();
            Ok(())
        });
        if let Err(error) = dispatcher.post(&action) {
            self.cancel();
            return Err(error.into());
        }
        Ok(())
    }

    fn poll(self: Arc<Self>) {
        let mut future = {
            let mut state = self.state.lock().expect("scoped task lock poisoned");
            state.scheduled = false;
            if state.finished || state.polling {
                return;
            }
            state.polling = true;
            state.notified = false;
            state.future.take().expect("active task future")
        };
        let waker = Waker::from(self.clone());
        let mut context = Context::from_waker(&waker);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            future.as_mut().poll(&mut context)
        }));
        let pending = match outcome {
            Ok(value) => value.is_pending(),
            Err(error) => {
                self.cancel();
                std::panic::resume_unwind(error);
            }
        };
        let mut future = Some(future);
        let (finished, notified) = {
            let mut state = self.state.lock().expect("scoped task lock poisoned");
            state.polling = false;
            state.finished |= !pending;
            if !state.finished {
                state.future = future.take();
            }
            (state.finished, state.notified)
        };
        if finished {
            self.unregister();
        }
        drop(future);
        if !finished && notified {
            let _ = self.schedule();
        }
    }
}

impl Wake for ScopedTask {
    fn wake(self: Arc<Self>) {
        let _ = self.schedule();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct Noop;
    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }

    #[test]
    fn consumed_completions_remain_terminal() {
        for result in [
            Ok(7),
            Err(AsyncFailure {
                hresult: sys::E_FAIL,
                message: "failure".into(),
            }),
        ] {
            let state = Arc::new(Mutex::new(CompletionSlot::default()));
            let waker = Waker::from(Arc::new(Noop));
            let mut cx = Context::from_waker(&waker);
            assert!(CompletionSlot::is_pending(&state));
            assert!(CompletionSlot::poll(&state, &mut cx).is_pending());
            CompletionSlot::publish(&state, result).unwrap();
            assert!(!CompletionSlot::is_pending(&state));
            assert!(CompletionSlot::poll(&state, &mut cx).is_ready());
            assert!(!CompletionSlot::is_pending(&state));
            assert!(CompletionSlot::publish(&state, Ok(8)).is_err());
        }
    }

    #[repr(C)]
    struct DispatcherVtbl {
        query:
            unsafe extern "system" fn(*mut Dispatcher, *const sys::Guid, *mut *mut c_void) -> i32,
        add: unsafe extern "system" fn(*mut Dispatcher) -> u32,
        release: unsafe extern "system" fn(*mut Dispatcher) -> u32,
        access: unsafe extern "system" fn(*mut Dispatcher, *mut i32) -> i32,
        post: unsafe extern "system" fn(*mut Dispatcher, *mut sys::IAvnAction) -> i32,
    }

    #[repr(C)]
    struct Dispatcher {
        vtbl: &'static DispatcherVtbl,
        queue: Mutex<Vec<sys::ComPtr<sys::IAvnAction>>>,
        fail: AtomicBool,
    }

    unsafe extern "system" fn query(
        _: *mut Dispatcher,
        _: *const sys::Guid,
        _: *mut *mut c_void,
    ) -> i32 {
        sys::E_FAIL
    }
    unsafe extern "system" fn add(this: *mut Dispatcher) -> u32 {
        Arc::increment_strong_count(this);
        2
    }
    unsafe extern "system" fn release(this: *mut Dispatcher) -> u32 {
        Arc::decrement_strong_count(this);
        1
    }
    unsafe extern "system" fn access(_: *mut Dispatcher, value: *mut i32) -> i32 {
        *value = 1;
        0
    }
    unsafe extern "system" fn post(this: *mut Dispatcher, action: *mut sys::IAvnAction) -> i32 {
        if (*this).fail.load(Ordering::Relaxed) {
            return sys::E_FAIL;
        }
        let action = std::mem::ManuallyDrop::new(sys::ComPtr::from_raw(action).unwrap());
        (*this).queue.lock().unwrap().push((*action).clone());
        0
    }
    static VTABLE: DispatcherVtbl = DispatcherVtbl {
        query,
        add,
        release,
        access,
        post,
    };

    fn dispatcher() -> (Arc<Dispatcher>, sys::ComPtr<sys::IAvnDispatcher>) {
        let dispatcher = Arc::new(Dispatcher {
            vtbl: &VTABLE,
            queue: Mutex::new(Vec::new()),
            fail: AtomicBool::new(false),
        });
        let raw = Arc::into_raw(dispatcher.clone()) as *mut sys::IAvnDispatcher;
        (dispatcher, unsafe { sys::ComPtr::from_raw(raw).unwrap() })
    }

    fn drain(dispatcher: &Dispatcher) {
        loop {
            let action = dispatcher.queue.lock().unwrap().pop();
            let Some(action) = action else { break };
            unsafe {
                let table = *(action.as_raw() as *const *const *const c_void);
                let invoke: unsafe extern "system" fn(*mut sys::IAvnAction) -> i32 =
                    std::mem::transmute(*table.add(3));
                assert_eq!(invoke(action.as_raw()), 0);
            }
        }
    }

    #[test]
    fn completed_tasks_are_removed_and_self_wakes_are_not_lost() {
        let (dispatcher, raw) = dispatcher();
        let scope = Arc::new(TaskScope::default());
        for _ in 0..1000 {
            scope.spawn(raw.clone(), async {}).unwrap();
            let weak = Arc::downgrade(&scope.state.lock().unwrap().tasks[0]);
            drain(&dispatcher);
            assert!(scope.state.lock().unwrap().tasks.is_empty());
            assert!(weak.upgrade().is_none());
        }
        let polls = Arc::new(AtomicUsize::new(0));
        let count = polls.clone();
        scope
            .spawn(
                raw,
                std::future::poll_fn(move |cx| {
                    if count.fetch_add(1, Ordering::Relaxed) == 0 {
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    } else {
                        Poll::Ready(())
                    }
                }),
            )
            .unwrap();
        drain(&dispatcher);
        assert_eq!(polls.load(Ordering::Relaxed), 2);
        assert!(scope.state.lock().unwrap().tasks.is_empty());
    }

    #[test]
    fn shutdown_during_poll_does_not_restore_future_or_deadlock() {
        let (dispatcher, raw) = dispatcher();
        let scope = Arc::new(TaskScope::default());
        let owner = scope.clone();
        scope
            .spawn(
                raw.clone(),
                std::future::poll_fn(move |cx| {
                    owner.clear();
                    cx.waker().wake_by_ref();
                    Poll::<()>::Pending
                }),
            )
            .unwrap();
        drain(&dispatcher);
        assert!(scope.state.lock().unwrap().tasks.is_empty());
        assert!(scope.spawn(raw, async {}).is_err());
    }

    #[test]
    fn failed_dispatch_removes_task() {
        let (dispatcher, raw) = dispatcher();
        let scope = Arc::new(TaskScope::default());
        dispatcher.fail.store(true, Ordering::Relaxed);
        assert!(scope.spawn(raw, async {}).is_err());
        assert!(scope.state.lock().unwrap().tasks.is_empty());
    }

    #[test]
    fn failed_wake_dispatch_cancels_an_already_pending_task() {
        let (dispatcher, raw) = dispatcher();
        let scope = Arc::new(TaskScope::default());
        let saved = Arc::new(Mutex::new(None));
        let capture = saved.clone();
        scope
            .spawn(
                raw,
                std::future::poll_fn(move |cx| {
                    *capture.lock().unwrap() = Some(cx.waker().clone());
                    Poll::<()>::Pending
                }),
            )
            .unwrap();
        drain(&dispatcher);
        assert_eq!(scope.state.lock().unwrap().tasks.len(), 1);
        dispatcher.fail.store(true, Ordering::Relaxed);
        saved.lock().unwrap().take().unwrap().wake();
        assert!(scope.state.lock().unwrap().tasks.is_empty());
    }

    #[test]
    fn cancellation_drops_futures_outside_scope_and_task_locks() {
        struct ReentrantDrop(Arc<TaskScope>, Arc<AtomicBool>);
        impl Future for ReentrantDrop {
            type Output = ();
            fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
                Poll::Pending
            }
        }
        impl Drop for ReentrantDrop {
            fn drop(&mut self) {
                self.0.clear();
                self.1.store(true, Ordering::Relaxed);
            }
        }
        let (dispatcher, raw) = dispatcher();
        let scope = Arc::new(TaskScope::default());
        let dropped = Arc::new(AtomicBool::new(false));
        scope
            .spawn(raw, ReentrantDrop(scope.clone(), dropped.clone()))
            .unwrap();
        drain(&dispatcher);
        scope.clear();
        assert!(dropped.load(Ordering::Relaxed));
        assert!(scope.state.lock().unwrap().tasks.is_empty());
    }
}
