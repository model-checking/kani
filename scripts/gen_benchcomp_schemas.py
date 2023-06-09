#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Dump the docstring of benchcomp data structures (currently just
# visualizations) into files to be rendered into the documentation.


import inspect
import pathlib
import sys
import textwrap

sys.path.append(str(pathlib.Path(__file__).parent.parent / "tools" / "benchcomp"))

# autopep8: off
import benchcomp.visualizers
# autopep8: on


def main():
    viz_headings = []
    viz_docs = []
    for name, obj in inspect.getmembers(benchcomp.visualizers):
        if not inspect.isclass(obj):
            continue

        viz_headings.append(f"- [{name}](#{name})")
        viz_docs.append(f"### {name}")
        viz_docs.append("")
        viz_docs.append(inspect.getdoc(obj))

    out_dir = sys.argv[1]

    with open(f"{out_dir}/visualization_list.txt", "w") as handle:
        print("\n".join(viz_headings), file=handle)

    with open(f"{out_dir}/visualization_docs.txt", "w") as handle:
        print("\n".join(viz_docs), file=handle)


if __name__ == "__main__":
    main()
