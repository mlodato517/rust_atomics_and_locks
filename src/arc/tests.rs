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
}
