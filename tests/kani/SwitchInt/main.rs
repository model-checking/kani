// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn doswitch_int() -> i32 {
    for i in [99].iter() {
        if *i == 99 {
            return 1;
        }
    }
    return 2;
}

fn doswitch_chars() -> i32 {
    for c in "a".chars() {
        if c == 'a' {
            return 1;
        }
    }
    return 2;
}

fn doswitch_bytes() -> i32 {
    for c in "a".bytes() {
        if c == ('a' as u8) {
            return 1;
        }
    }
    return 2;
}

#[kani::proof]
#[kani::unwind(2)]
fn main() {
    let v = doswitch_int();
    assert!(v == 1);
    let v = doswitch_chars();
    assert!(v == 1);
    let v = doswitch_bytes();
    assert!(v == 1);
}

// Check that Kani can codegen a SwitchInt that has no targets (only an otherwise)
// c.f. https://github.com/model-checking/kani/issues/4103
pub enum Reference {
    ByName { alias: String },
}

#[kani::proof]
fn check_nontrivial_drop() {
    let result: Reference = Reference::ByName { alias: "foo".into() };
    drop(result)
}
