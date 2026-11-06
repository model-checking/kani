// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A constant must not share storage with a static whose initializer has the
//! same bytes. Storing to the static must not change the constant.
//! Several statics are used because codegen order follows symbol hashes.

use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

#[derive(Clone, Copy)]
enum Mode {
    A,
    B,
}

trait HasMode {
    const MODE: Mode;
}

struct Small;
impl HasMode for Small {
    const MODE: Mode = Mode::A;
}

#[inline(never)]
fn mode_id(m: Mode) -> u32 {
    match m {
        Mode::A => 1,
        Mode::B => 2,
    }
}

fn generic_mode<T: HasMode>() -> u32 {
    mode_id(T::MODE)
}

macro_rules! flags {
    ($($s:ident)*) => {
        $(static $s: AtomicBool = AtomicBool::new(false);)*
        fn set_flags() { $($s.store(true, SeqCst);)* }
    };
}
flags!(S0 S1 S2 S3 S4 S5 S6 S7 S8 S9 S10 S11 S12 S13 S14 S15);

#[kani::proof]
fn check_const_unchanged_by_static_store() {
    set_flags();
    assert_eq!(generic_mode::<Small>(), 1);
}
