use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

pub struct Mutex<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

pub struct MutexGuard<'a, T> {
    lock: &'a AtomicBool,
    data: &'a mut T,
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Mutex<T> {
        Mutex {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    fn obtain_lock(&self) {
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.lock.load(Ordering::Relaxed) {
                spin_loop();
            }
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.obtain_lock();
        MutexGuard {
            lock: &self.lock,
            // This is safe since we have verified that no one else is using the data.
            data: unsafe { &mut *self.data.get() },
        }
    }

    // pub fn inner(&self) -> T {
    //     // if self.locked.
    // }
}

impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn basic() {
        let m = Mutex::new(23);
        let val = m.lock();
        assert_eq!(23, *val);
        drop(val);
    }
}
