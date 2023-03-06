# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Benchcomp regression testing suite. This suite uses Python's stdlib unittest
# module, but nevertheless actually runs the binary rather than running unit
# tests.

import pathlib
import subprocess
import tempfile
import unittest

import yaml


class Benchcomp:
    """Invocation of benchcomp binary with optional subcommand and flags"""

    def __init__(self, config):
        self.proc, self.stdout, self.stderr = None, None, None

        with tempfile.NamedTemporaryFile(
                mode="w", delete=False, suffix=".yaml") as tmp:
            yaml.dump(config, tmp, default_flow_style=False)
        self.config_file = tmp.name

        self.bc = str(pathlib.Path(__file__).parent.parent /
                      "bin" / "benchcomp")

        wd = tempfile.mkdtemp()
        self.working_directory = pathlib.Path(wd)

    def __call__(self, subcommand=None, default_flags=None, *flags):
        subcommand = subcommand or []
        default_flags = default_flags or [
            "--out-prefix", "/tmp/benchcomp/test"]
        config_flags = ["--config", str(self.config_file)]

        cmd = [self.bc, *config_flags, *subcommand, *default_flags, *flags]
        self.proc = subprocess.Popen(
            cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            cwd=self.working_directory)
        self.stdout, self.stderr = self.proc.communicate()


class RegressionTests(unittest.TestCase):
    def test_return_0(self):
        """Ensure that benchcomp terminates with return code 0"""

        with tempfile.TemporaryDirectory() as tmp:
            run_bc = Benchcomp({
                "variants": {
                    "variant_1": {
                        "config": {
                            "directory": tmp,
                            "command_line": "true",
                        }
                    },
                    "variant_2": {
                        "config": {
                            "directory": tmp,
                            "command_line": "true",
                        }
                    }
                },
                "run": {
                    "suites": {
                        "suite_1": {
                            "parser": {"module": "test"},
                            "variants": ["variant_1", "variant_2"]
                        }
                    }
                },
                "visualize": [],
            })
            run_bc()
            self.assertEqual(
                run_bc.proc.returncode, 0, msg=run_bc.stderr)

            with open(run_bc.working_directory / "result.yaml") as handle:
                result = yaml.safe_load(handle)

    def test_return_0_on_fail(self):
        """Ensure that benchcomp terminates with 0 even if a suite fails"""

        with tempfile.TemporaryDirectory() as tmp:
            run_bc = Benchcomp({
                "variants": {
                    "variant_1": {
                        "config": {
                            "directory": tmp,
                            "command_line": "false",
                        }
                    },
                    "variant_2": {
                        "config": {
                            "directory": tmp,
                            "command_line": "true",
                        }
                    }
                },
                "run": {
                    "suites": {
                        "suite_1": {
                            "parser": {"module": "test"},
                            "variants": ["variant_1", "variant_2"]
                        }
                    }
                },
                "visualize": [],
            })
            run_bc()
            self.assertEqual(
                run_bc.proc.returncode, 0, msg=run_bc.stderr)

            with open(run_bc.working_directory / "result.yaml") as handle:
                result = yaml.safe_load(handle)

    def test_env(self):
        """Ensure that benchcomp reads the 'env' key of variant config"""

        with tempfile.TemporaryDirectory() as tmp:
            run_bc = Benchcomp({
                "variants": {
                    "env_set": {
                        "config": {
                            "command_line": "echo $QJTX > out",
                            "directory": tmp,
                            "env": {"QJTX": "foo"}
                        }
                    },
                    "env_unset": {
                        "config": {
                            "command_line": "echo $QJTX > out",
                            "directory": tmp,
                        }
                    }
                },
                "run": {
                    "suites": {
                        "suite_1": {
                            "parser": {"module": "test"},
                            "variants": ["env_unset", "env_set"]
                        }
                    }
                },
                "visualize": [],
            })
            run_bc()
            self.assertEqual(
                run_bc.proc.returncode, 0, msg=run_bc.stderr)

            with open(run_bc.working_directory / "result.yaml") as handle:
                result = yaml.safe_load(handle)

            self.assertEqual(
                result["benchmarks"]["suite_1"]["variants"][
                    "env_set"]["metrics"]["foos"], 1,
                msg=yaml.dump(result, default_flow_style=False))

            self.assertEqual(
                result["benchmarks"]["suite_1"]["variants"][
                    "env_unset"]["metrics"]["foos"], 0,
                msg=yaml.dump(result, default_flow_style=False))
