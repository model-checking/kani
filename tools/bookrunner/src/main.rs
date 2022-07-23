// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(extend_one)]
#![feature(rustc_private)]
#![allow(clippy::too_many_arguments, clippy::redundant_clone, clippy::len_zero)]

mod bookrunner;
mod books;
mod litani;
mod util;

fn main() {
    books::generate_run();
}
