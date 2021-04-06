#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

rm -rf .sandbox || true
mkdir .sandbox

TEST_DIR=${1:-.}
UNWIND=${2:-10}

EXIT_CODE=0
for f in `find $TEST_DIR -name '*.rs'`; do
    BASE=`basename "$f"`
    NAME=${BASE%.rs}

    printf "Verifying %-64s" $f
    if [[ "$f" == *fixme* ]]; then
        echo "SKIP (known FAIL)"
        continue
    fi
    if [[ "$f" == *ignore* ]]; then
        echo "SKIP (not supported)"
        continue
    fi

    rmc $f -- --object-bits 11 --unwind $UNWIND > .sandbox/"$NAME".output

    CODE=$?
    if [[ $CODE == 0 ]]; then
        if [[ $NAME == *_fail* ]]; then
            echo "FAIL (expected verify failure)"
            EXIT_CODE=1
        else
            echo "PASS"
        fi
    else
        if [[ $NAME != *_fail* ]]; then
            echo "FAIL (expected verify okay)"
            EXIT_CODE=1
        else
            echo "PASS"
        fi
    fi
done

exit $EXIT_CODE
