// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session::KaniSession;

impl KaniSession {
    /// Given a set of goto binaries (`inputs`), produce `output` by linking everything
    /// together (including essential libraries). The result is generic over all proof harnesses.
    pub fn link_goto_binary(&self, inputs: &[PathBuf], output: &Path) -> Result<()> {
        let mut args: Vec<OsString> = Vec::new();
        args.extend(inputs.iter().map(|x| x.clone().into_os_string()));
        args.extend(self.args.c_lib.iter().map(|x| x.clone().into_os_string()));

        if self.args.is_cross_target() {
            // goto-cc compiles any C source it is given with the host's C configuration, and
            // that configuration then replaces the `__CPROVER_architecture_*` symbols that
            // kani-compiler wrote for the target. When it links goto binaries instead, the first
            // input's symbols win. So compile `kani_lib.c` on its own and link the object after
            // the Rust inputs, which keeps the target's machine model.
            //
            // The object is still compiled for the host. That is safe because, outside its
            // headers, `kani_lib.c` uses only `size_t`, `uint8_t` and pointers, never `int`,
            // `long double`, `wchar_t` or `char` (apart from its ASCII assertion messages), and
            // those three types are the same width on every target `check_target` accepts.
            // `--c-lib` is rejected with a cross target, because arbitrary C has no such limit.
            let kani_lib_o = output.with_extension("kani_lib.o");
            self.record_temporary_file(&kani_lib_o);
            let mut cmd = Command::new("goto-cc");
            cmd.arg("-c").arg(&self.kani_lib_c).arg("-o").arg(&kani_lib_o);
            self.run_suppress(cmd)?;
            args.push(kani_lib_o.into_os_string());
        } else {
            // `kani_lib.c` defines the allocator entry points (`__rust_alloc` and friends) that
            // rustc expects a backend to provide.
            args.push(self.kani_lib_c.clone().into_os_string());
        }

        args.push("-o".into());
        args.push(output.to_owned().into_os_string());

        // TODO get goto-cc path from self
        let mut cmd = Command::new("goto-cc");
        cmd.args(args);

        self.run_suppress(cmd)?;

        Ok(())
    }

    /// Produce a goto binary with its entry point set to a particular proof harness.
    pub fn specialize_to_proof_harness(
        &self,
        input: &Path,
        output: &Path,
        function: &str,
    ) -> Result<()> {
        let mut cmd = Command::new("goto-cc");
        cmd.arg(input).args(["--function", function, "-o"]).arg(output);

        self.run_suppress(cmd)?;

        Ok(())
    }
}
