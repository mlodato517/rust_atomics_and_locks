#[cfg(loom)]
mod loom_tests {
    // This test panics and then aborts. Looking at the code it appears to be because there is no
    // active thread. Maybe a bug with parking?
    #[test]
    fn vague_loom_test() {
        loom::model(|| {
            let (tx, rx) = crate::oneshot::channel();
            loom::thread::spawn(move || {
                tx.send(String::from("hello"));
            });
            let value = rx.recv();

            assert_eq!(value, "hello");
        });
    }
}

#[cfg(not(loom))]
mod non_loom_tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn vague_general_test() {
        let (tx, rx) = crate::oneshot::channel();
        std::thread::spawn(move || {
            tx.send(String::from("hello"));
        });
        let value = rx.recv();

        assert_eq!(value, "hello");
    }

    #[test]
    fn drop_no_recv() {
        struct Wrapper(Arc<AtomicBool>);
        impl Drop for Wrapper {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }
        let data = Arc::new(AtomicBool::new(false));
        let wrapper = Wrapper(Arc::clone(&data));

        let (tx, rx) = crate::oneshot::channel();
        tx.send(wrapper);
        drop(rx);

        let dropped = data.load(Ordering::Relaxed);
        assert!(dropped);
    }

    #[test]
    fn drop_before_send() {
        struct Wrapper(Arc<AtomicBool>);
        impl Drop for Wrapper {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }
        let data = Arc::new(AtomicBool::new(false));
        let wrapper = Wrapper(Arc::clone(&data));

        let (tx, rx) = crate::oneshot::channel();
        drop(rx);
        tx.send(wrapper);

        let dropped = data.load(Ordering::Relaxed);
        assert!(dropped);
    }
}
