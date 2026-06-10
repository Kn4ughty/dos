use core::sync::atomic::AtomicU64;

pub mod pit;
pub mod rtc;

// 500 million years
pub static MS_COUNT: AtomicU64 = AtomicU64::new(0);

const MS_HZ: f64 = 1000.0;

pub fn init() {
    pit::set_interval(MS_HZ);
}
