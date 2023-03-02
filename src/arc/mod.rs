use std::cell::UnsafeCell;
use std::ops::Deref;
use std::ptr::NonNull;

use crate::sync::atomic::{self, AtomicUsize, Ordering};

#[cfg(test)]
mod tests;

// SAFETY: We can clone and Send `Weak<T>` to another thread while reading on the current thread so
// we require `T: Sync`. We also need `T: Send` because the last `Weak` that drops (dropping the
// inner data) could occur on another thread.
unsafe impl<T> Send for Weak<T> where T: Send + Sync {}
unsafe impl<T> Sync for Weak<T> where T: Send + Sync {}

pub struct Arc<T> {
    weak: Weak<T>,
}

struct Inner<T> {
    arc_count: AtomicUsize,
    total_count: AtomicUsize,
    data: UnsafeCell<Option<T>>,
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        // There is nothing to synchronize with here. Since we exist, we know the count isn't zero.
        // We _could_ be racing against another thread dropping an `Arc` but, worst case, that
        // drops the count to one. Since there is no code that must have "happened before" this, we
        // can use `Relaxed`.
        let weak = self.weak.clone();
        if weak.data().arc_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            eprintln!("Too many Arcs on the dance floor!");
            std::process::abort();
        }
        Self { weak }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: The existance of an `Arc` means the data hasn't been dropped. The existance of a
        // `&Arc` means that no other `Arc` currently has `&mut T`.
        unsafe { (*self.weak.data().data.get()).as_ref().unwrap() }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        let inner = self.weak.data();

        // We want to make sure every other drop "happens before" the drop to zero. So we want to
        // use `Acquire` for the final drop and `Release` for every other drop. `AcqRel` works but
        // is overkill.
        if inner.arc_count.fetch_sub(1, Ordering::Release) == 1 {
            atomic::fence(Ordering::Acquire);
            // SAFETY: Only one thread should get to here. Since we've decremented the count to
            // zero we know there are no other Arcs trying to read this data and we can safely drop
            // it. Furthermore, we know that no `Weak`s will read a value of not-zero and try to
            // upgrade and read this data.
            unsafe {
                *self.weak.data().data.get() = None;
            }
        }
    }
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        Self {
            weak: Weak {
                inner: NonNull::from(Box::leak(Box::new(Inner {
                    arc_count: AtomicUsize::new(1),
                    total_count: AtomicUsize::new(1),
                    data: UnsafeCell::new(Some(data)),
                }))),
            },
        }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        let inner = arc.weak.data();

        // When getting a general count, we don't need any synchronization so we can use `Relaxed`
        // ordering. Getting a 1, however, means we're going to exclusively acquire the data so we
        // must ensure any previous `Drop`s have been observed so we use an `Acquire` fence to
        // synchronize with the `Release` store in the `fetch_sub`.
        //
        // It's important we check `total_count` here and not `arc_count`. If there was 1 Arc and 1
        // Weak then, when we got the mutable reference here, the Weak could upgrade to an Arc and
        // we'd have a problem.
        if inner.total_count.load(Ordering::Relaxed) == 1 {
            atomic::fence(Ordering::Acquire);

            // SAFETY: We've proven there is only one instance of us so there are no aliasing
            // borrows. This `&mut` borrow will last as long as we do so we can't be cloned and no
            // other `&mut` can be obtained.
            //
            // Technically we could remove the `Some` and `unwrap` but this seems semantically more
            // accurate.
            unsafe { Some(arc.weak.inner.as_mut().data.get_mut().as_mut().unwrap()) }
        } else {
            None
        }
    }

    pub fn downgrade(arc: &Self) -> Weak<T> {
        arc.weak.clone()
    }
}

pub struct Weak<T> {
    inner: NonNull<Inner<T>>,
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        if self.data().total_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }

        Self { inner: self.inner }
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        // Same logic as in Arc::drop
        if self.data().total_count.fetch_sub(1, Ordering::Release) == 1 {
            atomic::fence(Ordering::Acquire);

            // Now that all the Arcs are gone and we're dropping the last Weak, we can drop the
            // inner data.
            // SAFETY: All other owners have this data have been dropped.
            unsafe {
                drop(Box::from_raw(self.inner.as_ptr()));
            }
        }
    }
}

impl<T> Weak<T> {
    pub fn upgrade(&self) -> Option<Arc<T>> {
        let arc_count = &self.data().arc_count;

        // We want to:
        //   1. make sure there currently exists an `Arc`
        //   2. increase that count to account for the new `Arc` we're creating if the `Arc` wasn't
        //      dropped between when we read and when we tried to increase it
        //
        // We can't use a `fetch_add` here and check if what was returned was `> 0`. This is
        // because, if we add 1 to the `arc_count` and see that it used to be zero, another `Weak`
        // could also be trying to upgrade, see our phantom 1, and incorrectly succeed.
        //
        // I'm unsure on the success/failure ordering right now. I'm going to go with `Acquire`
        // because the book keeps using this informal-but-nice-mental-model language of like,
        // "We're releasing an Arc" or "We're acquiring the last Arc to drop" and here I think
        // we're "Acquiring the data to make a new Arc". Also, in terms of reordering, we don't
        // want the creation of the `Arc` to happen before we increment the count. But it isn't
        // obviously important here.
        //
        // Apparently it's okay for Relaxed on both sides - awesome.
        let mut count = arc_count.load(Ordering::Relaxed);
        loop {
            if count == 0 {
                return None;
            }
            assert!(count < usize::MAX);
            if let Err(e) =
                arc_count.compare_exchange(count, count + 1, Ordering::Relaxed, Ordering::Relaxed)
            {
                count = e;
                continue;
            }

            return Some(Arc { weak: self.clone() });
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
        // we'll only get shared references so we don't have to worry about aliasing with a `&mut`.
        unsafe { self.inner.as_ref() }
    }
}
