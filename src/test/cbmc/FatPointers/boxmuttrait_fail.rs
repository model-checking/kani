// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

use std::io::{sink, Write};

fn main() {
    let mut log: Box<dyn Write + Send> = Box::new(sink());
    let dest: Box<dyn Write + Send> = Box::new(log.as_mut());

    let mut log2: Box<dyn Write + Send> = Box::new(sink());
    let buffer = vec![1, 2, 3, 5, 8];
    let num_bytes = log2.write(&buffer).unwrap();
    assert!(num_bytes == 8); // Should be == 5
}
