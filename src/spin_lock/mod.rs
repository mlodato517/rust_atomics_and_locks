use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

use crate::hint;
use crate::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
mod tests;

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}
impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: The existence of this guard means we own the data. Because we have a `&self`, no
        // one else has a `&mut self`, so no one has a `&mut Self::Target`, so creating a
        // `&Self::Target` is safe.
        unsafe { &*self.lock.data.get() }
    }
}
impl<'a, T> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: The existence of this guard means we own the data. Because we have a `&mut
        // self`, no one else has a reference to `self`, so no one has a reference to
        // `Self::Target`, so creating a `&mut Self::Target` is safe.
        unsafe { &mut *self.lock.data.get() }
    }
}

// SAFETY: If we're sending the lock then we own it - i.e. there are no outstanding guards.
// In that case, because `UnsafeCell impl Send`, we can also be sent. The lock may be dropped by
// the other thread which is okay because there are no outstanding borrows.
unsafe impl<T> Send for SpinLock<T> where T: Send {}

// SAFETY: If we're sharing the lock then the other thread can't get a `T` - only a `&/&mut T` so
// we just need `T` to be `Sync`. The locking mechanism ensures that two shared versions don't get
// mutable access concurrently.
unsafe impl<T> Sync for SpinLock<T> where T: Sync {}

pub struct SpinLock<T> {
    data: UnsafeCell<T>,
    locked: AtomicBool,
}
impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            locked: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<T> {
        // ORDERING: Acquiring a `false` here means that the `SpinLockGuard::drop` that stored it
        // (with `Release` ordering), and everything before that, has already happened. That means
        // that the previous owner has dropped the lock guard and we're safe to acquire a new one.
        // We atomically swap in a `true` so, if two threads are racing to lock, only one succeeds.
        while self.locked.swap(true, Ordering::Acquire) {
            hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }

    pub fn into_inner(this: Self) -> T {
        this.data.into_inner()
    }
}
