// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(command_access)]

mod books;
mod dashboard;
mod litani;
mod util;

fn main() {
    books::generate_dashboard();
}
