#[cfg(loom)]
pub(crate) use loom::hint::spin_loop;

#[cfg(not(loom))]
pub(crate) use std::hint::spin_loop;
