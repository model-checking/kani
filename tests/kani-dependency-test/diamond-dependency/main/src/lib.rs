// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use dependency1;
use dependency2;

#[no_mangle]
fn harness() {
    assert!(dependency1::delegate_get_int() == 0);
    assert!(dependency2::delegate_get_int() == 1);

    assert!(dependency1::delegate_use_struct() == 3);
    assert!(dependency2::delegate_use_struct() == 1);
}
