use std::cell::UnsafeCell;
use std::mem::MaybeUninit;

use crate::sync::atomic::{AtomicBool, Ordering};
use crate::sync::Arc;
use crate::thread::{self, Thread};

#[cfg(test)]
mod tests;

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
    receiving_thread: Thread,
}
impl<T> Sender<T> {
    pub fn send(self, data: T) {
        // SAFETY: The receiving thread won't read this data until it sees `ready == true`.
        // Because it loads this with `Acquire` and we store with `Release` it means `ready ==
        // true` implies we're done writing. So we won't have concurrent access. Also, we can't
        // have concurrent writes because `send` takes `self` by value and drops it so this is
        // only called once.
        unsafe { (*self.inner.data.get()).write(data) };
        self.inner.ready.store(true, Ordering::Release);
        self.receiving_thread.unpark();
    }
}
pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
    // The book handles thread parking by preventing sending the Receiver.
    _no_send: std::marker::PhantomData<*const ()>,
}
impl<T> Receiver<T> {
    pub fn recv(self) -> T {
        while !self.inner.ready.swap(false, Ordering::Acquire) {
            thread::park();
        }
        // SAFETY: We read `ready == true` with `Acquire` ordering. So everything before the
        // `Release` has completed - namely the writing of this value. Also, we consume and drop
        // `self` so we won't read this twice and generate two owned `T`s from the same data.
        // It's important to note that we swapped back in `false` for ready - this ensures we don't
        // re-drop this value when the last `Inner` drops.
        unsafe { (*self.inner.data.get()).assume_init_read() }
    }
}
struct Inner<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}
// SAFETY: We want to share `Inner` between the sending and receiving thread so we want `Sync`.
// The `T` is only on one thread at a time. If we want to send the `Sender` to another
// thread, then `T` needs to be `Send` so we can send it back to the receiving thread.
unsafe impl<T> Sync for Inner<T> where T: Send {}

impl<T> Drop for Inner<T> {
    fn drop(&mut self) {
        // NB We use `get_mut_raw` here so we can share this with `loom`.
        if self.ready.get_mut_raw() {
            // SAFETY: We are the last `Inner` and read `ready == true`. So the `Sender` has sent
            // and the `Receiver` hasn't read. No other value is around and able to read/write this
            // data so it's safe to access.
            unsafe { (*self.data.get()).assume_init_drop() };
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        data: UnsafeCell::new(MaybeUninit::uninit()),
        ready: AtomicBool::new(false),
    });

    (
        Sender {
            inner: Arc::clone(&inner),
            receiving_thread: thread::current(),
        },
        Receiver {
            inner,
            _no_send: std::marker::PhantomData,
        },
    )
}
