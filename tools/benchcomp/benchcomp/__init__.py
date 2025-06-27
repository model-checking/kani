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


class ConfigFile(collections.UserDict):
    _schema: str = textwrap.dedent("""\
variants:
  type: dict
  keysrules:
    type: string
  valuesrules:
    schema:
      config:
        type: dict
        keysrules:
          type: string
        valuesrules:
          allow_unknown: true
          schema:
            command_line:
              type: string
            directory:
              type: string
            env:
              type: dict
              keysrules:
                type: string
              valuesrules:
                type: string
run:
  type: dict
  keysrules:
    type: string
  schema:
    suites:
      type: dict
      keysrules:
        type: string
      valuesrules:
        schema:
          variants:
            type: list
          parser:
            type: dict
            keysrules:
              type: string
            valuesrules:
              anyof:
                - schema:
                    type: {}
filters:
    type: list
    default: []
    schema:
        type: dict
        keysrules:
            type: string
            allowed: ["command_line"]
        valuesrules:
            type: string
visualize: {}
""")

    def __init__(self, path):
        super().__init__()

        try:
            with open(path, encoding="utf-8") as handle:
                data = yaml.safe_load(handle)
        except (FileNotFoundError, OSError) as exc:
            raise argparse.ArgumentTypeError(
                f"{path}: file not found") from exc

        schema = yaml.safe_load(self._schema)
        try:
            import cerberus
            validate = cerberus.Validator(schema)
            if not validate(data):
                for error in validate._errors:
                    doc_path = "/".join(error.document_path)
                    msg = (
                        f"config file '{path}': key "
                        f"'{doc_path}': expected "
                        f"{error.constraint}, got '{error.value}'")
                    if error.rule:
                        msg += f" (rule {error.rule})"
                    msg += f" while traversing {error.schema_path}"
                    logging.error(msg)
                logging.error(validate.document_error_tree["variants"])
                raise argparse.ArgumentTypeError(
                    "failed to validate configuration file")
        except ImportError:
            pass
        self.data = data


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
