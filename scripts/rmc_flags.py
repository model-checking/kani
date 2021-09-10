#!/usr/bin/env python3
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import argparse
import pathlib as pl

# Taken from https://github.com/python/cpython/blob/3.9/Lib/argparse.py#L858
# Cannot use `BooleanOptionalAction` with Python 3.8
class BooleanOptionalAction(argparse.Action):
    """ Implements argparse.BooleanOptionalAction introduced on Python 3.9

        This allows us to define an action as well as its negation to control a
        boolean option. For example, --default-checks and --no-default-checks
        options to control the same boolean property.
    """
    def __init__(self,
                 option_strings,
                 dest,
                 default=None,
                 type=None,
                 choices=None,
                 required=False,
                 help=None,
                 metavar=None):

        _option_strings = []
        for option_string in option_strings:
            _option_strings.append(option_string)

            if option_string.startswith('--'):
                option_string = '--no-' + option_string[2:]
                _option_strings.append(option_string)

        if help is not None and default is not None:
            help += f" (default: {default})"

        super().__init__(
            option_strings=_option_strings,
            dest=dest,
            nargs=0,
            default=default,
            type=type,
            choices=choices,
            required=required,
            help=help,
            metavar=metavar)

    def __call__(self, parser, namespace, values, option_string=None):
        if option_string in self.option_strings:
            setattr(namespace, self.dest, not option_string.startswith('--no-'))

    def format_usage(self):
        return ' | '.join(self.option_strings)

class ExtendAction(argparse.Action):
    """ Implements the "extend" option added on Python 3.8.

        Extend combines in one list all the arguments provided using the
        same option. For example, --c-lib <libA> --c-lib <libB> <libC> will
        generate a list [<libA>, <libB>, <libC>].
    """
    def __init__(self,
                 option_strings,
                 dest,
                 default=[],
                 **kwargs):

        if type(default) is not list:
            raise ValueError('default value for ExtendAction must be a list')

        super().__init__(
            option_strings=option_strings,
            dest=dest,
            default=default,
            **kwargs)

    def __call__(self, parser, namespace, values, option_string=None):
        items = getattr(namespace, self.dest, [])
        items.extend(values)
        setattr(namespace, self.dest, items)

# Add flags related to debugging output.
def add_loudness_flags(make_group, add_flag, config):
    group = make_group(
        "Loudness flags", "Determine how much textual output to produce.")
    add_flag(group, "--debug", default=False, action=BooleanOptionalAction,
             help="Produce full debug information")
    add_flag(group, "--quiet", "-q", default=False, action=BooleanOptionalAction,
             help="Produces no output, just an exit code and requested artifacts; overrides --verbose")
    add_flag(group, "--verbose", "-v", default=False, action=BooleanOptionalAction,
             help="Output processing stages and commands, along with minor debug information")

# Add flags which specify configurations for the proof.
def add_linking_flags(make_group, add_flag, config):
    group = make_group("Linking flags",
                       "Provide information about how to link the prover for RMC.")
    add_flag(group, "--c-lib", type=pl.Path, nargs="*", default=[],
             action=ExtendAction,
             help="Link external C files referenced by Rust code")
    add_flag(group, "--function", default="main",
             help="Entry point for verification")

# Add flags that produce extra artifacts.
def add_artifact_flags(make_group, add_flag, config):
    default_target = config["default-target"]
    assert default_target is not None, \
        f"Missing item in parser config: \"default-target\".\n" \
        "This is a bug; please report this to https://github.com/model-checking/rmc/issues."

    group = make_group(
        "Artifact flags", "Produce artifacts in addition to a basic RMC report.")
    add_flag(group, "--gen-c", default=False, action=BooleanOptionalAction,
             help="Generate C file equivalent to inputted program")
    add_flag(group, "--gen-c-runnable", default=False, action=BooleanOptionalAction,
             help="Generate C file equivalent to inputted program; "
                  "performs additional processing to produce valid C code "
                  "at the cost of some readability")
    add_flag(group, "--gen-symbols", default=False, action=BooleanOptionalAction,
             help="Generate a goto symbol table")
    add_flag(group, "--keep-temps", default=False, action=BooleanOptionalAction,
             help="Keep temporary files generated throughout RMC process")
    add_flag(group, "--target-dir", type=pl.Path, default=default_target, metavar="DIR",
             help=f"Directory for all generated artifacts; defaults to \"{default_target}\"")

