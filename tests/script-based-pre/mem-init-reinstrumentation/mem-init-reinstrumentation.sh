#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -u

KANI_OUTPUT=`kani -Z uninit-checks alloc-zeroed.rs`
echo "$KANI_OUTPUT" | egrep -q "kani::mem_init::.*pointer_dereference"
INSTRUMENTATION_DETECTED=$?

if [[ $INSTRUMENTATION_DETECTED == 0 ]]; then
    echo "failed: pointer checks are detected in initialized memory instrumentaiton"
    exit 1
elif [[ $INSTRUMENTATION_DETECTED == 1 ]]; then
    echo "success: no pointer checks are detected in initialized memory instrumentaiton"
    exit 0
else 
    echo "failed: error occured when runnning egrep"
    exit 0
fi
