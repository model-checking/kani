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


def cbmc_viewer_version():
    cmd = ["cbmc-viewer", "--version"]
    try:
        version = subprocess.run(cmd,
                                 capture_output=True, text=True, check=True)
    except (OSError, subprocess.SubprocessError) as error:
        print(error)
        print(f"Can't run command '{' '.join(cmd)}'")
        sys.exit(EXIT_CODE_FAIL)

    match = re.match("CBMC viewer ([0-9]+).([0-9]+)", version.stdout)
    if not match:
        print(f"Can't parse CBMC-viewer version string: '{version.stdout.strip()}'")
        sys.exit(EXIT_CODE_FAIL)

    return match.groups()

def complete_version(*version):
    numbers = [int(num) if num else 0 for num in version]
    return (numbers + [0, 0])[:2]

def main():
    parser = argparse.ArgumentParser(
        description='Check CBMC-viewer version matches major/minor')
    parser.add_argument('--major', required=True)
    parser.add_argument('--minor', required=True)
    args = parser.parse_args()

    current_version = complete_version(*cbmc_viewer_version())
    desired_version = complete_version(args.major, args.minor)

    if desired_version > current_version:
        version_string = '.'.join([str(num) for num in current_version])
        desired_version_string = '.'.join([str(num) for num in desired_version])
        print(f'ERROR: CBMC-viewer version is {version_string}, expected at least {desired_version_string}')
        sys.exit(EXIT_CODE_MISMATCH)

if __name__ == "__main__":
    main()
