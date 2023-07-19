# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

import re
import sys
from operator import attrgetter

def postprocess(checks):
    sorted_checks = sorted(checks, key=attrgetter('file', 'line'))
    filter_checks = filter(lambda x: x.description == "cover_experiment", sorted_checks)
    files = {}
    for check in filter_checks:
        if check.file not in files:
            files[check.file] = [(check.line, check.status)]
        else:
            files[check.file].append((check.line, check.status))

    coverage_results = {}
    for file in files:
        lines = set([check[0] for check in files[file]])
        line_results = []
        for line in lines:
            satisfiable_statuses = [s == "SATISFIED" for (x, s) in files[file] if line == x]
            covered_status = "COVERED" if all(satisfiable_statuses) else "UNCOVERED"
            line_results.append((line, covered_status))
        coverage_results[file] = sorted(line_results)

    return coverage_results


class Check:
    def __init__(self, check_number, status, description, file, line):
        self.check_number = check_number
        self.status = status
        self.description = description
        self.file = file
        self.line = line

    def __str__(self):
        return f"Check ID: {self.check_number}\nStatus: {self.status}\nDescription: {self.description}\nLocation: {self.file}:{self.line}"

check_regex = re.compile(r"Check (\d+):")
status_regex = re.compile(r"Status: (\w+)")
description_regex = re.compile(r'Description: "([^"]+)"')
location_regex = re.compile(r"Location: (.+):(\d+):")

if len(sys.argv) < 2:
    print("Usage: python parse_results.py <file_path>")
    sys.exit(1)

file_path = sys.argv[1]

with open(file_path, "r") as file:
    checks = []
    current_check = None

    for line in file:

        check_match = check_regex.match(line)
        if check_match:
            if current_check:
                checks.append(current_check)
                current_check = None
            check_number = check_match.group(1)
            current_check = Check(check_number, "", "", "", "")

        status_match = status_regex.search(line)
        if status_match and current_check:
            status = status_match.group(1)
            current_check.status = status

        description_match = description_regex.search(line)
        if description_match and current_check:
            description = description_match.group(1)
            current_check.description = description

        location_match = location_regex.search(line)
        if location_match and current_check:
            file = location_match.group(1)
            current_check.file = file
            line = location_match.group(2)
            current_check.line = line

    coverage_results = postprocess(checks)

    for file in coverage_results:
        for (x, s) in coverage_results[file]:
            print(file, x, s, sep=", ")
        print()
