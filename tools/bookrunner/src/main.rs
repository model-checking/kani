// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(extend_one)]
#![feature(rustc_private)]

mod bookrunner;
mod books;
mod litani;
mod util;

fn main() {
    books::generate_run();
}
