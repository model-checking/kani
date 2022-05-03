#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Script that enables auto formating of python files.
# Usage:
# $ ./python-fmt.sh [--diff | --in-place | --help]

set -o errexit
set -o pipefail
set -o nounset

if ! type autopep8 >& /dev/null
then
    echo -e "[Error] Could not find autopep8. To install autopep8 run:"
    echo -e "    $ pip install --upgrade autopep8"
    exit 1
fi

mode=${1:-"--diff"}

pattern="--diff\|--in-place"
if [ $(expr match "${mode}" "${pattern}") -eq 0 ]
then
    if [ "${mode}" != "--help" ]
    then
        echo -e "[Error] Invalid option '${mode}'."
    fi
    echo "Usage: "
    echo "$ ./python-fmt.sh [--diff | --in-place | --help]"
    exit 1
fi

echo "Running autopep8 in ${mode} mode"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
autopep8 $mode -r $SCRIPT_DIR --exit-code
echo "Success"
