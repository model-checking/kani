#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Source kani-dependencies to get KISSAT_VERSION
source kani-dependencies

if [ -z "${KISSAT_VERSION:-}" ]; then
  echo "$0: Error: KISSAT_VERSION is not specified"
  exit 1
fi


# Kissat release
FILE="rel-${KISSAT_VERSION}.tar.gz"
URL="https://github.com/arminbiere/kissat/archive/refs/tags/$FILE"

set -x

wget -O "$FILE" "$URL"
tar -xvzf $FILE
DIR_NAME="kissat-rel-${KISSAT_VERSION}"
cd $DIR_NAME
./configure && make kissat && sudo install build/kissat /usr/local/bin
cd -

# Clean up on success
rm $FILE
rm -rf $DIR_NAME
