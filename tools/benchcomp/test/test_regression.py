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
    def setUp(self):
        self.kani_dir = pathlib.Path(__file__).parent.parent.parent.parent

    def test_kani_perf_fail(self):
        cmd = (
            "rm -rf build target &&"
            "mkdir -p build/tests/perf/Unwind-Attribute/expected &&"
            "kani tests/kani/Unwind-Attribute/fixme_lib.rs > "
            "build/tests/perf/Unwind-Attribute/expected/expected.out"
        )
        self._run_kani_perf_test(cmd, False)

    def test_kani_perf_success(self):
        cmd = (
            "rm -rf build target &&"
            "mkdir -p build/tests/perf/Arbitrary/expected &&"
            "kani tests/kani/Arbitrary/arbitrary_impls.rs > "
            "build/tests/perf/Arbitrary/expected/expected.out"
        )
        self._run_kani_perf_test(cmd, True)

    def _run_kani_perf_test(self, command, expected_pass):
        """Ensure that the kani_perf parser can parse the output of a perf test"""

        # The two variants are identical; we're not actually checking the
        # returned metrics in this test, only checking that the parser works
        run_bc = Benchcomp({
            "variants": {
                "run_1": {
                    "config": {
                        "directory": str(self.kani_dir),
                        "command_line": command,
                    },
                },
                "run_2": {
                    "config": {
                        "directory": str(self.kani_dir),
                        "command_line": command,
                    },
                },
            },
            "run": {
                "suites": {
                    "suite_1": {
                        "parser": { "module": "kani_perf" },
                        "variants": ["run_1", "run_2"]
                    }
                }
            },
            "visualize": [{"type": "dump_yaml"}],
        })
        run_bc()
        self.assertEqual(run_bc.proc.returncode, 0, msg=run_bc.stderr)

        results = yaml.safe_load(run_bc.stdout)

        expected_types = {
            "solver_runtime": float,
            "symex_runtime": float,
            "verification_time": float,
            "success": bool,
        }

        all_succeeded = True

        for _, bench in results["benchmarks"].items():
            for _, variant in bench["variants"].items():

                all_succeeded &= variant["metrics"]["success"]

                for metric, ttype in expected_types.items():
                    self.assertIn(metric, variant["metrics"], msg=run_bc.stdout)
                    self.assertTrue(
                        isinstance(variant["metrics"][metric], ttype),
                        msg=run_bc.stdout)

        self.assertEqual(expected_pass, all_succeeded, msg=run_bc.stdout)

    def test_error_on_regression_two_benchmarks_previously_failed(self):
        """Ensure that benchcomp terminates with exit of 0 when the "error_on_regression" visualization is configured and one of the benchmarks continues to fail (no regression)."""

        with tempfile.TemporaryDirectory() as tmp:
            run_bc = Benchcomp({
                "variants": {
                    "passed": {
                        "config": {
                            "directory": str(tmp),
                            "command_line":
                                "mkdir bench_1 bench_2 && "
                                "echo true > bench_1/success &&"
                                "echo false > bench_2/success"
                        },
                    },
                    "failed": {
                        "config": {
                            "directory": str(tmp),
                            "command_line":
                                "mkdir bench_1 bench_2 && "
                                "echo true > bench_1/success &&"
                                "echo false > bench_2/success"
                        }
                    }
                },
                "run": {
                    "suites": {
                        "suite_1": {
                            "parser": { "module": "test_file_to_metric" },
                            "variants": ["passed", "failed"]
                        }
                    }
                },
                "visualize": [{
                    "type": "error_on_regression",
                    "variant_pairs": [["passed", "failed"]],
                    "checks": [{
                        "metric": "success",
                        "test":
                            "lambda old, new: False if not old else not new"
                    }]
                }]
            })
            run_bc()
            self.assertEqual(
                run_bc.proc.returncode, 0, msg=run_bc.stderr)


    def test_error_on_regression_two_benchmarks_one_failed(self):
        """Ensure that benchcomp terminates with exit of 1 when the "error_on_regression" visualization is configured and one of the benchmarks' success metric has regressed"""

        with tempfile.TemporaryDirectory() as tmp:
            run_bc = Benchcomp({
                "variants": {
                    "passed": {
                        "config": {
                            "directory": str(tmp),
                            "command_line":
                                "mkdir bench_1 bench_2 && "
                                "echo true > bench_1/success &&"
                                "echo true > bench_2/success"
                        },
                    },
                    "failed": {
                        "config": {
                            "directory": str(tmp),
                            "command_line":
                                "mkdir bench_1 bench_2 && "
                                "echo true > bench_1/success &&"
                                "echo false > bench_2/success"
                        }
                    }
                },
                "run": {
                    "suites": {
                        "suite_1": {
                            "parser": { "module": "test_file_to_metric" },
                            "variants": ["passed", "failed"]
                        }
                    }
                },
                "visualize": [{
                    "type": "error_on_regression",
                    "variant_pairs": [["passed", "failed"]],
                    "checks": [{
                        "metric": "success",
                        "test":
                            "lambda old, new: False if not old else not new"
                    }]
                }]
            })
            run_bc()
            self.assertEqual(
                run_bc.proc.returncode, 1, msg=run_bc.stderr)


    def test_error_on_regression_visualization_success_regressed(self):
        """Ensure that benchcomp terminates with exit of 1 when the "error_on_regression" visualization is configured and one of the benchmarks' success metric has regressed"""

        with tempfile.TemporaryDirectory() as tmp:
            run_bc = Benchcomp({
                "variants": {
                    "passed": {
                        "config": {
                            "directory": str(tmp),
                            "command_line": "mkdir bench_1 && echo true > bench_1/success"
                        },
                    },
                    "failed": {
                        "config": {
                            "directory": str(tmp),
                            "command_line": "mkdir bench_1 && echo false > bench_1/success"
                        }
                    }
                },
                "run": {
                    "suites": {
                        "suite_1": {
                            "parser": { "module": "test_file_to_metric" },
                            "variants": ["passed", "failed"]
                        }
                    }
                },
                "visualize": [{
                    "type": "error_on_regression",
                    "variant_pairs": [["passed", "failed"]],
                    "checks": [{
                        "metric": "success",
                        "test":
                            "lambda old, new: False if not old else not new"
                    }]
                }]
            })
            run_bc()
            self.assertEqual(
                run_bc.proc.returncode, 1, msg=run_bc.stderr)


    def test_error_on_regression_visualization_success_no_regressed(self):
        """Ensure that benchcomp terminates with exit of 0 when the "error_on_regression" visualization is configured and none of the benchmarks' success metric has regressed"""

        with tempfile.TemporaryDirectory() as tmp:
            run_bc = Benchcomp({
                "variants": {
                    "passed": {
                        "config": {
                            "directory": str(tmp),
                            "command_line": "mkdir bench_1 && echo true > bench_1/success"
                        },
                    },
                    "failed": {
                        "config": {
                            "directory": str(tmp),
                            "command_line": "mkdir bench_1 && echo true > bench_1/success"
                        }
                    }
                },
                "run": {
                    "suites": {
                        "suite_1": {
                            "parser": { "module": "test_file_to_metric" },
                            "variants": ["passed", "failed"]
                        }
                    }
                },
                "visualize": [{
                    "type": "error_on_regression",
                    "variant_pairs": [["passed", "failed"]],
                    "checks": [{
                        "metric": "success",
                        "test":
                            "lambda old, new: False if not old else not new"
                    }]
                }]
            })
            run_bc()
            self.assertEqual(
                run_bc.proc.returncode, 0, msg=run_bc.stderr)


    def test_error_on_regression_visualization_ratio_no_regressed(self):
        """Ensure that benchcomp terminates with exit of 0 when the "error_on_regression" visualization is configured and none of the metrics regressed"""

        with tempfile.TemporaryDirectory() as tmp:
            run_bc = Benchcomp({
                "variants": {
                    "more": {
                        "config": {
                            "directory": str(tmp),
                            "command_line": "mkdir bench_1 && echo 10 > bench_1/n_bugs"
                        },
                    },
                    "less": {
                        "config": {
                            "directory": str(tmp),
                            "command_line": "mkdir bench_1 && echo 5 > bench_1/n_bugs"
                        }
                    }
                },
                "run": {
                    "suites": {
                        "suite_1": {
                            "parser": { "module": "test_file_to_metric" },
                            "variants": ["less", "more"]
                        }
                    }
                },
                "visualize": [{
                    "type": "error_on_regression",
                    "variant_pairs": [["more", "less"]],
                    "checks": [{
                        "metric": "n_bugs",
                        "test": "lambda old, new: new / old > 1.75",
                    }]
                }]
            })
            run_bc()
            self.assertEqual(
                run_bc.proc.returncode, 0, msg=run_bc.stderr)


    def test_error_on_regression_visualization_ratio_regressed(self):
        """Ensure that benchcomp terminates with exit of 1 when the "error_on_regression" visualization is configured and one of the metrics regressed"""

        with tempfile.TemporaryDirectory() as tmp:
            run_bc = Benchcomp({
                "variants": {
                    "more": {
                        "config": {
                            "directory": str(tmp),
                            "command_line": "mkdir bench_1 && echo 10 > bench_1/n_bugs"
                        },
                    },
                    "less": {
                        "config": {
                            "directory": str(tmp),
                            "command_line": "mkdir bench_1 && echo 5 > bench_1/n_bugs"
                        }
                    }
                },
                "run": {
                    "suites": {
                        "suite_1": {
                            "parser": { "module": "test_file_to_metric" },
                            "variants": ["less", "more"]
                        }
                    }
                },
                "visualize": [{
                    "type": "error_on_regression",
                    "variant_pairs": [["less", "more"]],
                    "checks": [{
                        "metric": "n_bugs",
                        "test": "lambda old, new: new / old > 1.75",
                    }]
                }]
            })
            run_bc()
            self.assertEqual(
                run_bc.proc.returncode, 1, msg=run_bc.stderr)



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
