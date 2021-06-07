// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Causes a panic in transmute unless i reset the global var count
fn test2() {
    let s = "foo".to_string();
    assert!(s.chars().nth(1) == Some('o'));
}

/// Runs forever
fn test3() {
    let s: &str = &("f".to_string() + "o");
    assert!(s.len() == 2);
}

// Used to trigger a fault in exact_div
// Now triggers a fault in codegen_place
fn test4() {
    let s: &str = "1";
    let ptr: *const u8 = s.as_ptr();

    unsafe {
        assert!(!ptr.is_null());
        //assert!(*ptr.offset(1) as char == '2'); // u8 to char not handled yet
    }
}
fn main() {
    //  test2();
    //  test3();
    test4();
}
