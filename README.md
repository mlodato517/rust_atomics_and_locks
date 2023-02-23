# Rust Atomics and Locks

This is my coding along with [this wonderful book][book]. In general, I'll try
to implement a chapter, update that implementation with the book's
implementation, and record my learnings in the README.

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
