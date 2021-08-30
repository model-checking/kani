# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import subprocess
import atexit
import os
import os.path
import sys
import re
import pathlib


RMC_CFG = "rmc"
RMC_RUSTC_EXE = "rmc-rustc"
MY_PATH = pathlib.Path(__file__).parent.parent.absolute()
GEN_C_LIB = MY_PATH / "library" / "rmc" / "gen_c_lib.c"
EXIT_CODE_SUCCESS = 0
CBMC_VERIFICATION_FAILURE_EXIT_CODE = 10

MEMORY_SAFETY_CHECKS = ["--bounds-check",
                        "--pointer-check",
                        "--pointer-primitive-check"]
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
    
def ensure_dependencies_in_path():
    for program in [RMC_RUSTC_EXE, "symtab2gb", "cbmc", "cbmc-viewer", "goto-instrument", "goto-cc"]:
        ensure(is_exe(program), f"Could not find {program} in PATH")

# Assert a condition holds, or produce a user error message.
def ensure(condition, message=None, retcode=1):
    if not condition:
        if message:
            print(f"ERROR: {message}")
        sys.exit(retcode)

# Deletes a file; used by atexit.register to remove temporary files on exit
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

# Add sets of selected default CBMC flags
def add_selected_default_cbmc_flags(args):
    if args.memory_safety_checks:
        add_set_cbmc_flags(args, MEMORY_SAFETY_CHECKS)
    if args.overflow_checks:
        add_set_cbmc_flags(args, OVERFLOW_CHECKS)
    if args.unwinding_checks:
        add_set_cbmc_flags(args, UNWINDING_CHECKS)

# Updates environment to use gotoc backend debugging
def add_rmc_rustc_debug_to_env(env):
    env["RUSTC_LOG"] = env.get("RUSTC_LOG", "rustc_codegen_llvm::gotoc")

# Prints info about the RMC process
def print_rmc_step_status(step_name, completed_process, verbose=False):
    status = "PASS"
    if completed_process.returncode != EXIT_CODE_SUCCESS:
        status = "FAIL"
    if verbose:
        print(f"[RMC] stage: {step_name} ({status})")
        print(f"[RMC] cmd: {' '.join(completed_process.args)}")

# Handler for running arbitrary command-line commands
def run_cmd(cmd, label=None, cwd=None, env=None, output_to=None, quiet=False, verbose=False, debug=False, scanners=[], dry_run=False):
    # If this a dry run, we emulate running a successful process whose output is the command itself
    # We set `output_to` to `stdout` so that the output is not omitted below
    if dry_run:
        cmd_line = ' '.join(cmd)
        if output_to != "stdout" and output_to is not None:
            cmd_line += f" > \"{output_to}\""

        output_to = "stdout"

        process = subprocess.CompletedProcess(None, EXIT_CODE_SUCCESS, stdout=cmd_line)
    else:
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

# Generates a symbol table from a rust file
def compile_single_rust_file(input_filename, output_filename, verbose=False, debug=False, keep_temps=False, mangler="v0", dry_run=False, use_abs=False, abs_type="std", symbol_table_passes=[]):
    if not keep_temps:
        atexit.register(delete_file, output_filename)

    build_cmd = [RMC_RUSTC_EXE, 
            "-Z", "codegen-backend=gotoc", 
            "-Z", "trim-diagnostic-paths=no",
            "-Z", f"symbol-mangling-version={mangler}", 
            "-Z", f"symbol_table_passes={' '.join(symbol_table_passes)}",
            f"--cfg={RMC_CFG}"]

    if use_abs:
        build_cmd += ["-Z", "force-unstable-if-unmarked=yes",
                "--cfg=use_abs",
                "--cfg", f'abs_type="{abs_type}"']

    build_cmd += ["-o", output_filename, input_filename]

    if "RUSTFLAGS" in os.environ:
        build_cmd += os.environ["RUSTFLAGS"].split(" ")
    build_env = os.environ
    if debug:
        add_rmc_rustc_debug_to_env(build_env)

    return run_cmd(build_cmd, env=build_env, label="compile", verbose=verbose, debug=debug, dry_run=dry_run)

