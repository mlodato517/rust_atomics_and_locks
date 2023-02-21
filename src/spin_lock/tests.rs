use super::SpinLock;

#[cfg(loom)]
mod loom_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn vague_loom_test() {
        loom::model(|| {
            let lock = Arc::new(SpinLock::new(String::from("hello")));
            let lock2 = Arc::clone(&lock);
            let jh = loom::thread::spawn(move || lock2.lock().push('!'));

            {
                lock.lock().push('!');
            }

            jh.join().unwrap();

            assert_eq!(*lock.lock(), "hello!!");
        });
    }
}

#[cfg(not(loom))]
mod non_loom_tests {
    use super::*;

    #[test]
    fn vague_general_test() {
        let lock = SpinLock::new(String::from("hello"));
        std::thread::scope(|s| {
            s.spawn(|| lock.lock().push('!'));
            s.spawn(|| lock.lock().push('!'));
        });

        assert_eq!(*lock.lock(), "hello!!");
    }

    #[test]
    fn test_from_book() {
        let x = SpinLock::new(Vec::new());
        std::thread::scope(|s| {
            s.spawn(|| x.lock().push(1));
            s.spawn(|| {
                let mut g = x.lock();
                g.push(2);
                g.push(2);
            });
        });
        let g = x.lock();
        assert!(g.as_slice() == [1, 2, 2] || g.as_slice() == [2, 2, 1]);
    }
}
