use core::{
    sync::atomic::{AtomicU64, Ordering},
    task::Waker,
    time::Duration,
};
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

#[derive(Debug, Clone)]
struct TimeWaker {
    target_tick: u64,
    waker: Waker,
}

const MAX_SLEEPERS: usize = crate::task::executor::MAX_TASKS;

static TIMER_WAKERS: Mutex<TimerWakerList> = Mutex::new(TimerWakerList::new());

#[derive(Debug)]
struct TimerWakerList([Option<TimeWaker>; MAX_SLEEPERS]);

impl TimerWakerList {
    pub const fn new() -> Self {
        TimerWakerList([const { None }; MAX_SLEEPERS])
    }

    pub fn push(&mut self, waker: TimeWaker) {
        // log::info!("push called");
        for slot in &mut self.0 {
            if slot.is_none() {
                *slot = Some(waker);
                return;
            }
        }

        log::error!("TIMER_WAKERS full: {:?}", self);

        panic!("TIMER_WAKERS full, increase MAX_SLEEPERS");
    }

    fn wake_expired(&mut self, now: u64) {
        for slot in &mut self.0 {
            if let Some(tw) = slot
                && now >= tw.target_tick
            {
                tw.waker.wake_by_ref();
                log::trace!("waking target {:?} by ref", tw);
                *slot = None;
            }
        }
    }
}

pub fn register_waker(sleep_ms: u64, waker: Waker) {
    without_interrupts(|| {
        let target_tick = get_ticks() + sleep_ms;
        let mut wakers = TIMER_WAKERS.lock();

        wakers.push(TimeWaker { target_tick, waker });
    });
}

/// Wake any expired tasks
pub fn wake_expired() {
    without_interrupts(|| {
        let now = get_ticks();
        let mut wakers = TIMER_WAKERS.lock();
        wakers.wake_expired(now);
    });
}

pub struct Instant {
    start_tick_ms: u64,
}

impl Instant {
    #[must_use]
    pub fn now() -> Instant {
        Instant {
            start_tick_ms: get_ticks(),
        }
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        Duration::from_millis(get_ticks() - self.start_tick_ms)
    }
}
