#[cfg(not(loom))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(loom))]
use std::sync::Arc;

#[cfg(loom)]
#[test]
fn vague_loom_test() {
    loom::model(|| {
        let (tx, rx) = super::channel();
        loom::thread::spawn(move || {
            tx.send(String::from("hello"));
        });
        let value = rx.recv();

        assert_eq!(value, "hello");
    });
}

#[cfg(not(loom))]
#[test]
fn vague_general_test() {
    let (tx, rx) = super::channel();
    std::thread::spawn(move || {
        tx.send(String::from("hello"));
    });
    let value = rx.recv();

    assert_eq!(value, "hello");
}

#[cfg(not(loom))]
#[test]
fn drop_no_recv() {
    struct Wrapper(Arc<AtomicBool>);
    impl Drop for Wrapper {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let data = Arc::new(AtomicBool::new(false));
    let wrapper = Wrapper(Arc::clone(&data));

    let (tx, rx) = super::channel();
    tx.send(wrapper);
    drop(rx);

    let dropped = data.load(Ordering::SeqCst);
    assert!(dropped);
}

#[cfg(not(loom))]
#[test]
fn drop_before_send() {
    struct Wrapper(Arc<AtomicBool>);
    impl Drop for Wrapper {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let data = Arc::new(AtomicBool::new(false));
    let wrapper = Wrapper(Arc::clone(&data));

    let (tx, rx) = super::channel();
    drop(rx);
    tx.send(wrapper);

    let dropped = data.load(Ordering::SeqCst);
    assert!(dropped);
}
