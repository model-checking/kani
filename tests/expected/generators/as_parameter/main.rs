// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that includes a generator as a parameter to a function
// Codegen should succeed, but verification should fail (codegen_unimplemented)

#![feature(generators, generator_trait)]

use std::ops::Generator;

fn foo<T>(g: T)
where
    T: Generator,
{
    let _ = g;
}

#[kani::proof]
fn main() {
    foo(|| {
        yield 1;
        return 2;
    });
}
