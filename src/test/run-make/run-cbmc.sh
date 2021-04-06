#!/bin/bash

set -x

DIR=$1
INPUT=$2
OUTPUT=${2/%.rs/}
TMPFILE=$3
FUNC=${4:-main}
symtab2gb "$DIR/${OUTPUT}.json" --out "$DIR/$OUTPUT" >/dev/null 2>&1 || exit 100
cbmc $CBMC_EXTRA_FLAGS --object-bits 11 --signed-overflow-check --unsigned-overflow-check --function "$FUNC" "$DIR/$OUTPUT" > "$DIR/$TMPFILE" || true
