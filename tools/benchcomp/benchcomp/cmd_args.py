# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Command line argument processing


import argparse
import importlib
import pathlib
import re
import textwrap

import benchcomp
import benchcomp.entry.benchcomp
import benchcomp.entry.run


def _get_epilogs():
    epilogs = {
        "top_level": """\
            benchcomp can help you to understand the difference between two or
            more toolchains, by running benchmarks that use those toolchains and
            comparing the results.

            benchcomp runs two or more 'variants' of a set of benchmark suites,
            and compares and visualizes the results of these variants. This
            allows you to understand the differences between the two variants,
            for example how they affect the benchmarks' performance or output or
            even whether they pass at all.

            benchmark is structured as a pipeline of several commands. Running
            `benchcomp` runs each of them sequentially. You can run the
            subcommands manually to dump the intermediate files if required.""",
        "run": """\
            The run command writes one YAML file for each (suite, variant) pair.
            These YAML files are in "suite.yaml" format.  Typically, users
            should read the combined YAML file emitted by `benchcomp collate`
            rather than the multiple YAML files written by `benchcomp run`.

            The `run` command writes its output files into a directory, which
            `collate` then reads from. By default, `run` writes the files into a
            new directory with a common prefix on each invocation, meaning that
            all previous runs are preserved without the user needing to specify
            a different directory each time. Benchcomp also creates a symbolic
            link to the latest run. Thus, the directories after several runs
            will look something like this:

            /tmp/benchcomp/suites/2F0D3DC4-0D02-4E95-B887-4759F08FA90D
            /tmp/benchcomp/suites/119F11EB-9BC0-42D8-9EC1-47DFD661AC88
            /tmp/benchcomp/suites/A3E83FE8-CD42-4118-BED3-ED89EC88BFB0
            /tmp/benchcomp/suites/latest -> /tmp/benchcomp/suites/119F11EB...

            '/tmp/benchcomp/suites' is the "out-prefix"; the UUID is the
            "out-dir"; and '/tmp/benchcomp/latest' is the "out-symlink". Users
            can set each of these manually by passing the corresponding flag, if
            needed.

            Passing `--out-symlink ./latest` will place the symbolic link in the
            current directory, while keeping all runs under /tmp to avoid
            clutter. If you wish to keep all previous runs in a local directory,
            you can do so with

                `--out-prefix ./output --out-symlink ./output/latest`""",
        "filter": "",  # TODO
        "visualize": "",  # TODO
        "collate": "",
    }

    wrapper = textwrap.TextWrapper()
    ret = {}
    for subcommand, epilog in epilogs.items():
        paragraphs = re.split(r"\n\s*\n", epilog)
        buf = []
        for p in paragraphs:
            p = textwrap.dedent(p)
            buf.extend(wrapper.wrap(p))
            buf.append("")
        ret[subcommand] = "\n".join(buf)
    return ret


def _existing_directory(arg):
    path = pathlib.Path(arg)
    if not path.exists():
        raise ValueError(f"directory '{arg}' must already exist")
    return path


def _get_args_dict():
    epilogs = _get_epilogs()
    ret = {
        "top_level": {
            "description":
                "Run and compare variants of a set of benchmark suites",
            "epilog": epilogs["top_level"],
            "formatter_class": argparse.RawDescriptionHelpFormatter,
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
                        "default":
                            benchcomp.entry.run.get_default_out_prefix() /
                        benchcomp.entry.run.get_default_out_symlink(),
                        "help":
                            "symbolically link D to the output directory "
                            "(default: %(default)s)",
                    }, {
                        "flags": ["--no-copy"],
                        "action": "store_false",
                        "dest": "copy_benchmarks_dir",
                        "help":
                            "do not make a fresh copy of the benchmark "
                            "directories before running each variant",
                    }, {
                        "flags": ["--no-cleanup-run-dirs"],
                        "action": "store_false",
                        "dest": "cleanup_directory",
                        "help":
                            "do not delete fresh copies of benchmark "
                            "directories after running each variant",
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
                    }, {
                        "flags": ["--only"],
                        "nargs": "+",
                        "metavar": "V",
                        "help":
                            "Only run visualization V; ignore others in "
                            "config file"
                    }, {
                        "flags": ["--except"],
                        "dest": "except_for",
                        "nargs": "+",
                        "metavar": "V",
                        "help": "Run all visualizations except V",
                    }],
                },
            }
        }
    }
    for subcommand, info in ret["subparsers"]["parsers"].items():
        info["epilog"] = epilogs[subcommand]
        info["formatter_class"] = argparse.RawDescriptionHelpFormatter
    return ret


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
    for subcommand, info in subparsers.items():
        args = info.pop("args")
        subparser = subs.add_parser(name=subcommand, **info)

        # Set entrypoint to benchcomp.entry.visualize.main()
        # when user invokes `benchcomp visualize`, etc
        mod = importlib.import_module(f"benchcomp.entry.{subcommand}")
        subparser.set_defaults(func=mod.main)

        for arg in args:
            flags = arg.pop("flags")
            subparser.add_argument(*flags, **arg)
            if arg not in global_args:
                parser.add_argument(*flags, **arg)

    return parser.parse_args()
