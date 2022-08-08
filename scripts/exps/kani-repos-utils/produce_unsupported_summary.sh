#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Print a table from a file with data about unsupported features.
# We capture the data in unsupported constructs expressions of the
# form " - <construct> (<instances>)". For example:
# ```
#             - 'simd_eq' intrinsic (1)
#             - Rvalue::ThreadLocalRef (2)
# ```
#
# And then compute:
#  1. The number of crates impacted by a given construct
#     (i.e., the number of times it appears in the data)
#  2. The number of instances of a given construct
#     (i.e., the number of instances according to the captured value)
#
# The data is sorted in descending order by the number of crates impacted.

if [[ $# -ne 1 ]]; then
  echo "$0: Error: Missing input file."
  exit 1
fi

echo "SUMMARY - UNSUPPORTED FEATURES"
echo "========================================================="
echo "Unsupported feature | Crates impacted | Instances of use"
echo "---------------------------------------------------------"
cat $1 | sed -n 's/^ *- \([^(]*\)(\([0-9]*\))/\2 | \1/p' | \
awk -F '|' '{
    if(times[$2]){
        times[$2] += $1;
        crates[$2] += 1;
    } else {
        times[$2] = $1;
        crates[$2] = 1;
    }
}
END {
    for (i in times) {
        printf "%3d | %3d | %s\n", crates[i], times[i], i;
    }
}' | \
sort -r | awk -F '|' '{
    printf "%45.45s | %3d | %3d\n", $3, $1, $2
}'
echo "========================================================="
