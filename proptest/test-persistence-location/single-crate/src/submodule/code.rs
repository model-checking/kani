#![allow(unreachable_code)]
mod test {
    proptest! {
        #[test]
        fn the_test(_ in 0u32..100) { panic!() }
    }
}
