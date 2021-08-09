// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(command_access)]

mod dashboard;
mod litani;
mod reference;
mod util;

fn main() {
    reference::display_reference_dashboard();
}
