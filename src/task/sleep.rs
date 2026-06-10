use crate::time::{get_ticks, register_waker};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

pub struct SleepFuture {
    target_tick: u64,
}

pub fn sleep_duration(duration: Duration) -> SleepFuture {
    let ms_count = u64::try_from(duration.as_millis()).expect("Waker sleep too long");
    SleepFuture {
        target_tick: get_ticks() + ms_count,
    }
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if get_ticks() >= self.target_tick {
            Poll::Ready(())
        } else {
            register_waker(self.target_tick, cx.waker().clone());
            Poll::Pending
        }
    }
}
