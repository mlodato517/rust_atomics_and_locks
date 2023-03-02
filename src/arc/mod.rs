use std::ops::Deref;
use std::ptr::NonNull;

use crate::sync::atomic::{self, AtomicUsize, Ordering};

#[cfg(test)]
mod tests;

// SAFETY: We can clone and Send `Arc<T>` to another thread while reading on the current thread so
// we require `T: Sync`. We also need `T: Send` because the last `Arc` that drops (dropping the
// inner data) could occur on another thread.
unsafe impl<T> Send for Arc<T> where T: Send + Sync {}
unsafe impl<T> Sync for Arc<T> where T: Send + Sync {}

pub struct Arc<T> {
    inner: NonNull<Inner<T>>,
}

struct Inner<T> {
    count: AtomicUsize,
    data: T,
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        let inner = self.data();

        // There is nothing to synchronize with here. Since we exist, we know the count isn't zero.
        // We _could_ be racing against another thread dropping an `Arc` but, worst case, that
        // drops the count to one. Since there is no code that must have "happened before" this, we
        // can use `Relaxed`.
        if inner.count.fetch_add(1, Ordering::Relaxed) == usize::MAX / 2 {
            eprintln!("Too many Arcs on the dance floor!");
            std::process::abort();
        }
        Self { inner: self.inner }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data().data
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        let inner = self.data();

        // We want to make sure every other drop "happens before" the drop to zero. So we want to
        // use `Acquire` for the final drop and `Release` for every other drop. `AcqRel` works but
        // is overkill.
        if inner.count.fetch_sub(1, Ordering::Release) == 1 {
            atomic::fence(Ordering::Acquire);
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
        let inner = NonNull::from(Box::leak(Box::new(Inner {
            count: AtomicUsize::new(1),
            data,
        })));
        Self { inner }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        let inner = arc.data();

        // When getting a general count, we don't need any synchronization so we can use `Relaxed`
        // ordering. Getting a 1, however, means we're going to exclusively acquire the data so we
        // must ensure any previous `Drop`s have been observed so we use an `Acquire` fence to
        // synchronize with the `Release` store in the `fetch_sub`.
        if inner.count.load(Ordering::Relaxed) == 1 {
            atomic::fence(Ordering::Acquire);

            // SAFETY: We've proven there is only one instance of us so there are no aliasing
            // borrows. This `&mut` borrow will last as long as we do so we can't be cloned and no
            // other `&mut` can be obtained.
            Some(unsafe { &mut arc.inner.as_mut().data })
        } else {
            None
        }
    }

    fn data(&self) -> &Inner<T> {
        // SAFETY: The pointer is properly aligned because `Box` ensured that on construction and
        // we never move the pointer. It is dereferenceable because `Box` ensured that the pointer
        // points to a single object contained within an allocation. The pointer points to an
        // initialized instance of `T` because we started with an initialized instance of `T` in
        // the constructer and haven't dropped the memory because we exist (so the count didn't
        // drop to zero in `Drop`). The returned lifetime is valid here because it will live as
        // long as we live and the underlying data won't be dropped while we're alive. Furthermore,
        // we'll only get shared references (except for `get_mut` which checks the count) so we
        // don't have to worry about aliasing with a `&mut`.
        unsafe { self.inner.as_ref() }
    }
}
