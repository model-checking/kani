# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import subprocess
import atexit
import os
import os.path
import re


RMC_CFG = "rmc"
RMC_RUSTC_EXE = "rmc-rustc"
EXIT_CODE_SUCCESS = 0

MEMORY_SAFETY_CHECKS = ["--pointer-check",
                        "--pointer-primitive-check",
                        "--bounds-check"]
OVERFLOW_CHECKS = ["--conversion-check",
                   "--div-by-zero-check",
                   "--float-overflow-check",
                   "--nan-check",
                   "--pointer-overflow-check",
                   "--signed-overflow-check",
                   "--undefined-shift-check",
                   "--unsigned-overflow-check"]
UNWINDING_CHECKS = ["--unwinding-assertions"]

# A Scanner is intended to match a pattern with an output
# and edit the output based on an edit function
class Scanner:
    def __init__(self, pattern, edit_fun):
        self.pattern  = re.compile(pattern)
        self.edit_fun = edit_fun

    # Returns whether the scanner's pattern matches some text
    def match(self, text):
        return self.pattern.search(text) != None

    # Returns an edited output based on the scanner's edit function
    def edit_output(self, text):
        return self.edit_fun(text)

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

# Add a set of CBMC flags to the CBMC arguments
def add_set_cbmc_flags(args, flags):
    # We print a warning if the user has passed the flag via `cbmc_args`
    # Otherwise we append it to the CBMC arguments
    for arg in flags:
        # This behavior must be reviewed if the set of flags is extended
        if arg in args.cbmc_args:
            print("WARNING: Default CBMC argument `{}` not added (already specified)".format(arg))
        else:
            args.cbmc_args.append(arg)

# Add sets of default CBMC flags
def add_default_cbmc_flags(args):
    if not args.no_memory_safety_checks:
        add_set_cbmc_flags(args, MEMORY_SAFETY_CHECKS)
    if not args.no_overflow_checks:
        add_set_cbmc_flags(args, OVERFLOW_CHECKS)
    if not args.no_unwinding_checks:
        add_set_cbmc_flags(args, UNWINDING_CHECKS)

def add_rmc_rustc_debug_to_env(env):
    env["RUSTC_LOG"] = env.get("RUSTC_LOG", "rustc_codegen_llvm::gotoc")

def print_rmc_step_status(step_name, completed_process, verbose=False):
    status = "PASS"
    if completed_process.returncode != EXIT_CODE_SUCCESS:
        status = "FAIL"
    if verbose:
        print(f"[RMC] stage: {step_name} ({status})")
        print(f"[RMC] cmd: {' '.join(completed_process.args)}")

def run_cmd(cmd, label=None, cwd=None, env=None, output_to=None, quiet=False, verbose=False, debug=False, scanners=[], dry_run=False):
    if dry_run:
        print(' '.join(cmd))
        return EXIT_CODE_SUCCESS

    process = subprocess.run(
        cmd, universal_newlines=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        env=env, cwd=cwd)
    
    # Print status
    if label != None:
        print_rmc_step_status(label, process, verbose)
    
    stdout = process.stdout
    for scanner in scanners:
        if scanner.match(stdout):
            stdout = scanner.edit_output(stdout)

    # Write to stdout if specified, or if failure, or verbose or debug
    if (output_to == "stdout" or process.returncode != EXIT_CODE_SUCCESS or verbose or debug) and not quiet:
        print(stdout)
    
    # Write to file if given
    if output_to != None and output_to != "stdout":
        with open(output_to, "w") as f:
            f.write(stdout)
    
    return process.returncode

def compile_single_rust_file(input_filename, output_filename, verbose=False, debug=False, keep_temps=False, mangler="v0", dry_run=False):
    if not keep_temps:
        atexit.register(delete_file, output_filename)
    build_cmd = [RMC_RUSTC_EXE, "-Z", "codegen-backend=gotoc", "-Z", f"symbol-mangling-version={mangler}",
                 f"--cfg={RMC_CFG}", "-o", output_filename, input_filename]
    build_env = os.environ
    if debug:
        add_rmc_rustc_debug_to_env(build_env)

    return run_cmd(build_cmd, env=build_env, label="compile", verbose=verbose, debug=debug, dry_run=dry_run)


def cargo_build(crate, target_dir="target", verbose=False, debug=False, mangler="v0", dry_run=False):
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
    return run_cmd(build_cmd, env=build_env, cwd=crate, label="build", verbose=verbose, debug=debug, dry_run=dry_run)

