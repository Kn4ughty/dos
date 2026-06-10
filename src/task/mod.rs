use alloc::boxed::Box;
use core::fmt::Debug;
use core::sync::atomic::{AtomicU64, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use core::{future::Future, pin::Pin};

use core::pin::pin;

pub mod executor;
pub mod keyboard;
pub mod sleep;

pub struct Task {
    id: TaskId,
    name: &'static str,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Debug for Task {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static, name: &'static str) -> Task {
        Task {
            id: TaskId::new(),
            name,
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

/// Contains a unique id to a task, incrementing from 0
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        // Since the only requirement is that the id is unique, order does not matter
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

fn noop_waker() -> Waker {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(core::ptr::null(), &VTABLE),
        |_| {},
        |_| {},
        |_| {},
    );

    unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) }
}

pub fn block_on<F: Future>(fut: F) -> F::Output {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut fut = pin!(fut);

    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => {}
        }
    }
}