# Add flags to turn off default checks.
def add_check_flags(make_group, add_flag, config):
    group = make_group("Check flags", "Disable some or all default checks.")
    add_flag(group, "--default-checks", default=True, action=BooleanOptionalAction,
             help="Turn on all default checks")
    add_flag(group, "--memory-safety-checks", default=True, action=BooleanOptionalAction,
             help="Turn on default memory safety checks")
    add_flag(group, "--overflow-checks", default=True, action=BooleanOptionalAction,
             help="Turn on default overflow checks")
    add_flag(group, "--unwinding-checks", default=True, action=BooleanOptionalAction,
             help="Turn on default unwinding checks")

# Add flags needed only for visualizer.
def add_visualizer_flags(make_group, add_flag, config):
    group = make_group(
        "Visualizer flags", "Generate an HTML-based UI for the generated RMC report.\nSee https://github.com/awslabs/aws-viewer-for-cbmc.")
    add_flag(group, "--srcdir", type=pl.Path, default=".",
             help="The source directory: the root of the source tree")
    add_flag(group, "--visualize", default=False, action=BooleanOptionalAction,
             help="Generate visualizer report to <target-dir>/report/html/index.html")
    add_flag(group, "--wkdir", type=pl.Path, default=".",
             help="""
                  The working directory: used to determine source locations in output;
                  this is generally the location from which rmc is currently being invoked
                  """)

# Add flags for ad-hoc features.
def add_other_flags(make_group, add_flag, config):
    group = make_group("Other flags")
    add_flag(group, "--allow-cbmc-verification-failure", default=False, action=BooleanOptionalAction,
             help="Do not produce error return code on CBMC verification failure")
    add_flag(group, "--dry-run", default=False, action=BooleanOptionalAction,
             help="Print commands instead of running them")

# Add flags we don't expect end-users to use.
def add_developer_flags(make_group, add_flag, config):
    group = make_group(
        "Developer flags", "These are generally meant for use by RMC developers, and are not stable.")
    add_flag(group, "--cbmc-args", nargs=argparse.REMAINDER, default=[],
             help="Pass through directly to CBMC; must be the last flag")
    add_flag(group, "--mangler", default="v0", choices=["v0", "legacy"],
             help="Change what mangler is used by the Rust compiler")
    add_flag(group, "--use-abs", default=False, action=BooleanOptionalAction,
             help="Use abstractions for the standard library")
    add_flag(group, "--abs-type", default="std", choices=["std", "rmc", "c-ffi", "no-back"],
             help="Choose abstraction for modules of standard library if available")

# Adds the flags common to both rmc and cargo-rmc.
# Allows you to specify flags/groups of flags to not add.
# This does not return the parser, but mutates the one provided.
def add_flags(parser, config, exclude_flags=[], exclude_groups=[]):
    # Keep track of what excluded flags and groups we've seen
    # so we can warn for possibly incorrect names passed in.
    excluded_flags = set()
    excluded_groups = set()

    # Add a group to the parser with title/description, and get a handler for it.
    def make_group(title=None, description=None):
        if title in exclude_groups:
            excluded_groups.add(group.title)
            return None

        return parser.add_argument_group(title, description)

    # Add the flag to the group, 
    def add_flag(group, flag, *args, **kwargs):
        if group == None:
            return

        if flag in exclude_flags:
            excluded_flags.add(flag)
            return
        group.add_argument(flag, *args, **kwargs)

    add_groups = [
        add_loudness_flags,
        add_linking_flags,
        add_artifact_flags,
        add_check_flags,
        add_visualizer_flags,
        add_other_flags,
        add_developer_flags
    ]

    for add_group in add_groups:
        add_group(make_group, add_flag, config)

    # Error if any excluded flags/groups don't exist.
    extra_flags = set(exclude_flags) - excluded_flags
    extra_groups = set(exclude_groups) - excluded_groups
    assert len(extra_flags.union(extra_groups)) == 0, \
        f"Attempt to exclude parser options which don't exist: {extra_groups.union(extra_flags)}\n" \
        "This is a bug; please report this to https://github.com/model-checking/rmc/issues."
