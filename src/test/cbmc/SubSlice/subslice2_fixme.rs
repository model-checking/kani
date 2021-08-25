// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// At present, this test fails with a CBMC invariant violation:
//
// Invariant check failed
// File: ../src/solvers/flattening/boolbv_add_sub.cpp:68 function: convert_add_sub
// Condition: it->type() == type
// Reason: add/sub with mixed types:
// +
//   * type: signedbv
//       * width: 64
//       * #c_type: signed_long_int
//   0: constant
//       * type: signedbv
//           * width: 64
//           * #c_type: signed_long_int
//       * value: 1
//   1: constant
//       * type: unsignedbv
//           * #source_location:
//             * file: <built-in-additions>
//             * line: 1
//             * working_directory: /home/ubuntu/rmc-rebase
//           * width: 64
//           * #typedef: __CPROVER_size_t
//           * #c_type: unsigned_long_int
//       * value: 1
//
// Full support for subslice projection to be added in
// https://github.com/model-checking/rmc/issues/357

fn main() {
    let arr = [1, 2, 3];
    // s is a slice (&[i32])
    let [s @ ..] = &arr[1..];
    assert!(s[0] == 2);
    assert!(s[1] == 3);
}