def append_unwind_tip(text):
    unwind_tip = ("[RMC] info: Verification output shows one or more unwinding failures.\n"
                  "[RMC] tip: Consider increasing the unwinding value or disabling `--unwinding-assertions`.\n")
    return text + unwind_tip

def symbol_table_to_gotoc(json_filename, cbmc_filename, verbose=False, keep_temps=False, dry_run=False):
    if not keep_temps:
        atexit.register(delete_file, cbmc_filename)
    cmd = ["symtab2gb", json_filename, "--out", cbmc_filename]
    return run_cmd(cmd, label="to-gotoc", verbose=verbose, dry_run=dry_run)

def run_cbmc(cbmc_filename, cbmc_args, verbose=False, quiet=False, dry_run=False):
    cbmc_cmd = ["cbmc"] + cbmc_args + [cbmc_filename]
    scanners = []
    if "--unwinding-assertions" in cbmc_args:
        # Pass a scanner that shows a tip if the CBMC output contains unwinding failures
        unwind_asserts_pattern = ".*unwinding assertion.*: FAILURE"
        unwind_asserts_scanner = Scanner(unwind_asserts_pattern, append_unwind_tip)
        scanners.append(unwind_asserts_scanner)
    return run_cmd(cbmc_cmd, label="cbmc", output_to="stdout", verbose=verbose, quiet=quiet, scanners=scanners, dry_run=dry_run)

def run_visualize(cbmc_filename, cbmc_args, verbose=False, quiet=False, keep_temps=False, function="main", srcdir=".", wkdir=".", outdir=".", dry_run=False):
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

    run_goto_cc(cbmc_filename, temp_goto_filename, verbose, quiet, function=function, dry_run=dry_run)
    
    def run_cbmc_local(cbmc_args, output_to, dry_run=False):
        cbmc_cmd = ["cbmc"] + cbmc_args + [temp_goto_filename]
        return run_cmd(cbmc_cmd, label="cbmc", output_to=output_to, verbose=verbose, quiet=quiet, dry_run=dry_run)

    cbmc_xml_args = cbmc_args + ["--xml-ui"]
    retcode = run_cbmc_local(cbmc_xml_args + ["--trace"], results_filename, dry_run=dry_run)
    run_cbmc_local(cbmc_xml_args + ["--cover", "location"], coverage_filename, dry_run=dry_run)
    run_cbmc_local(cbmc_xml_args + ["--show-properties"], property_filename, dry_run=dry_run)

    run_cbmc_viewer(temp_goto_filename, results_filename, coverage_filename,
                    property_filename, verbose, quiet, srcdir, wkdir, os.path.join(outdir, "report"), dry_run=dry_run)

    return retcode

def run_goto_cc(src, dst, verbose=False, quiet=False, function="main", dry_run=False):
    cmd = ["goto-cc"] + ["--function", function] + [src] + ["-o", dst]
    return run_cmd(cmd, label="goto-cc", verbose=verbose, quiet=quiet, dry_run=dry_run)

def run_cbmc_viewer(goto_filename, results_filename, coverage_filename, property_filename, verbose=False, quiet=False, srcdir=".", wkdir=".", reportdir="report", dry_run=False):
    cmd = ["cbmc-viewer"] + \
          ["--result", results_filename] + \
          ["--coverage", coverage_filename] + \
          ["--property", property_filename] + \
          ["--srcdir", os.path.realpath(srcdir)] + \
          ["--wkdir", os.path.realpath(wkdir)] + \
          ["--goto", goto_filename] + \
          ["--reportdir", reportdir]
    return run_cmd(cmd, label="cbmc-viewer", verbose=verbose, quiet=quiet, dry_run=dry_run)


def run_goto_instrument(input_filename, output_filename, args, verbose=False, dry_run=False):
    cmd = ["goto-instrument"] + args + [input_filename]
    return run_cmd(cmd, label="goto-instrument", verbose=verbose, output_to=output_filename, dry_run=dry_run)


def goto_to_c(goto_filename, c_filename, verbose=False, dry_run=False):
    return run_goto_instrument(goto_filename, c_filename, ["--dump-c"], verbose, dry_run=dry_run)


def goto_to_symbols(goto_filename, symbols_filename, verbose=False, dry_run=False):
    return run_goto_instrument(goto_filename, symbols_filename, ["--show-symbol-table"], verbose, dry_run=dry_run)
