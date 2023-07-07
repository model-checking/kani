#!/usr/bin/python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
import re
import sys
import os.path as path
from enum import Enum
from itertools import chain


COMMENT_OR_EMPTY_PATTERN = '^(//.*$|#.*$|\\s*$)'

STANDARD_HEADER_PATTERN_1 = '(//|#|/\\*) Copyright Kani Contributors'
STANDARD_HEADER_PATTERN_2 = '(//|#| \\*) SPDX-License-Identifier: Apache-2.0 OR MIT'

MODIFIED_HEADER_PATTERN_1 = '(//|#) SPDX-License-Identifier: Apache-2.0 OR MIT'
MODIFIED_HEADER_PATTERN_2 = COMMENT_OR_EMPTY_PATTERN
MODIFIED_HEADER_PATTERN_3 = '(//|#) Modifications Copyright Kani Contributors'
MODIFIED_HEADER_PATTERN_4 = '(//|#) See GitHub history for details.'

class CheckResult(Enum):
    FAIL = 1
    PASS_STANDARD = 2
    PASS_MODIFIED = 3

def matches_header_lines(header, lines):
    matches = [regex.search(lines[idx]) for (regex, idx) in header]
    return all(matches)

def get_header(has_shebang, regexes):
    init_idx = 0 if not has_shebang else 1
    indices = range(init_idx, init_idx + len(regexes))
    return zip(regexes, indices)

def match_somewhere(regexes, lines, empty_or_comment_regex):
    """ Matches all MODIFIED_HEADER patterns within the file. This is used
    when the license is not at the top header, and there are licenses of
    external libraries. """
    maybe_match_head_index = [index for index, line in enumerate(lines) if regexes[0].search(line)][:1]
    if maybe_match_head_index and maybe_match_head_index[0] + len(regexes) <= len(lines):
        match_head_index = maybe_match_head_index[0]
        matches_pre_license = (empty_or_comment_regex.search(lines[index]) for index in range(match_head_index))
        matches_license = (regex.search(lines[match_head_index + index]) for index, regex in enumerate(regexes))
        return all(chain(matches_license, matches_pre_license))
    else:
        return False

def result_into_bool(result):
    if result == CheckResult.FAIL:
        return False
    return True

def copyright_check(filename):
    fo = open(filename)
    lines = fo.readlines()

    # The check is failed if the file is empty
    if len(lines) == 0:
        return CheckResult.FAIL

    # Scripts may include in their first line a character sequence starting with
    # '#!' (also know as shebang) to indicate an interpreter for execution.
    # The values for the minimum number of lines and the indices of copyright
    # lines depend on whether the file has a shebang or not.
    shb_re = re.compile('#!\\S+')
    has_shebang = shb_re.search(lines[0])
    min_lines = 3 if has_shebang else 2

    # The check is failed if the file does not contain enough lines
    if len(lines) < min_lines:
        return CheckResult.FAIL

    # Compile the regexes for the standard header
    regexes = []
    regexes.append(re.compile(STANDARD_HEADER_PATTERN_1))
    regexes.append(re.compile(STANDARD_HEADER_PATTERN_2))

    # We define a header as a list of pairs `(regex, idx)`
    # where `regex` is matched against `lines[idx]`
    header = get_header(has_shebang, regexes)

    # The copyright check succeeds if the regexes can be found
    if matches_header_lines(header, lines):
        return CheckResult.PASS_STANDARD

    # If there was no match, this may be a modified file which
    # includes a 4-lines header (i.e., `min_lines` is greater)
    min_lines = 5 if has_shebang else 4

    # The check is failed if the file does not contain enough lines
    if len(lines) < min_lines:
        return CheckResult.FAIL

    # Compile the regexes for the modified header
    regexes = []
    regexes.append(re.compile(MODIFIED_HEADER_PATTERN_1))
    regexes.append(re.compile(MODIFIED_HEADER_PATTERN_2))
    regexes.append(re.compile(MODIFIED_HEADER_PATTERN_3))
    regexes.append(re.compile(MODIFIED_HEADER_PATTERN_4))

    if match_somewhere(regexes, lines, re.compile(COMMENT_OR_EMPTY_PATTERN)):
        return CheckResult.PASS_MODIFIED

    # We fail if there were no matches
    return CheckResult.FAIL


if __name__ == "__main__":
    filenames = sys.argv[1:]

    # Only check regular files (skip symbolic link to directories)
    filenames = [fname for fname in filenames if path.isfile(fname)]

    # Get the copyright check for each file
    checks = [copyright_check(fname) for fname in filenames]

    all_checks_pass = True
    for i in range(len(filenames)):
        print(f'Copyright check - {filenames[i]}: ', end='')

        if checks[i] == CheckResult.PASS_STANDARD:
            print('PASS')
        elif checks[i] == CheckResult.PASS_MODIFIED:
            print('PASS (MODIFIED)')
        else:
            all_checks_pass = False
            print('FAIL')

    if all_checks_pass:
        sys.exit(0)
    else:
        sys.exit(1)
