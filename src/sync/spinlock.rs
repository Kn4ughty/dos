use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

#[derive(Debug)]
pub struct Mutex<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

#[derive(Debug)]
pub struct MutexGuard<'a, T> {
    lock: &'a AtomicBool,
    data: &'a mut T,
}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Mutex<T> {
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

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(MutexGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }
}

impl<T> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T> core::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<T> Drop for MutexGuard<'_, T> {
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

    #[test_case]
    fn try_lock() {
        let m = Mutex::new(42);
        let val1 = m.try_lock();
        assert_eq!(val1.as_ref().map(|r| **r), Some(42));

        let val2 = m.try_lock();
        assert_eq!(val2.as_ref().map(|r| **r), None);
        drop(val1);
        let val2 = m.try_lock();
        assert_eq!(val2.as_ref().map(|r| **r), Some(42));
    }
}
