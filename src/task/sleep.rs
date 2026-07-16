use crate::time::{get_ticks, register_waker_absolute};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

pub struct SleepFuture {
    target_tick: u64,
    registered: bool,
}

/// Asynchronously sleeps for at least `duration`
/// The resolution of the sleep time is found in whatever `time::get_ticks` does
#[must_use]
pub fn sleep_duration(duration: Duration) -> SleepFuture {
    let ms_count = u64::try_from(duration.as_millis()).expect("Waker sleep too long");
    SleepFuture {
        target_tick: get_ticks() + ms_count,
        registered: false,
    }
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if get_ticks() >= self.target_tick {
            return Poll::Ready(());
        }

        if !self.registered {
            register_waker_absolute(self.target_tick, cx.waker().clone());
            self.registered = true;
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::get_ticks;

    #[test_case]
    fn sleep_sleeps() {
        let start = get_ticks();

        crate::task::block_on(async { sleep_duration(Duration::from_millis(50)).await });

        let end = get_ticks();
        assert!((end - start) >= 50);
    }
}
