# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT


import pathlib
import tempfile
import textwrap
import unittest

import yaml

import benchcomp
import benchcomp.cmd_args


class TestConfigFile(unittest.TestCase):
    def validate_against_schema(self, data):
        with tempfile.NamedTemporaryFile(mode="w") as tmp:
            yaml.dump(data, tmp, default_flow_style=False)
            benchcomp.ConfigFile(pathlib.Path(tmp.name))

    def test_1(self):
        self.validate_against_schema(yaml.safe_load(textwrap.dedent("""\
          variants:
            variant_1:
              config:
                command_line: cmd_1
                directory: dir_1

            variant_2:
              config:
                command_line: cmd_1
                directory: dir_1
                env:
                  ENV_VAR_1: value
                  ENV_VAR_2: value

          run:
            suites:
              suite_1:
                variants:
                  - variant_1
                  - variant_2
                parser:
                  module: test
          visualize: []
        """)))
