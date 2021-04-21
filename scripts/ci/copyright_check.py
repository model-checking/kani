#!/usr/bin/python3
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT
import re
import sys

def copyright_check(filename):
    fo = open(filename)
    lines = fo.readlines()

    # The check is failed if the file is empty
    if len(lines) == 0:
        return False

    # Scripts may include in their first line a character sequence starting with
    # '#!' (also know as shebang) to indicate an interpreter for execution.
    # The values for the minimum number of lines and the indices of copyright
    # lines depend on whether the file has a shebang or not.
    shb_re = re.compile('#!\S+')
    has_shebang = shb_re.search(lines[0])
    min_lines = 3 if has_shebang else 2
    
    # The check is failed if the file does not contain enough lines
    if len(lines) < min_lines:
        return False

    # Compile the regexes for copyright lines
    fst_re = re.compile('(//|#) Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.')
    snd_re = re.compile('(//|#) SPDX-License-Identifier: Apache-2.0 OR MIT')

    fst_idx = min_lines - 2
    snd_idx = min_lines - 1

    # The copyright check succeeds if the regexes can be found
    if fst_re.search(lines[fst_idx]) and snd_re.search(lines[snd_idx]):
        return True
    else:
        return False

if __name__ == "__main__":
    filenames = sys.argv[1:]
    checks = [copyright_check(fname) for fname in filenames]
    
    for i in range(len(filenames)):
        print(f'Copyright check - {filenames[i]}: ', end='')
        print('PASS') if checks[i] else print('FAIL')

    if all(checks):
        sys.exit(0)
    else:
        sys.exit(1)
