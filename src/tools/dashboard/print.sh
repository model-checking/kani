#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# `rustdoc` treats this script as `rustc` and sends code extracted from markdown
# files to stdin of this script. Instead of compiling the code, this scripts
# simply copies the contents of stdin to the location where `rustdoc` caches the
# "compiled" output.

FILE="$6"
BASE=`basename "$FILE"`
mkdir -p "$BASE"
cp "/dev/stdin" "$FILE"
