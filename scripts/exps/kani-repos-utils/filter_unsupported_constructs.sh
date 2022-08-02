#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Filter lines with unsupported constructs.
# These appear in `stderr` after a warning as follows:
#
# ```
# warning: Found the following unsupported constructs:
#             - 'simd_eq' intrinsic (1)
#             - Rvalue::ThreadLocalRef (2)
# ```

if [[ $# -ne 1 ]]; then
  echo "$0: Error: Missing input file (i.e., a file that contains Kani's `stderr` output)."
  exit 1
fi

sed -n '/^ *- [^(]*([0-9]*)/p' $1
