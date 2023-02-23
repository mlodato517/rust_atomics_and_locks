#[cfg(loom)]
pub(crate) use loom::thread::{current, park, Thread};

#[cfg(not(loom))]
pub(crate) use std::thread::{current, park, Thread};