# Generates a symbol table (and some other artifacts) from a rust crate
def cargo_build(crate, target_dir="target", verbose=False, debug=False, mangler="v0", dry_run=False, symbol_table_passes=[]):
    ensure(os.path.isdir(crate), f"Invalid path to crate: {crate}")

    rustflags = [
        "-Z", "codegen-backend=gotoc",
        "-Z", "trim-diagnostic-paths=no",
        "-Z", f"symbol-mangling-version={mangler}",
        "-Z", f"symbol_table_passes={' '.join(symbol_table_passes)}", 
        f"--cfg={RMC_CFG}"]
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

# Adds information about unwinding to the RMC output
def append_unwind_tip(text):
    unwind_tip = ("[RMC] info: Verification output shows one or more unwinding failures.\n"
                  "[RMC] tip: Consider increasing the unwinding value or disabling `--unwinding-assertions`.\n")
    return text + unwind_tip

# Generates a goto program from a symbol table
def symbol_table_to_gotoc(json_filename, cbmc_filename, verbose=False, keep_temps=False, dry_run=False):
    if not keep_temps:
        atexit.register(delete_file, cbmc_filename)
    cmd = ["symtab2gb", json_filename, "--out", cbmc_filename]
    return run_cmd(cmd, label="to-gotoc", verbose=verbose, dry_run=dry_run)

# Links in external C programs into a goto program
def link_c_lib(src, dst, c_lib, verbose=False, quiet=False, function="main", dry_run=False):
    cmd = ["goto-cc"] + ["--function", function] + [src] + c_lib + ["-o", dst]
    return run_cmd(cmd, label="goto-cc", verbose=verbose, quiet=quiet, dry_run=dry_run)

# Runs CBMC on a goto program
def run_cbmc(cbmc_filename, cbmc_args, verbose=False, quiet=False, dry_run=False):
    cbmc_cmd = ["cbmc"] + cbmc_args + [cbmc_filename]
    scanners = []
    if "--unwinding-assertions" in cbmc_args:
        # Pass a scanner that shows a tip if the CBMC output contains unwinding failures
        unwind_asserts_pattern = ".*unwinding assertion.*: FAILURE"
        unwind_asserts_scanner = Scanner(unwind_asserts_pattern, append_unwind_tip)
        scanners.append(unwind_asserts_scanner)
    return run_cmd(cbmc_cmd, label="cbmc", output_to="stdout", verbose=verbose, quiet=quiet, scanners=scanners, dry_run=dry_run)

# Generates a viewer report from a goto program
def run_visualize(cbmc_filename, prop_args, cover_args, verbose=False, quiet=False, keep_temps=False, function="main", srcdir=".", wkdir=".", outdir=".", dry_run=False):
    results_filename = os.path.join(outdir, "results.xml")
    coverage_filename = os.path.join(outdir, "coverage.xml")
    property_filename = os.path.join(outdir, "property.xml")
    if not keep_temps:
        for filename in [results_filename, coverage_filename, property_filename]:
            atexit.register(delete_file, filename)

    # 1) cbmc --xml-ui --trace ~/rmc/library/rmc/rmc_lib.c <cbmc_filename> > results.xml
    # 2) cbmc --xml-ui --cover location ~/rmc/library/rmc/rmc_lib.c <cbmc_filename> > coverage.xml
    # 3) cbmc --xml-ui --show-properties ~/rmc/library/rmc/rmc_lib.c <cbmc_filename> > property.xml
    # 4) cbmc-viewer --result results.xml --coverage coverage.xml --property property.xml --srcdir . --goto <cbmc_filename> --reportdir report
    
    def run_cbmc_local(cbmc_args, output_to, dry_run=False):
        cbmc_cmd = ["cbmc"] + cbmc_args + [cbmc_filename]
        return run_cmd(cbmc_cmd, label="cbmc", output_to=output_to, verbose=verbose, quiet=quiet, dry_run=dry_run)

    cbmc_prop_args = prop_args + ["--xml-ui"]
    cbmc_cover_args = cover_args + ["--xml-ui"]

    retcode = run_cbmc_local(cbmc_prop_args + ["--trace"], results_filename, dry_run=dry_run)
    run_cbmc_local(cbmc_cover_args + ["--cover", "location"], coverage_filename, dry_run=dry_run)
    run_cbmc_local(cbmc_prop_args + ["--show-properties"], property_filename, dry_run=dry_run)

    run_cbmc_viewer(cbmc_filename, results_filename, coverage_filename,
                    property_filename, verbose, quiet, srcdir, wkdir, os.path.join(outdir, "report"), dry_run=dry_run)

    return retcode

