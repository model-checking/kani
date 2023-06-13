# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Each *Parser class here specifies a different way that a parser can be
# invoked: as an executable (for parsers that users write on their local
# machine) or python module (that is checked into the Kani codebase).

# Each class has a __call__ method that takes a directory. The directory should
# be a benchmark suite that has completed a run. The __call__ method parses and
# returns the result of the run (by parsing output files in the directory etc).


import dataclasses
import subprocess
import logging
import importlib
import sys

import yaml


def get_parser(parser_config):
    if "module" in parser_config:
        return _ModuleParser(parser_config["module"])
    if "command" in parser_config:
        return _CommandParser(parser_config["command"])

    logging.error(
        "Parser dict should contain either a"
        "'module' or 'command' key: '%s'", str(parser_config))
    sys.exit(1)



class _ModuleParser:
    """A parser implemented as a module under benchcomp.parsers"""

    def __init__(self, mod):
        self.parser_mod_name = f"benchcomp.parsers.{mod}"
        try:
            self.parser = importlib.import_module(self.parser_mod_name)
        except BaseException as exe:
            logging.error(
                "Failed to load parser module %s: %s",
                self.parser_mod_name, str(exe))
            sys.exit(1)


    def __call__(self, root_directory):
        try:
            return self.parser.main(root_directory)
        except BaseException as exe:
            logging.error(
                "Parser '%s' in directory %s failed: %s",
                self.parser_mod_name, str(root_directory), str(exe))
            return _empty_parser_result()



@dataclasses.dataclass
class _CommandParser:
    """A parser that is a command that prints the parse result to stdout"""

    shell_cmd: str


    def __call__(self, root_directory):
        try:
            with subprocess.Popen(
                    self.shell_cmd, shell=True, text=True,
                    stdout=subprocess.PIPE, cwd=root_directory) as proc:
                out, _ = proc.communicate(timeout=120)
        except subprocess.CalledProcessError as exc:
            logging.warning(
                "Invocation of parser '%s' in directory %s exited with code %d",
                self.shell_cmd, str(root_directory), exc.returncode)
            return _empty_parser_result()
        except (OSError, subprocess.SubprocessError) as exe:
            logging.error(
                "Invocation of parser '%s' in directory %s failed: %s",
                self.shell_cmd, str(root_directory), str(exe))
            return _empty_parser_result()

        try:
            return yaml.safe_load(out)
        except yaml.YAMLError:
            logging.error(
                "Parser '%s' in directory %s printed invalid YAML:<%s>",
                self.shell_cmd, str(root_directory), out)
            return _empty_parser_result()



def _empty_parser_result():
    return {
        "benchmarks": {},
        "metrics": {},
    }
