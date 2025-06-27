# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Entrypoint for `benchcomp filter`


import json
import logging
import pathlib
import subprocess
import sys
import tempfile

import yaml


def main(args):
    """Filter the results file by piping it into a list of scripts"""

    with open(args.result_file) as handle:
        old_results = yaml.safe_load(handle)

    if "filters" not in args.config:
        return old_results

    tmp_root = pathlib.Path(tempfile.gettempdir()) / "benchcomp" / "filter"
    tmp_root.mkdir(parents=True, exist_ok=True)
    tmpdir = pathlib.Path(tempfile.mkdtemp(dir=str(tmp_root)))

    for idx, filt in enumerate(args.config["filters"]):
        with open(args.result_file) as handle:
            old_results = yaml.safe_load(handle)

        json_results = json.dumps(old_results, indent=2)
        in_file = tmpdir / f"{idx}.in.json"
        out_file = tmpdir / f"{idx}.out.json"
        cmd_out = _pipe(
            filt["command_line"], json_results, in_file, out_file)

        try:
            new_results = yaml.safe_load(cmd_out)
        except yaml.YAMLError as exc:
            logging.exception(
                "Filter command '%s' produced invalid YAML. Stdin of"
                " the command is saved in %s, stdout is saved in %s.",
                filt["command_line"], in_file, out_file)
            if hasattr(exc, "problem_mark"):
                logging.error(
                    "Parse error location: line %d, column %d",
                    exc.problem_mark.line+1, exc.problem_mark.column+1)
            sys.exit(1)

        with open(args.result_file, "w") as handle:
            yaml.dump(new_results, handle, default_flow_style=False, indent=2)

        return new_results


def _pipe(shell_command, in_text, in_file, out_file):
    """Pipe `in_text` into `shell_command` and return the output text

    Save the in and out text into files for later inspection if necessary.
    """

    with open(in_file, "w") as handle:
        print(in_text, file=handle)

    logging.debug(
        "Piping the contents of '%s' into '%s', saving into '%s'",
        in_file, shell_command, out_file)

    timeout = 60
    with subprocess.Popen(
            shell_command, shell=True, text=True, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE) as proc:
        try:
            out, _ = proc.communicate(input=in_text, timeout=timeout)
        except subprocess.TimeoutExpired:
            logging.error(
                "Filter command failed to terminate after %ds: '%s'",
                timeout, shell_command)
            sys.exit(1)

    with open(out_file, "w") as handle:
        print(out, file=handle)

    if proc.returncode:
        logging.error(
            "Filter command '%s' exited with code %d. Stdin of"
            " the command is saved in %s, stdout is saved in %s.",
            shell_command, proc.returncode, in_file, out_file)
        sys.exit(1)

    return out
