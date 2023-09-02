# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Entrypoint for `benchcomp run`. This command runs all combinations of
# benchmark suites x variants that are defined in a config file. After each
# combination, this command uses a 'parser' to write the list of benchmarks and
# their associated metrics to a file using a unified schema called
# `suite.yaml`. Parsers are python submodules of benchcomp.parsers; the
# configuration file describes which parser to use for each benchmark suite.


import dataclasses
import logging
import os
import pathlib
import shutil
import subprocess
import typing
import uuid

import yaml

import benchcomp
import benchcomp.parsers


@dataclasses.dataclass
class _SingleInvocation:
    """Run and parse the result of a single suite x variant"""

    suite_id: str
    variant_id: str

    parse: typing.Any

    suite_yaml_out_dir: pathlib.Path
    copy_benchmarks_dir: bool

    command_line: str
    directory: pathlib.Path

    cleanup_directory: bool
    env: dict = dataclasses.field(default_factory=dict)
    timeout: int = None
    memout: int = None
    patches: list = dataclasses.field(default_factory=list)

    def __post_init__(self):
        self.directory = pathlib.Path(self.directory).expanduser()
        if self.copy_benchmarks_dir:
            self.working_copy = pathlib.Path(
                f"/tmp/benchcomp/suites/{uuid.uuid4()}")
        else:
            self.working_copy = pathlib.Path(self.directory)

    def __call__(self):
        env = dict(os.environ)
        env.update(self.env)

        if self.copy_benchmarks_dir:
            shutil.copytree(
                self.directory, self.working_copy,
                ignore_dangling_symlinks=True, symlinks=True)

        try:
            subprocess.run(
                self.command_line, shell=True, env=env, cwd=self.working_copy,
                check=True)
        except subprocess.CalledProcessError as exc:
            logging.warning(
                "Invocation of suite %s with variant %s exited with code %d",
                self.suite_id, self.variant_id, exc.returncode)
        except (OSError, subprocess.SubprocessError):
            logging.error(
                "Invocation of suite %s with variant %s failed", self.suite_id,
                self.variant_id)

        suite = self.parse(self.working_copy)

        suite["suite_id"] = self.suite_id
        suite["variant_id"] = self.variant_id

        out_file = f"{self.suite_id}@{self.variant_id}_suite.yaml"
        with open(
                self.suite_yaml_out_dir / out_file, "w",
                encoding="utf-8") as handle:
            yaml.dump(suite, handle, default_flow_style=False)

        if self.cleanup_directory and self.copy_benchmarks_dir:
            shutil.rmtree(self.working_copy)


@dataclasses.dataclass
class _Run:
    """Run all suite x variant combinations, write results to a directory"""

    config: benchcomp.ConfigFile
    out_prefix: pathlib.Path
    out_dir: str
    out_symlink: str
    copy_benchmarks_dir: bool
    cleanup_directory: bool
    result: dict = None

    def __call__(self):
        out_path = (self.out_prefix / self.out_dir)
        out_path.mkdir(parents=True)

        for suite_id, suite in self.config["run"]["suites"].items():
            parse = benchcomp.parsers.get_parser(suite["parser"])
            for variant_id in suite["variants"]:
                variant = self.config["variants"][variant_id]
                config = dict(variant).pop("config")
                invoke = _SingleInvocation(
                    suite_id, variant_id,
                    parse, suite_yaml_out_dir=out_path,
                    copy_benchmarks_dir=self.copy_benchmarks_dir,
                    cleanup_directory=self.cleanup_directory,
                    **config)
                invoke()

        # Atomically symlink the symlink dir to the output dir, even if
        # there is already an existing symlink with that name
        tmp_symlink = pathlib.Path(
            self.out_symlink).with_suffix(f".{uuid.uuid4()}")
        tmp_symlink.parent.mkdir(exist_ok=True)
        tmp_symlink.symlink_to(out_path)
        tmp_symlink.rename(self.out_symlink)


def get_default_out_symlink():
    return "latest"


def get_default_out_dir():
    return str(uuid.uuid4())


def get_default_out_prefix():
    return pathlib.Path("/tmp") / "benchcomp" / "suites"


def main(args):
    run = _Run(
        args.config, args.out_prefix, args.out_dir, args.out_symlink,
        args.copy_benchmarks_dir, args.cleanup_directory)
    run()
    return run
