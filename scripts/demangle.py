#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

"""This small script replaces the tags in GotoC code with the proper Rust type names.

This makes understanding the C code much easier and can help with debugging.
"""

import json
import sys

if __name__ == "__main__":
    if len(sys.argv) < 1 or sys.argv[1] in ["-h", "--help"]:
        print("Usage: demangle.py file")
        print("This will replace the tags in `file.out.c` with the type names from `file.type_map.json`.")
        print("The output will be written to `file.out.demangled.c`.")
        sys.exit(0)
    name = sys.argv[1]
    if name.endswith(".rs"):
        name = name[:-3]
    with open(f"{name}.type_map.json") as tyfile:
        typemap = json.load(tyfile)
    with open(f"{name}.out.c") as cfile:
        code = cfile.read()
    for tag, typename in typemap.items():
        if tag.startswith("tag-"):
            tag = tag[4:]
            code = code.replace(tag, typename)
        else:
            print(f"Unexpected tag: {tag} does not start with 'tag-'.", file=sys.stderr)
    with open(f"{name}.out.demangled.c", "w") as outfile:
        outfile.write(code)
        print(f"Demangled output written to `{outfile.name}`.")
