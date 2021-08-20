#!/usr/bin/env python3
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# A script for testing the --gen-c-runnable flag which checks that
# the produced C program compiles with gcc.

import sys
import os
import pathlib

# To make this run significantly faster and to avoid issues with CBMC flags,
# disable the call to CBMC before running this script.

def test_file(path, fails_to=None):
    os.system(f"rmc --gen-c-runnable {path} > /dev/null")
    c_name = path.parent.joinpath(path.name[:-3] + "_runnable.c")
    retcode = os.system(f"gcc {c_name} 2> /dev/null")
    os.system(f"rm {c_name} a.out 2> /dev/null")
    if retcode != 0:
        print(f"Fail ({retcode}): {path}")
        if fails_to:
            with open(fails_to, "a") as f:
                f.write(f"{path}\n")

def test_dir(path, fails_to=None):
    for sub in path.glob(f"*"):
        test_path(sub, fails_to)

def test_path(path, fails_to=None):
    if path.is_dir():
        test_dir(path, fails_to)
    elif path.is_file() and pathlib.Path(path).suffix == ".rs":
        test_file(path, fails_to)
    else:
        pass

def main():
    if len(sys.argv) not in [2, 3]:
        print("Usage: python3 test_gen_c.py <path> [<fails to>]")
        sys.exit(1)
    
    path = sys.argv[1]
    if len(sys.argv) == 3:
        fails_to = sys.argv[2]
        with open(fails_to, "w") as f:
            pass
    else:
        fails_to = None
    test_path(pathlib.Path(path), fails_to)

if __name__ == "__main__":
    main()