# Handler for calling cbmc-viewer
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

# Handler for calling goto-instrument
def run_goto_instrument(input_filename, output_filename, args, verbose=False, dry_run=False):
    cmd = ["goto-instrument"] + args + [input_filename, output_filename]
    return run_cmd(cmd, label="goto-instrument", verbose=verbose, dry_run=dry_run)

# Generates a C program from a goto program
def goto_to_c(goto_filename, c_filename, verbose=False, dry_run=False):
    return run_goto_instrument(goto_filename, c_filename, ["--dump-c"], verbose, dry_run=dry_run)
    
# Fix remaining issues with output of --gen-c-runnable
def gen_c_postprocess(c_filename, dry_run=False):
    if not dry_run:
        with open(c_filename, "r") as f:
            lines = f.read().splitlines()

        # Import gen_c_lib.c
        lines.insert(0, f"#include \"{GEN_C_LIB}\"")

        # Convert back to string
        string_contents = "\n".join(lines)

        # Remove builtin macros
        to_remove = [
            # memcmp
            """// memcmp
// file <builtin-library-memcmp> function memcmp
int memcmp(void *, void *, unsigned long int);""",

            # memcpy
            """// memcpy
// file <builtin-library-memcpy> function memcpy
void * memcpy(void *, void *, unsigned long int);""",

            # memmove
            """// memmove
// file <builtin-library-memmove> function memmove
void * memmove(void *, void *, unsigned long int);""",

            # sputc
            """// __sputc
// file /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/stdio.h line 260
inline signed int __sputc(signed int _c, FILE *_p);""",

            """// __sputc
// file /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/stdio.h line 260
inline signed int __sputc(signed int _c, FILE *_p)
{
  signed int tmp_pre=_p->_w - 1;
  _p->_w = tmp_pre;
  __CPROVER_bool tmp_if_expr;
  if(tmp_pre >= 0)
    tmp_if_expr = 1;

  else
    tmp_if_expr = (_p->_w >= _p->_lbfsize ? ((signed int)(char)_c != 10 ? 1 : 0) : 0) ? 1 : 0;
  unsigned char *tmp_post;
  unsigned char tmp_assign;
  signed int return_value___swbuf;
  if(tmp_if_expr)
  {
    tmp_post = _p->_p;
    _p->_p = _p->_p + 1l;
    tmp_assign = (unsigned char)_c;
    *tmp_post = tmp_assign;
    return (signed int)tmp_assign;
  }

  else
  {
    return_value___swbuf=__swbuf(_c, _p);
    return return_value___swbuf;
  }
}"""
        ]

        for block in to_remove:
            string_contents = string_contents.replace(block, "")

        # Print back to file
        with open(c_filename, "w") as f:
            f.write(string_contents)


# Generates the CMBC symbol table from a goto program
def goto_to_symbols(goto_filename, symbols_filename, verbose=False, dry_run=False):
    return run_goto_instrument(goto_filename, symbols_filename, ["--show-symbol-table"], verbose, dry_run=dry_run)
