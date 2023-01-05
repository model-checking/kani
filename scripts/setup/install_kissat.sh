#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Kissat release 3.0.0
FILE="rel-3.0.0.tar.gz"
URL="https://github.com/arminbiere/kissat/archive/refs/tags/$FILE"

set -x

wget -O "$FILE" "$URL"
tar -xvzf $FILE
cd kissat-rel-3.0.0
./configure && make kissat && sudo install build/kissat /usr/local/bin
cd -

# Clean up on success
rm $FILE
rm -rf kissat-rel-3.0.0
