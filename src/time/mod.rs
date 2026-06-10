use alloc::vec::Vec;
use core::{sync::atomic::AtomicU64, sync::atomic::Ordering, task::Waker, time::Duration};
use x86_64::instructions::interrupts::without_interrupts;

use crate::spinlock::Mutex;

pub mod pit;
pub mod rtc;

// 500 million years
static MS_COUNT: AtomicU64 = AtomicU64::new(0);

const MS_HZ: f64 = 1000.0;

pub fn init() {
    pit::set_interval(MS_HZ);
}

pub fn get_ticks() -> u64 {
    MS_COUNT.load(Ordering::Relaxed)
}

struct TimeWaker {
    target_tick: u64,
    waker: Waker,
}

static TIMER_WAKERS: Mutex<Vec<TimeWaker>> = Mutex::new(Vec::new());

pub fn register_waker(sleep_ms: u64, waker: Waker) {
    // let ms_count = u64::try_from(sleep_time.as_millis()).expect("Waker sleep too long");
    let target_tick = get_ticks() + sleep_ms;

    without_interrupts(|| {
        TIMER_WAKERS.lock().push(TimeWaker { target_tick, waker });
    });
}

/// Wake any expired tasks
pub fn wake_expired() {
    let now = get_ticks();

    let mut wakers = TIMER_WAKERS.lock();

    wakers.retain(|waker| {
        if now >= waker.target_tick {
            waker.waker.wake_by_ref();
            false
        } else {
            true
        }
    });
}
