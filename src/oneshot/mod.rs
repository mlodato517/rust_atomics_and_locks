use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::thread::Thread;

use crate::sync::atomic::{AtomicBool, Ordering};
use crate::thread;

#[cfg(test)]
mod tests;

// SAFETY: The `T` is only on one thread at a time. Either the sending thread or the receiving
// thread. If we want to send the `Sender` to another thread, then `T` needs to be `Send` so we can
// send it back to the receiving thread. If we want to send the `Receiver` to another thread, then
// `T` needs to be `Send` so we can send it to that receiving thread.
unsafe impl<T> Send for Sender<T> where T: Send {}
unsafe impl<T> Send for Receiver<T> where T: Send {}

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}
impl<T> Sender<T> {
    pub fn send(self, data: T) {
        if Arc::strong_count(&self.inner) == 1 {
            // The sender and receiver aren't `Clone` so this means the receiver has been dropped.
            // We'll do nothing and just drop the `T`.
        } else {
            // SAFETY: The receiving thread won't read this data until it sees `ready == true`.
            // Because it loads this with `Acquire` and we store with `Release` it means `ready ==
            // true` implies we're done writing. So we won't have concurrent access. Also, we can't
            // have concurrent writes because `send` takes `self` by value and drops it so this is
            // only called once.
            unsafe { (*self.inner.data.get()).write(data) };
            self.inner.ready.store(true, Ordering::Release);
            // TODO: This is a deadlock. There is no happens-before relationship here because
            // `waiting` and `ready` are different variables. I think this thread can see a `false`
            // here even if the other thread has already stored `true`, read `false`, and is
            // parked.
            if self.inner.waiting.load(Ordering::Acquire) {
                // SAFETY: Reading `waiting == true` with `Acquire` implies everything that
                // happened before the `Release` has happened. Which is to say, the `recv` thread
                // is already done writing to this variable. So it is safe to read from.
                unsafe { (*self.inner.thread.get()).as_ref().unwrap().unpark() };
            }
        }
    }
}
pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}
impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        if !self.inner.waiting.load(Ordering::Acquire) {
            // We were dropped before `recv` was called. We should drop the inner value if we have
            // it.
            // Huh, wait, this is another race. We could read false here and someone could call
            // `send` right after. Then we won't drop the data. This is really hard...
            if self.inner.ready.load(Ordering::Acquire) {
                // SAFETY: We read `ready == true` with `Acquire` ordering. So everything before
                // the `Release` has completed - namely the writing of this value. We're being
                // dropped so we won't do this twice.
                drop(unsafe { (*self.inner.data.get()).assume_init_read() });
            }
        }
    }
}
impl<T> Receiver<T> {
    pub fn recv(self) -> T {
        // SAFETY: The `send` thread won't read this until `waiting == true`. When it does read
        // `waiting == true`, everything before the `Release` store will have finished. That is the
        // writing of this value so we won't have concurrent access.
        unsafe { (*self.inner.thread.get()) = Some(std::thread::current()) };
        self.inner.waiting.store(true, Ordering::Release);
        while !self.inner.ready.load(Ordering::Acquire) {
            thread::park();
        }
        // SAFETY: We read `ready == true` with `Acquire` ordering. So everything before the
        // `Release` has completed - namely the writing of this value. Also, we consume and drop
        // `self` so we won't read this twice and generate two owned `T`s from the same data.
        unsafe { (*self.inner.data.get()).assume_init_read() }
    }
}
struct Inner<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
    thread: UnsafeCell<Option<Thread>>,
    waiting: AtomicBool,
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        data: UnsafeCell::new(MaybeUninit::uninit()),
        ready: AtomicBool::new(false),
        thread: UnsafeCell::new(None),
        waiting: AtomicBool::new(false),
    });

    (
        Sender {
            inner: Arc::clone(&inner),
        },
        Receiver { inner },
    )
}
