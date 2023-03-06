# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Command line argument processing


import argparse
import importlib
import pathlib

import benchcomp
import benchcomp.entry.benchcomp
import benchcomp.entry.run


_EPILOG = """
benchcomp can help you to understand the difference between two or more
toolchains, by running benchmarks that use those toolchains and comparing the
results.

benchcomp runs two or more 'variants' of a set of benchmark suites, and compares
and visualizes the results of these variants. This allows you to understand the
differences between the two variants, for example how they affect the
benchmarks' performance or output or even whether they pass at all.

benchmark is structured as a pipeline of several commands. Running `benchcomp`
runs each of them sequentially. You can run the subcommands manually to dump the
intermediate files if required.
"""


def _existing_directory(arg):
    path = pathlib.Path(arg)
    if not path.exists():
        raise ValueError(f"directory '{arg}' must already exist")
    return path


def _non_existing_directory(arg):
    path = pathlib.Path(arg)
    if path.exists():
        raise ValueError(f"directory '{arg}' must not already exist")
    return path


def _get_args_dict():
    return {
        "top_level": {
            "description":
                "Run and compare variants of a set of benchmark suites",
            "epilog": _EPILOG,
        },
        "args": [],
        "subparsers": {
            "title": "benchcomp subcommands",
            "description":
                "You can invoke each stage of the benchcomp pipeline "
                "separately if required",
            "parsers": {
                "run": {
                    "help": "run all variants of all benchmark suites",
                    "args": [{
                        "flags": ["--out-prefix"],
                        "metavar": "D",
                        "type": pathlib.Path,
                        "default": benchcomp.entry.run.get_default_out_prefix(),
                        "help":
                            "write suite.yaml files to a new directory under D "
                            "(default: %(default)s)",
                    }, {
                        "flags": ["--out-dir"],
                        "metavar": "D",
                        "type": str,
                        "default": benchcomp.entry.run.get_default_out_dir(),
                        "help":
                            "write suite.yaml files to D relative to "
                            "--out-prefix (must not exist) "
                            "(default: %(default)s)",
                    }, {
                        "flags": ["--out-symlink"],
                        "metavar": "D",
                        "type": pathlib.Path,
                        "default": benchcomp.entry.run.get_default_out_symlink(),
                        "help":
                            "symbolically link D to the output directory "
                            "(default: %(default)s)",
                    }],
                },
                "collate": {
                    "args": [{
                        "flags": ["--suites-dir"],
                        "metavar": "D",
                        "type": _existing_directory,
                        "default":
                            benchcomp.entry.run.get_default_out_prefix() /
                        benchcomp.entry.run.get_default_out_symlink(),
                        "help":
                            "directory containing suite.yaml files "
                            "(default: %(default)s)"
                    }, {
                        "flags": ["--out-file"],
                        "metavar": "F",
                        "default": benchcomp.Outfile("result.yaml"),
                        "type": benchcomp.Outfile,
                        "help":
                            "write result to F instead of %(default)s. "
                            "'-' means print to stdout",
                    }],
                },
                "filter": {
                    "help": "transform a result by piping it through a program",
                    "args": [],
                },
                "visualize": {
                    "help": "render a result in various formats",
                    "args": [{
                        "flags": ["--result-file"],
                        "metavar": "F",
                        "default": pathlib.Path("result.yaml"),
                        "type": pathlib.Path,
                        "help":
                            "read result from F instead of %(default)s. "
                    }],
                },
            }
        }
    }


def _get_global_args():
    return [{
        "flags": ["-c", "--config"],
        "default": "benchcomp.yaml",
        "type": benchcomp.ConfigFile,
        "metavar": "F",
        "help": "read configuration from file F (default: %(default)s)",
    }, {
        "flags": ["-v", "--verbose"],
        "action": "store_true",
        "help": "enable verbose output",
    }, {
        "flags": ["--fail-fast"],
        "action": "store_true",
        "help": "terminate with 1 at the first sign of trouble",
    }]


def get():
    ad = _get_args_dict()
    parser = argparse.ArgumentParser(**ad["top_level"])

    parser.set_defaults(func=benchcomp.entry.benchcomp.main)

    global_args = _get_global_args()

    ad["args"].extend(global_args)
    for arg in ad["args"]:
        flags = arg.pop("flags")
        parser.add_argument(*flags, **arg)

    subparsers = ad["subparsers"].pop("parsers")
    subs = parser.add_subparsers(**ad["subparsers"])
    seen_flags = set()
    for subcommand, info in subparsers.items():
        args = info.pop("args")
        subparser = subs.add_parser(name=subcommand, **info)

        # Set entrypoint to benchcomp.entry.visualize.main()
        # when user invokes `benchcomp visualize`, etc
        mod = importlib.import_module(f"benchcomp.entry.{subcommand}")
        subparser.set_defaults(func=mod.main)

        for arg in args:
            flags = arg.pop("flags")
            seen_flags = seen_flags.union(flags)
            subparser.add_argument(*flags, **arg)
            if arg not in global_args:
                parser.add_argument(*flags, **arg)

    return parser.parse_args()
