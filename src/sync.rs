#[cfg(loom)]
pub(crate) mod atomic {
    use std::ops::Deref;

    pub(crate) use loom::sync::atomic::AtomicU32;
    pub(crate) use loom::sync::atomic::Ordering;

    pub(crate) struct AtomicBool(loom::sync::atomic::AtomicBool);
    impl Deref for AtomicBool {
        type Target = loom::sync::atomic::AtomicBool;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl AtomicBool {
        pub(crate) fn new(val: bool) -> Self {
            Self(loom::sync::atomic::AtomicBool::new(val))
        }

        pub(crate) fn get_mut_raw(&mut self) -> bool {
            // SAFETY: We have `&mut` so definitely no other threads have this
            unsafe { self.0.unsync_load() }
        }
    }
}
#[cfg(loom)]
pub(crate) use loom::sync::Arc;

#[cfg(not(loom))]
pub(crate) mod atomic {
    use std::ops::Deref;

    pub(crate) use std::sync::atomic::AtomicU32;
    pub(crate) use std::sync::atomic::Ordering;

    pub(crate) struct AtomicBool(std::sync::atomic::AtomicBool);
    impl Deref for AtomicBool {
        type Target = std::sync::atomic::AtomicBool;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl AtomicBool {
        pub(crate) fn new(val: bool) -> Self {
            Self(std::sync::atomic::AtomicBool::new(val))
        }

        pub(crate) fn get_mut_raw(&mut self) -> bool {
            *self.0.get_mut()
        }
    }
}
#[cfg(not(loom))]
pub(crate) use std::sync::Arc;
