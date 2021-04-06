#!/usr/bin/env python3
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""
Test runner for cargo-rmc-test directories

Given a cargo directory we look for expected.<func> files in the top-level. We
run cargo-rmc with <func> as the entry point and make sure that every line in
expected.<func> appears in the actual output.
"""


import argparse
import glob
import os
import sys
import subprocess


def compare_output(expected, actual):
    """Check that every line in expected appears in actual"""
    with open(expected) as f, open(actual) as g:
        expected = f.read().splitlines()
        actual = g.read().splitlines()
        def appears_in_actual(line):
            return line in actual
        not_found = [ line for line in expected if not appears_in_actual(line) ]
        for line in not_found:
            print(f"ERROR expected line missing: {line}", file=sys.stderr)
        return not_found == []


def main():
    """Check a cargo directory with cargo-rmc against expected outputs"""
    parser = argparse.ArgumentParser(description="Drive a cargo-rmc test")
    parser.add_argument("cargo_dir", help="cargo directory to verify")
    args = parser.parse_args()
    os.chdir(args.cargo_dir)
    exit_code = 0
    for expected_result in glob.glob("expected.*"):
        func = expected_result.split(".")[1]
        cmd = ["cargo", "rmc", ".", "--function", func]
        output = f"out.{func}"
        print(f"Verifying {args.cargo_dir}{func}".ljust(64+10), end="")
        with open(output, "w") as f:
            subprocess.run(cmd, stdout=f, check=True)
        if compare_output(expected_result, output):
            print("PASS")
        else:
            print("FAIL")
            exit_code = 1
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
