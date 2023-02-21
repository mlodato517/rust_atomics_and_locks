use std::sync::Arc;

use super::SpinLock;

#[cfg(loom)]
#[test]
fn vague_loom_test() {
    loom::model(|| {
        let lock = Arc::new(SpinLock::new(String::from("hello")));
        let lock2 = Arc::clone(&lock);
        let lock3 = Arc::clone(&lock);
        let jh = loom::thread::spawn(move || {
            lock.lock().push('!');
        });
        let jh2 = loom::thread::spawn(move || {
            lock2.lock().push('!');
        });
        jh.join().unwrap();
        jh2.join().unwrap();

        assert_eq!(*lock3.lock(), "hello!!");
    });
}

#[cfg(not(loom))]
#[test]
fn vague_general_test() {
    let lock = Arc::new(SpinLock::new(String::from("hello")));
    let lock2 = Arc::clone(&lock);
    std::thread::scope(|s| {
        s.spawn(|| {
            lock2.lock().push('!');
        });
        s.spawn(|| {
            lock2.lock().push('!');
        });
    });

    assert_eq!(*lock.lock(), "hello!!");
}
