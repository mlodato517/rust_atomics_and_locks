use std::ops::Deref;
use std::ptr::NonNull;

use crate::sync::atomic::{AtomicU32, Ordering};

#[cfg(test)]
mod tests;

// SAFETY: We can clone and Send `Arc<T>` to another thread while reading on the current thread so
// we require `T: Sync`.
unsafe impl<T> Send for Arc<T> where T: Sync {}
unsafe impl<T> Sync for Arc<T> where T: Sync {}

pub struct Arc<T> {
    inner: NonNull<Inner<T>>,
}

struct Inner<T> {
    count: AtomicU32,
    data: T,
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        // SAFETY: The pointer is properly aligned because `Box` ensured that on construction and
        // we never move the pointer. It is dereferenceable because `Box` ensured that the pointer
        // points to a single object contained within an allocation. The pointer points to an
        // initialized instance of `T` because we started with an initialized instance of `T` in
        // the constructer and haven't dropped the memory because we exist (so the count didn't
        // drop to zero in `Drop`). The returned lifetime is valid here because it will live as
        // long as we live and the underlying data won't be dropped while we're alive. Furthermore,
        // we'll only get shared references so we don't have to worry about aliasing with a `&mut`.
        let inner = unsafe { self.inner.as_ref() };

        // There is nothing to synchronize with here. Since we exist, we know the count isn't zero.
        // We _could_ be racing against another thread dropping an `Arc` but, worst case, that
        // drops the count to one. Since there is no code that must have "happened before" this, we
        // can use `Relaxed`.
        if inner.count.fetch_add(1, Ordering::Relaxed) == u32::MAX {
            eprintln!("Too many Arcs on the dance floor!");
            std::process::abort();
        }
        Self { inner: self.inner }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: See the note in `impl Clone`
        let inner = unsafe { self.inner.as_ref() };
        &inner.data
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        // SAFETY: See the note in `impl Clone`
        let inner = unsafe { self.inner.as_ref() };

        // I think here we're synchronizing to ensure that reading a value of not-zero _guarantees_
        // the data hasn't been dropped. I imagine that means we should decrement here with
        // `Acquire`. That way:
        //   1. reading a value of 0 means that the data's already been dropped (i.e. don't drop it
        //      twice or upgrade your `Weak`)
        //   2. reading a value of not-zero means that this thread has already chosen to not drop
        //      the data so you are safe to either drop the data or upgrade your `Weak` reference
        //
        // `fetch_sub` with `Acquire` ordering means that we store with `Relaxed` ordering which
        // doesn't sound right so maybe we need `AcqRel` in both places? When it comes to the naive
        // "compiler re-ordering" mental model, `Acquire` should be enough (later code shouldn't be
        // re-ordered before this store) so maybe we want to `fetch_sub` with `Release` and then
        // use an `Acquire` fence. That ensures that a thread dropping the data synchronizes with
        // the next thread attempting to read but I'm pretty unsure.
        if inner.count.fetch_sub(1, Ordering::AcqRel) == 1 {
            // SAFETY: Only one thread should get to here. Since we've decremented the count to
            // zero we know there are no other Arcs trying to read this data and we can safely read
            // it. Furthermore, we know that no Weak's will read a value of not-zero and try to
            // upgrade and read this data.
            drop(unsafe { Box::from_raw(self.inner.as_ptr()) })
        }
    }
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        // SAFETY: The requirement is that the pointer is non-null. `Box` guarantees that.
        let inner = unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(Inner {
                count: AtomicU32::new(1),
                data,
            })))
        };
        Self { inner }
    }
}
