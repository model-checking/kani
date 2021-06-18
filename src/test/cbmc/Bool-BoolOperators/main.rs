// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
include!("../../rmc-prelude.rs");

fn main() {
    assert!(true);
    assert!(true || false);
    assert!(!false);

    let a = true;
    let b = false;
    let c = a || b;
    let d = c && a;
    assert!(d && true);
    assert!(!b && d);

    let e: bool = __nondet();
    assert!(e || !e);
}
