# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Count and return the number of 'foo's in a file.


def main(root_dir):
    try:
        with open(root_dir / "out") as handle:
            data = handle.read().splitlines()
    except FileNotFoundError:
        data = []

    return {
        "metrics": {
            "foos": {},
        },
        "benchmarks": {
            "suite_1": {
                "metrics": {
                    "foos": len([l for l in data if l.strip() == "foo"]),
                },
            }
        },
    }
