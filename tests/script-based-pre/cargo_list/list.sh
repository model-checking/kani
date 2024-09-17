#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# The table in the expected omits the table borders because the runtest script
# does not evaluate the table borders in the captured output as equal to the table borders in the expected file.

cargo kani list -Z list