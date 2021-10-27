// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn from_str_test() {
        assert_eq!(Vec::from("123"), rmc_vec![b'1', b'2', b'3']);
    }

    from_str_test()
}
