#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

KANI_VERSION_CMD=`kani --version`
KANI_VERSION_CMD_VERSION=`echo ${KANI_VERSION_CMD} | awk '{print $2}'`

# Check that the version printed is the same. Note: We use `sed -n '1p'` instead
# of `head -n 1` to avoid https://github.com/model-checking/kani/issues/2618
KANI_STANDALONE_OUTPUT_HEAD=`kani dummy-file.rs | sed -n '1p'`
KANI_STANDALONE_OUTPUT_HEAD_VERSION=`echo ${KANI_STANDALONE_OUTPUT_HEAD} | awk '{print $4}'`

if [[ $KANI_VERSION_CMD_VERSION == $KANI_STANDALONE_OUTPUT_HEAD_VERSION ]]; then
    echo "success: version printed agrees"
else
    echo "failed: version printed differs ($KANI_VERSION_CMD_VERSION - $KANI_STANDALONE_OUTPUT_HEAD_VERSION)"
    exit 1
fi

KANI_STANDALONE_OUTPUT_HEAD_MODE=`echo ${KANI_STANDALONE_OUTPUT_HEAD} | awk '{print $5}'`

# Check that `(standalone)` appears in the version line
if [[ $KANI_STANDALONE_OUTPUT_HEAD_MODE == "(standalone)" ]]; then
    echo "success: \`(standalone)\` appears in version line"
else
    echo "failed: expected \`(standalone)\` in version line"
    exit 1
fi
