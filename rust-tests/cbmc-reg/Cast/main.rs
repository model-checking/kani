// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::cmp::Ordering;
pub enum Level {
    Error,
}

pub fn main() {
    let left = Level::Error;
    assert!((left as u8).cmp(&0) == Ordering::Equal);
}
