#[cfg(loom)]
pub(crate) use loom::thread::park;

#[cfg(not(loom))]
pub(crate) use std::thread::park;
