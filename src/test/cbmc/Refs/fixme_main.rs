// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// running with a unwind of 1 passes; running with unwind of two gets stuck in post-processing
// from arg_parser.rs in firecracker/src/utils/src

// warning: ignoring typecast
//   * type: struct_tag
//       * identifier: tag-std::alloc::Global
//   0: struct
//       * type: struct_tag
//           * identifier: tag-std::mem::ManuallyDrop<std::vec::Vec<&()>>
//       0: struct
//           * type: struct_tag
//               * identifier: tag-std::vec::Vec<&()>
//           0: struct
//               * type: struct_tag
//                   * identifier: tag-alloc::raw_vec::RawVec<&()>
//               0: struct
//                   * type: struct_tag
//                       * identifier: tag-std::alloc::Global
//               1: struct
//                   * type: struct_tag
//                       * identifier: tag-std::ptr::Unique<&()>
//                   0: struct
//                       * type: struct_tag
//                           * identifier: tag-std::marker::PhantomData<&()>
//                   1: constant
//                       * type: pointer
//                           * width: 64
//                           0: pointer
//                               * width: 64
//                               0: struct_tag
//                                   * identifier: tag-Unit
//                       * value: 8
//               2: constant
//                   * type: unsignedbv
//                       * #source_location:
//                         * file: <built-in-additions>
//                         * line: 1
//                         * working_directory: /Users/dsn/ws/RustToCBMC/src/RustToCBMC/rust-tests/cbmc-reg/Refs
//                       * width: 64
//                       * #typedef: __CPROVER_size_t
//                       * #c_type: unsigned_long_int
//                   * #source_location:
//                     * file: <built-in-additions>
//                     * line: 16
//                     * working_directory: /Users/dsn/ws/RustToCBMC/src/RustToCBMC/rust-tests/cbmc-reg/Refs
//                   * value: 0
//           1: constant
//               * type: unsignedbv
//                   * #source_location:
//                     * file: <built-in-additions>
//                     * line: 1
//                     * working_directory: /Users/dsn/ws/RustToCBMC/src/RustToCBMC/rust-tests/cbmc-reg/Refs
//                   * width: 64
//                   * #typedef: __CPROVER_size_t
//                   * #c_type: unsigned_long_int
//               * #source_location:
//                 * file: <built-in-additions>
//                 * line: 16
//                 * working_directory: /Users/dsn/ws/RustToCBMC/src/RustToCBMC/rust-tests/cbmc-reg/Refs
//               * value: 0

use std::collections::BTreeMap;

pub struct ArgParser<'a> {
    arguments: BTreeMap<&'a str, ()>,
}

impl<'a> ArgParser<'a> {
    fn format_arguments(&self) -> String {
        self.arguments
            .values()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|_arg| String::new())
            .collect::<Vec<_>>()
            .join("")
    }
}

fn main() {
    let a: ArgParser = ArgParser { arguments: BTreeMap::new() };
    a.format_arguments();
}
