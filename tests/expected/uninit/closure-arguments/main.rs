// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! A closure call passes its arguments as one tuple (`Fn::call(&c, (u, x))`), while the closure body
//! receives them as separate parameters. The state of a union argument must reach the parameter
//! it is passed as, whatever its position. A call through `FnOnce::call_once`'s shim is reported
//! as unsupported.

#[derive(Clone, Copy)]
union U {
    a: u8,
    b: u32,
}

#[kani::proof]
fn union_first_initialized() {
    let c = |u: U, x: u8| (unsafe { u.b }) + x as u32;
    let _ = c(U { b: 1 }, 2);
}

#[kani::proof]
fn union_second_initialized() {
    let c = |x: u8, u: U| (unsafe { u.b }) + x as u32;
    let _ = c(2, U { b: 1 });
}

#[kani::proof]
fn union_first_uninit_read() {
    let c = |u: U, x: u8| (unsafe { u.b }) + x as u32;
    let _ = c(U { a: 1 }, 2);
}

#[kani::proof]
fn union_second_uninit_read() {
    let c = |x: u8, u: U| (unsafe { u.b }) + x as u32;
    let _ = c(2, U { a: 1 });
}

#[kani::proof]
fn fn_once_closure() {
    let c = move |u: U| unsafe { u.b };
    fn call<F: FnOnce(U) -> u32>(f: F, u: U) -> u32 {
        f(u)
    }
    let _ = call(c, U { b: 3 });
}
