# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Common utilities for benchcomp


import argparse
import collections
import contextlib
import dataclasses
import logging
import pathlib
import sys
import textwrap

import yaml


class _SchemaValidator:
    """Validate data structures with a schema

    Objects of this class are callable, with a single `data` argument. The data
    is validated and returned if the `schema` packages is installed, or returned
    if not.
    """


    def __init__(self, schema_name):
        """
        schema_name: the name of a class in benchcomp.schemas
        """

        try:
            import benchcomp.schemas
            klass = getattr(benchcomp.schemas, schema_name)
            self.validate = klass()().validate
        except ImportError:
            self.validate = (lambda data: data)


    def __call__(self, data):
        return self.validate(data)



class ConfigFile(collections.UserDict):
    def __init__(self, path):
        super().__init__()

        try:
            with open(path, encoding="utf-8") as handle:
                data = yaml.safe_load(handle)
        except (FileNotFoundError, OSError) as exc:
            raise argparse.ArgumentTypeError(
                f"{path}: file not found") from exc

        validate = _SchemaValidator("BenchcompYaml")
        try:
            self.data = validate(data)
        except BaseException as exc:
            sys.exit(exc.code)


@dataclasses.dataclass
class Outfile:
    """Return a handle to a file on disk or stdout if given '-'"""

    path: str

    def __str__(self):
        return str(self.path)

    @contextlib.contextmanager
    def __call__(self):
        if self.path == "-":
            yield sys.stdout
            return
        path = pathlib.Path(self.path)
        path.parent.mkdir(exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            yield handle
