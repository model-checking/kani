// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn from_str_test() {
        assert_eq!(Vec::from("123"), kani_vec![b'1', b'2', b'3']);
    }

    from_str_test()
}
