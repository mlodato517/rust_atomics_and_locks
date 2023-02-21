pub(crate) mod atomic {
    #[cfg(loom)]
    pub(crate) use loom::sync::atomic::{AtomicBool, Ordering};

    #[cfg(not(loom))]
    pub(crate) use std::sync::atomic::{AtomicBool, Ordering};
}
