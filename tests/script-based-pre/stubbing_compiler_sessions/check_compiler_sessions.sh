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
# This is the only reliable way to get the number of sessions from the compiler.
# The other option would be to use debug comments.
# Ideally, the compiler should only print one set of statistics at the end of its run.
# In that case, we should include number of sessions to those stats.
runs=$(/usr/bin/env grep -c "Reachability Analysis Result" ${log_file})
echo "Rust compiler sessions: ${runs}"

# Cleanup
rm ${log_file}
