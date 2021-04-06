// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
enum Dummy {
    Dumb,
}

fn main() {
    // invoke replace on a zero-sized type
    let mut value: Dummy = Dummy::Dumb;
    let dst: &mut Dummy = &mut value;
    let src = Dummy::Dumb;
    core::mem::replace(dst, src);

    // invoke replace on the unit type
    let mut value2 = ();
    let dst2 = &mut value2;
    let src2 = ();
    core::mem::replace(dst2, src2);
}
