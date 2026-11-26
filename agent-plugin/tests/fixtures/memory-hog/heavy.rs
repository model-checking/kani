// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// A harness that needs far more memory than any cap the memory guard test sets: 65536 symbolic
// 32-bit multiplications.
#[kani::proof]
#[kani::unwind(300)]
fn heavy() {
    let a: [u32; 256] = kani::any();
    let mut s = 0u32;
    for i in 0..256 {
        for j in 0..256 {
            s = s.wrapping_add(a[i].wrapping_mul(a[j]));
        }
    }
    assert!(s != 0xdeadbeef);
}
