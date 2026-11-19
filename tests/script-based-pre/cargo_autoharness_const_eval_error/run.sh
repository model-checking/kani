#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# A constant whose evaluation fails (a type too big for the target) should be reported as a
# regular compilation error, not crash the reachability collector with "Instance with
# polymorphic constant". See https://github.com/model-checking/kani/issues/4814.
#
# Both the ICE and the fixed behavior end with E0080 and exit code 1, so matching the expected
# lines is not enough: fail with a distinct exit code if the compiler panicked.

output=$(cargo kani autoharness -Z autoharness --output-format=regular 2>&1)
status=$?
echo "${output}"

if echo "${output}" | grep -qE "panicked at|internal compiler error|Kani unexpectedly panicked"; then
    echo "FAILURE: kani-compiler panicked instead of reporting the error"
    exit 2
fi
exit ${status}
