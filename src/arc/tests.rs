use super::Arc;

// Note that this loom test doesn't actually catch any data races. But MIRI somehow does. Amazing.
#[cfg(loom)]
mod loom_tests {
    use super::*;

    #[test]
    fn vague_loom_test() {
        loom::model(|| {
            let data = String::from("hello world");
            let arc = Arc::new(data);
            let arc2 = Arc::clone(&arc);
            loom::thread::spawn(move || {
                let _s: &str = &arc;
            });

            assert_eq!(&*arc2, "hello world");
        });
    }
}

#[cfg(not(loom))]
mod non_loom_tests {
    use super::*;

    #[test]
    fn vague_general_test() {
        let data = String::from("hello world");
        let arc = Arc::new(data);
        let arc2 = Arc::clone(&arc);
        std::thread::spawn(move || {
            let _s: &str = &arc;
        });

        assert_eq!(&*arc2, "hello world");
    }

    #[test]
    fn test_from_book() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

        struct DetectDrop;

        impl Drop for DetectDrop {
            fn drop(&mut self) {
                NUM_DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Create two Arcs sharing an object containing a string
        // and a DetectDrop, to detect when it's dropped.
        let x = Arc::new(("hello", DetectDrop));
        let y = x.clone();

        // Send x to another thread, and use it there.
        let t = std::thread::spawn(move || {
            assert_eq!(x.0, "hello");
        });

        // In parallel, y should still be usable here.
        assert_eq!(y.0, "hello");

        // Wait for the thread to finish.
        t.join().unwrap();

        // One Arc, x, should be dropped by now.
        // We still have y, so the object shouldn't have been dropped yet.
        assert_eq!(NUM_DROPS.load(Ordering::Relaxed), 0);

        // Drop the remaining `Arc`.
        drop(y);

        // Now that `y` is dropped too,
        // the object should've been dropped.
        assert_eq!(NUM_DROPS.load(Ordering::Relaxed), 1);
    }
}
