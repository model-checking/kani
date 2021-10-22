// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(extend_one)]
#![feature(rustc_private)]

mod books;
mod dashboard;
mod litani;
mod util;

fn main() {
    books::generate_dashboard();
}
