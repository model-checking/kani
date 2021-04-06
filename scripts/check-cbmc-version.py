#!/usr/bin/env python3
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import argparse
import re
import sys
import subprocess


EXIT_CODE_SUCCESS = 0
EXIT_CODE_MISMATCH = 1
EXIT_CODE_FAIL = 2


def cbmc_version():
    cmd = ["cbmc", "--version"]
    try:
        version = subprocess.run(cmd,
                                 capture_output=True, text=True, check=True)
    except (OSError, subprocess.SubprocessError) as error:
        print(error)
        print(f"Can't run command '{' '.join(cmd)}'")
        sys.exit(EXIT_CODE_FAIL)

    match = re.match("([0-9]+).([0-9]+).([0-9]+)", version.stdout)
    if not match:
        print(f"Can't parse cbmc version string: '{version.stdout.strip()}'")
        sys.exit(EXIT_CODE_FAIL)

    return match.groups()

def complete_version(*version):
    numbers = [int(num) if num else 0 for num in version]
    return (numbers + [0, 0, 0])[:3]

def main():
    parser = argparse.ArgumentParser(
        description='Check CBMC version matches major/minor/patch')
    parser.add_argument('--major', required=True)
    parser.add_argument('--minor', required=True)
    parser.add_argument('--patch')
    args = parser.parse_args()

    current_version = complete_version(*cbmc_version())
    desired_version = complete_version(args.major, args.minor, args.patch)

    if desired_version > current_version:
        version_string = '.'.join([str(num) for num in current_version])
        print(f'WARNING: CBMC version is {version_string}')
        sys.exit(EXIT_CODE_MISMATCH)

if __name__ == "__main__":
    main()
