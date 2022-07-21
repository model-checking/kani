#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

if [[ $# -ne 1 ]]; then
  echo "$0: Error: Missing input file."
  exit 1
fi

# Filter lines with unsupported constructs.
# These appear in `stderr` after a warning as follows:
#
# ```
# warning: Found the following unsupported constructs:
#             - 'simd_eq' intrinsic (1)
#             - Rvalue::ThreadLocalRef (2)
# ```
sed -n '/^ *- [^(]*([0-9]*)/p' $1
