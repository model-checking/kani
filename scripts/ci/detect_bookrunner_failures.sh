#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# This script checks that the number of failures in a bookrunner run are below
# the threshold computed below.
#
# The threshold is roughly computed as: `1.05 * <number of expected failures>`
# The extra 5% allows us to account for occasional timeouts. It is reviewed and
# updated whenever the Rust toolchain version is updated.
EXPECTED=82
THRESHOLD=$(expr ${EXPECTED} \* 105 / 100) # Add 5% threshold

if [[ $# -ne 1 ]]; then
  echo "$0: Error: Specify the bookrunner text report"
  exit 1
fi

# Get the summary line, which looks like:
#   `# of tests: <total> ✔️ <passes>   ❌ <failures>`
SUMMARY_LINE=`head -n 1 $1`

# Parse the summary line and extract the number of failures
read -a strarr <<< $SUMMARY_LINE
NUM_FAILURES=${strarr[-1]}

# Print a message and return a nonzero code if the threshold is exceeded
if [[ $NUM_FAILURES -ge $THRESHOLD ]]; then
    echo "Error: The number of failures from bookrunner is higher than expected!"
    echo
    echo "Found $NUM_FAILURES which is higher than the threshold of $THRESHOLD"
    echo "This means that your changes are causing at least 5% more failures than in previous bookrunner runs."
    echo "To check these failures locally, run \`cargo run -p bookrunner\` and inspect the report in \`build/output/latest/html/index.html\`."
    echo "For more details on bookrunner, go to https://model-checking.github.io/kani/bookrunner.html"
    exit 1
fi
