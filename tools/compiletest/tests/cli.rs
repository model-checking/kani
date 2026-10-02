// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::process::Command;

// Regression for #1219: both help branches must print usage and exit successfully.
#[test]
fn help_exits_successfully() {
    for args in [
        vec!["--help"],
        vec!["-h"],
        vec!["--suite", "kani", "--help"],
        vec!["--suite", "kani", "-h"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_compiletest")).args(&args).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{args:?}: {}\n{stdout}\n{stderr}", output.status);
        assert!(stdout.contains("Usage:"), "{args:?}: {stdout}");
        assert!(stdout.contains("--help"), "{args:?}: {stdout}");
        assert!(!stderr.contains("panicked"), "{args:?}: {stderr}");
    }
}
