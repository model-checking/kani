# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import subprocess
import atexit
import os


RMC_CFG = "rmc"
RMC_RUSTC_EXE = "rmc-rustc"
EXIT_CODE_SUCCESS = 0


def is_exe(name):
    from shutil import which
    return which(name) is not None


def dependencies_in_path():
    for program in [RMC_RUSTC_EXE, "symtab2gb", "cbmc", "goto-instrument"]:
        if not is_exe(program):
            print("ERROR: Could not find {} in PATH".format(program))
            return False
    return True


def delete_file(filename):
    try:
        os.remove(filename)
    except OSError:
        pass


def add_rmc_rustc_debug_to_env(env):
    env["RUSTC_LOG"] = env.get("RUSTC_LOG", "rustc_codegen_llvm::gotoc")


def print_rmc_step_status(step_name, completed_process, verbose=False):
    status = "PASS"
    if completed_process.returncode != EXIT_CODE_SUCCESS:
        status = "FAIL"
    if verbose:
        print(f"[RMC] stage: {step_name} ({status})")
        print(f"[RMC] cmd: {' '.join(completed_process.args)}")


def compile_single_rust_file(input_filename, output_filename, verbose=False, debug=False, keep_temps=False):
    if not keep_temps:
        atexit.register(delete_file, output_filename)
    build_cmd = [RMC_RUSTC_EXE, "-Z", "codegen-backend=gotoc",
                 f"--cfg={RMC_CFG}", "-o", output_filename, input_filename]
    build_env = os.environ
    if debug:
        add_rmc_rustc_debug_to_env(build_env)
    process = subprocess.run(build_cmd, universal_newlines=True, env=build_env,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print_rmc_step_status("compile", process, verbose)
    if process.returncode != EXIT_CODE_SUCCESS or verbose or debug:
        print(process.stdout)
    return process.returncode


def cargo_build(crate, verbose=False, debug=False):
    rustflags = ["-Z", "codegen-backend=gotoc", f"--cfg={RMC_CFG}"]
    rustflags = " ".join(rustflags)
    if "RUSTFLAGS" in os.environ:
        rustflags = os.environ["RUSTFLAGS"] + " " + rustflags

    build_cmd = ["cargo", "build"]
    build_env = {"RUSTFLAGS": rustflags,
                 "RUSTC": RMC_RUSTC_EXE,
                 "PATH": os.environ["PATH"],
                 }
    if debug:
        add_rmc_rustc_debug_to_env(build_env)
    process = subprocess.run(build_cmd, universal_newlines=True, cwd=crate,
                             env=build_env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print_rmc_step_status("build", process, verbose)
    if process.returncode != EXIT_CODE_SUCCESS or verbose or debug:
        print(process.stdout)
    return process.returncode


def symbol_table_to_gotoc(json_filename, cbmc_filename, verbose=False, keep_temps=False):
    if not keep_temps:
        atexit.register(delete_file, cbmc_filename)
    cmd = ["symtab2gb", json_filename, "--out", cbmc_filename]
    process = subprocess.run(
        cmd, universal_newlines=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print_rmc_step_status("to-gotoc", process, verbose)
    if process.returncode != EXIT_CODE_SUCCESS or verbose:
        print(process.stdout)
    return process.returncode


def run_cbmc(cbmc_filename, cbmc_args, verbose=False, quiet=False):
    cbmc_cmd = ["cbmc"] + cbmc_args + [cbmc_filename]
    process = subprocess.Popen(cbmc_cmd, universal_newlines=True,
                               stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT)
    if not quiet:
        for line in iter(process.stdout.readline, ""):
            print(line, end="")
    process.stdout.close()
    retcode = process.wait()
    print_rmc_step_status("cbmc", process, verbose)
    return retcode


def run_goto_instrument(input_filename, output_filename, args, verbose=False):
    cmd = ["goto-instrument"] + args + [input_filename]
    with open(output_filename, "w") as f:
        process = subprocess.run(
            cmd, universal_newlines=True, stdout=f, stderr=subprocess.PIPE)
    print_rmc_step_status("goto-instrument", process, verbose)
    if process.returncode != 0 or verbose:
        print(process.stderr)
    return process.returncode


def goto_to_c(goto_filename, c_filename, verbose=False):
    return run_goto_instrument(goto_filename, c_filename, ["--dump-c"], verbose)


def goto_to_symbols(goto_filename, symbols_filename, verbose=False):
    return run_goto_instrument(goto_filename, symbols_filename, ["--show-symbol-table"], verbose)
