use super::Arc;

#[cfg(loom)]
mod loom_tests {
    use super::*;

    // Note that this loom test doesn't actually catch any data races. But MIRI somehow does. Amazing.
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

    #[test]
    fn vague_get_mut_test() {
        loom::model(|| {
            let data = String::from("hello world");
            let arc = Arc::new(data);
            let mut arc2 = Arc::clone(&arc);
            let (tx, rx) = loom::sync::mpsc::channel();
            let jh = loom::thread::spawn(move || {
                let _s: &str = &arc;
                let _ = rx.recv();
            });
            assert!(Arc::get_mut(&mut arc2).is_none());
            let _ = tx.send(());
            jh.join().unwrap();

            assert!(Arc::get_mut(&mut arc2).is_some());
        });
    }

    #[test]
    fn vague_weak_race_test() {
        loom::model(|| {
            let arc = Arc::new(0);
            let weak = Arc::downgrade(&arc);
            let weak2 = weak.clone();

            drop(arc);
            let jh = loom::thread::spawn(move || weak2.upgrade().is_some());

            // Just asserting here that upgrading racing with dropping works according to MIRI
            let upgraded = weak.upgrade().is_some();
            let other_upgraded = jh.join().unwrap();

            assert!(!upgraded && !other_upgraded);
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
    fn vague_get_mut_test() {
        let data = String::from("hello world");
        let arc = Arc::new(data);
        let mut arc2 = Arc::clone(&arc);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let jh = std::thread::spawn(move || {
            let _s: &str = &arc;
            let _ = rx.recv();
        });
        assert!(Arc::get_mut(&mut arc2).is_none());
        let _ = tx.send(());
        jh.join().unwrap();

        assert!(Arc::get_mut(&mut arc2).is_some());

        let weak = Arc::downgrade(&arc2);
        assert!(Arc::get_mut(&mut arc2).is_none());

        drop(weak);
        assert!(Arc::get_mut(&mut arc2).is_some());
    }

    #[test]
    fn vague_weak_drop_test() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

        struct DetectDrop;

        impl Drop for DetectDrop {
            fn drop(&mut self) {
                NUM_DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }

        let arc = Arc::new(DetectDrop);
        let weak = Arc::downgrade(&arc);

        assert_eq!(NUM_DROPS.load(Ordering::Relaxed), 0);
        assert!(weak.upgrade().is_some());

        drop(arc);

        assert_eq!(NUM_DROPS.load(Ordering::Relaxed), 1);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn vague_weak_race_test() {
        let arc = Arc::new(0);
        let weak = Arc::downgrade(&arc);
        let weak2 = weak.clone();

        std::thread::spawn(|| drop(arc));
        std::thread::spawn(move || {
            let _ = weak2.upgrade();
        });

        // Just asserting here that upgrading racing with dropping works according to MIRI
        weak.upgrade();
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
