// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let name = String::from("Mark");
    let name_str = name.as_str();
    assert!(name_str.len() > 0);
}
