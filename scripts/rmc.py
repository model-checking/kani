# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import subprocess
import atexit
import os
import os.path


RMC_CFG = "rmc"
RMC_RUSTC_EXE = "rmc-rustc"
EXIT_CODE_SUCCESS = 0


def is_exe(name):
    from shutil import which
    return which(name) is not None


def dependencies_in_path():
    for program in [RMC_RUSTC_EXE, "symtab2gb", "cbmc", "cbmc-viewer", "goto-instrument", "goto-cc"]:
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

def run_cmd(cmd, label=None, cwd=None, env=None, output_to=None, quiet=False, verbose=False, debug=False):
    process = subprocess.run(
        cmd, universal_newlines=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        env=env, cwd=cwd)
    
    # Print status
    if label != None:
        print_rmc_step_status(label, process, verbose)
    
    # Write to stdout if specified, or if failure, or verbose or debug
    if (output_to == "stdout" or process.returncode != EXIT_CODE_SUCCESS or verbose or debug) and not quiet:
        print(process.stdout)
    
    # Write to file if given
    if output_to != None and output_to != "stdout":
        with open(output_to, "w") as f:
            f.write(process.stdout)
    
    return process.returncode

def compile_single_rust_file(input_filename, output_filename, verbose=False, debug=False, keep_temps=False, mangler="v0"):
    if not keep_temps:
        atexit.register(delete_file, output_filename)
    build_cmd = [RMC_RUSTC_EXE, "-Z", "codegen-backend=gotoc", "-Z", f"symbol-mangling-version={mangler}",
                 f"--cfg={RMC_CFG}", "-o", output_filename, input_filename]
    build_env = os.environ
    if debug:
        add_rmc_rustc_debug_to_env(build_env)

    return run_cmd(build_cmd, env=build_env, label="compile", verbose=verbose, debug=debug)


def cargo_build(crate, target_dir="target", verbose=False, debug=False, mangler="v0"):
    rustflags = ["-Z", "codegen-backend=gotoc", "-Z", f"symbol-mangling-version={mangler}", f"--cfg={RMC_CFG}"]
    rustflags = " ".join(rustflags)
    if "RUSTFLAGS" in os.environ:
        rustflags = os.environ["RUSTFLAGS"] + " " + rustflags

    build_cmd = ["cargo", "build", "--target-dir", target_dir]
    build_env = {"RUSTFLAGS": rustflags,
                 "RUSTC": RMC_RUSTC_EXE,
                 "PATH": os.environ["PATH"],
                 }
    if debug:
        add_rmc_rustc_debug_to_env(build_env)
    return run_cmd(build_cmd, env=build_env, cwd=crate, label="build", verbose=verbose, debug=debug)


def symbol_table_to_gotoc(json_filename, cbmc_filename, verbose=False, keep_temps=False):
    if not keep_temps:
        atexit.register(delete_file, cbmc_filename)
    cmd = ["symtab2gb", json_filename, "--out", cbmc_filename]
    return run_cmd(cmd, label="to-gotoc", verbose=verbose)

def run_cbmc(cbmc_filename, cbmc_args, verbose=False, quiet=False):
    cbmc_cmd = ["cbmc"] + cbmc_args + [cbmc_filename]
    return run_cmd(cbmc_cmd, label="cbmc", output_to="stdout", verbose=verbose, quiet=quiet)

def run_visualize(cbmc_filename, cbmc_args, verbose=False, quiet=False, keep_temps=False, function="main", srcdir=".", wkdir=".", outdir="."):
    results_filename = os.path.join(outdir, "results.xml")
    coverage_filename = os.path.join(outdir, "coverage.xml")
    property_filename = os.path.join(outdir, "property.xml")
    temp_goto_filename = os.path.join(outdir, "temp.goto")
    if not keep_temps:
        for filename in [results_filename, coverage_filename, property_filename, temp_goto_filename]:
            atexit.register(delete_file, filename)

    # 1) goto-cc --function main <cbmc_filename> -o temp.goto
    # 2) cbmc --xml-ui --trace ~/rmc/library/rmc/rmc_lib.c temp.goto > results.xml
    # 3) cbmc --xml-ui --cover location ~/rmc/library/rmc/rmc_lib.c temp.goto > coverage.xml
    # 4) cbmc --xml-ui --show-properties ~/rmc/library/rmc/rmc_lib.c temp.goto > property.xml
    # 5) cbmc-viewer --result results.xml --coverage coverage.xml --property property.xml --srcdir . --goto temp.goto --reportdir report

    run_goto_cc(cbmc_filename, temp_goto_filename, verbose, quiet)
    
    def run_cbmc_local(cbmc_args, output_to):
        cbmc_cmd = ["cbmc"] + cbmc_args + [temp_goto_filename]
        return run_cmd(cbmc_cmd, label="cbmc", output_to=output_to, verbose=verbose, quiet=quiet)

    cbmc_xml_args = cbmc_args + ["--xml-ui"]
    retcode = run_cbmc_local(cbmc_xml_args + ["--trace"], results_filename)
    run_cbmc_local(cbmc_xml_args + ["--cover", "location"], coverage_filename)
    run_cbmc_local(cbmc_xml_args + ["--show-properties"], property_filename)

    run_cbmc_viewer(temp_goto_filename, results_filename, coverage_filename,
                    property_filename, verbose, quiet, srcdir, wkdir, os.path.join(outdir, "report"))

    return retcode

def run_goto_cc(src, dst, verbose=False, quiet=False, function="main"):
    cmd = ["goto-cc"] + ["--function", function] + [src] + ["-o", dst]
    return run_cmd(cmd, label="goto-cc", verbose=verbose, quiet=quiet)

def run_cbmc_viewer(goto_filename, results_filename, coverage_filename, property_filename, verbose=False, quiet=False, srcdir=".", wkdir=".", reportdir="report"):
    cmd = ["cbmc-viewer"] + \
          ["--result", results_filename] + \
          ["--coverage", coverage_filename] + \
          ["--property", property_filename] + \
          ["--srcdir", os.path.realpath(srcdir)] + \
          ["--wkdir", os.path.realpath(wkdir)] + \
          ["--goto", goto_filename] + \
          ["--reportdir", reportdir]
    return run_cmd(cmd, label="cbmc-viewer", verbose=verbose, quiet=quiet)


def run_goto_instrument(input_filename, output_filename, args, verbose=False):
    cmd = ["goto-instrument"] + args + [input_filename]
    return run_cmd(cmd, label="goto-instrument", verbose=verbose, output_to=output_filename)


def goto_to_c(goto_filename, c_filename, verbose=False):
    return run_goto_instrument(goto_filename, c_filename, ["--dump-c"], verbose)


def goto_to_symbols(goto_filename, symbols_filename, verbose=False):
    return run_goto_instrument(goto_filename, symbols_filename, ["--show-symbol-table"], verbose)
