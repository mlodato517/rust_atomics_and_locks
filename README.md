# Rust Atomics and Locks

This is my coding along with [this wonderful book][book]. In general, I'll try
to implement a chapter, update that implementation with the book's
implementation, and record my learnings in the README.

## Chapter 4 - Spin Lock

Unfortunately, I read this chapter before attempting an implementation. Luckily,
I was still given the opportunity to learn a lot about `Send`/`Sync`! Most of
the details are summarized in [this issue][send-sync-issue] but I'll summarize
here.

I originally thought that `Send` was for transferring ownership of a value
to another thread. So I thought this was specifically relating to sending
a `T`. However:

- I didn't notice the fact that, if you send `&mut T`, the receiving thread can
  use `std::mem::swap` (or similar) to get an owned `T`. So `Send` is
  definitely required for sending `&mut T`.
- Since `Send` was for `T` and I hadn't really thought about `&mut
  T`, I assumed that you needed `Sync` to send non-`T`. That's
  untrue - `Sync` is _only_ required if you need concurrent access
  to `&T` from multiple threads. The reason why `&T: Send` implies
  `Sync` is because `&T` is copy - if you're allowed to send `&T`
  to one thread then you're allowed to make copies and send to many
  threads. However, if you're in a situation where you have
  exclusive access (e.g. `Mutex`, spin lock), then you don't need
  `Sync`.
- You need `Send` _even if you only access `&T`_. If, for example, we remove
  the `DerefMut` implementation on our `SpinLockGuard`, we'd _still_ need to
  require `T: Send` to `impl Sync for SpinLock`. When we talk about `Send` and
  "sending values to another thread" that doesn't mean passing ownership - that
  means doing _anything_ with `T`, `&T`, or `&mut T` on a thread that the
  original `T` wasn't initialized on. Some more details in [this wonderful
  StackOverflow answer][send-sync-stackoverflow].

## Chapter 5 - Channels

As with Chapter 4, I'd unfortunately already skimmed this chapter before
attempting an implementation. Luckily, I did such a bad job at reading that
I got plenty of stuff wrong which leaves room for learning!

- Preventing sending the `Receiver` is a much easier way to avoid
  deadlocks (though I'll need to investigate `std`/`crossbeam` to
  see how this is handled in the wild).
- By dropping unsent messages in the `Drop` implementation of `Inner` we dodge
  some racy logic involving an extra `waiting` variable and avoid some kludgy
  `Arc::strong_count`ing.
- Implementing `Sync` on `Inner` and then letting autoimplementations bubble up
  to the public types is better. This is also useful in case you make `Inner`
  public later (as we would if we were avoiding the inner allocation).
- `AtomicBool::get_mut()` is a neat API for when you have `&mut
  self`.

[book]: https://marabos.nl/atomics/
[send-sync-issue]: https://github.com/mlodato517/rust_atomics_and_locks/issues/1
[send-sync-stackoverflow]: https://stackoverflow.com/a/68708557
