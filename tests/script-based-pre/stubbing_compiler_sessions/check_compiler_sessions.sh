#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Checks that the Kani compiler can encode harnesses with the same set of stubs
# in one rustc session.

set +e

log_file=output.log

kani stubbing.rs --enable-unstable --enable-stubbing --verbose >& ${log_file}

echo "------- Raw output ---------"
cat $log_file
echo "----------------------------"

# We print the reachability analysis results once for each session.
# Once we unify the stats per Kani compiler run, we should include number of
# sessions to the result.
runs=$(/usr/bin/env grep -c "Reachability Analysis Result" ${log_file})
echo "Rust compiler sessions: ${runs}"

# Cleanup
rm ${log_file}