// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --debug --harness ovoa --gen-exec-trace

#[kani::proof]
fn ovoa() {
    let value1: u32 = kani::any();
    assert!(value1 != 136);
}

#[derive(Copy, Clone)]
pub struct SOvoa {
    value1: u32,
}

unsafe impl kani::Invariant for SOvoa {
    fn is_valid(&self) -> bool {
        true
    }
}

#[kani::proof]
fn struct_ovoa() {
    let sovoa = kani::any::<SOvoa>();
    assert!(sovoa.value1 != 136);
}

#[kani::proof]
fn ovma() {
    let value1: u32 = kani::any();
    assert!(value1 != 136);
    assert!(value1 != 137);
}

#[kani::proof]
fn mvoa() {
    let value1: u8 = kani::any();
    let value2: u16 = kani::any();
    let value3: u32 = kani::any();
    let value4: u64 = kani::any();
    assert!(
        value1 != 136 &&
        value2 != 137 &&
        value3 != 138 &&
        value4 != 139
    );
}

#[kani::proof]
fn mvma() {
    let value1: u8 = kani::any();
    let value2: u16 = kani::any();
    let value3: u32 = kani::any();
    let value4: u64 = kani::any();
    assert!(value1 != 136);
    assert!(value2 != 137);
    assert!(value3 != 138);
    assert!(value4 != 139);
}

#[derive(Copy, Clone)]
pub struct Rating {
    value1: u8,
    value2: u16,
    value3: u32,
    value4: u64,
}

unsafe impl kani::Invariant for Rating {
    fn is_valid(&self) -> bool {
        true
    }
}

#[kani::proof]
fn struct_mvma() {
    let rating = kani::any::<Rating>();
    assert!(rating.value1 != 136);
    assert!(rating.value2 != 137);
    assert!(rating.value3 != 138);
    assert!(rating.value4 != 139);
}
