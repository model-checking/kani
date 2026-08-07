// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::InvocationType;
use crate::util;
use std::process::Command;

const KANI_RUST_VERIFIER: &str = "Kani Rust Verifier";
/// We assume this is the same as the `kani-verifier` version, but we should
/// make sure it's enforced through CI:
/// <https://github.com/model-checking/kani/issues/2626>
pub(crate) const KANI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Name of the CBMC version variable in the `kani-dependencies` pin file.
const CBMC_VERSION_VAR: &str = "CBMC_VERSION";

/// Print Kani's startup banner, shown during a normal verification run. At
/// present, this is only release version information.
///
/// There are two call sites (`kani-driver/src/main.rs`), both reached during
/// a normal verification run and both gated behind `!args.common_args.quiet`,
/// so this -- including the CBMC pin mismatch warning below -- produces no
/// output at all under `--quiet`. That is a deliberate choice, not an
/// oversight: `--quiet` has an existing, tested contract that it produces
/// *zero* output (`tests/script-based-pre/check-quiet/check-quiet.sh`
/// asserts `wc -l` is 0). A quiet, unattended measurement campaign is exactly
/// the scenario the CBMC pin check exists to protect -- and exactly where a
/// user is least likely to notice a warning even if we did print one -- but
/// violating an existing, enforced output contract to fix that is the wrong
/// trade: it would surprise every other quiet consumer for a benefit that a
/// warning buried in unread quiet output mostly wouldn't deliver anyway.
///
/// This is distinct from [`print_kani_version_flag`], which handles the
/// explicit `--version`/`-V` query and is never gated on `--quiet` (it's a
/// one-shot query, not verification output) and uses a different first-line
/// format for compatibility reasons documented there.
pub(crate) fn print_kani_version(invocation_type: InvocationType) {
    let kani_version = kani_version_release(invocation_type);
    // TODO: Print development version information.
    // <https://github.com/model-checking/kani/issues/2617>
    println!("{kani_version}");
    print_cbmc_version_info();
}

/// Print version information in response to an explicit `--version`/`-V`
/// query (see `StandaloneArgs` and `CargoKaniArgs` in
/// `kani-driver/src/args/mod.rs`, both of which set `disable_version_flag =
/// true` and define their own `version` field so this function -- rather
/// than clap's built-in version flag -- actually runs): clap's conventional
/// `<bin-name> <version>` first line, followed by the CBMC pin diagnostic
/// (see [`print_cbmc_version_info`]).
///
/// The first line is deliberately NOT [`print_kani_version`]'s
/// `Kani Rust Verifier <version> (<invocation>)` banner format:
/// `tests/script-based-pre/kani-version-flag-version` and
/// `cargo-kani-version-flag-version` both parse `kani --version` /
/// `cargo kani --version` output with `awk '{print $2}'`, expecting the
/// second whitespace-separated field to be the bare version number -- the
/// same contract clap's built-in `--version` upheld before this change
/// replaced it with an explicit flag. `bin_name` matches the binary the user
/// actually invoked (`kani` or `cargo-kani`), same as clap would have
/// printed.
pub(crate) fn print_kani_version_flag(invocation_type: InvocationType) {
    let bin_name = match invocation_type {
        InvocationType::CargoKani(_) => "cargo-kani",
        InvocationType::Standalone => "kani",
    };
    println!("{bin_name} {KANI_VERSION}");
    print_cbmc_version_info();
}

/// Determine the CBMC version currently resolved via `PATH` by invoking
/// `cbmc --version` and parsing its output (e.g. `"6.8.0 (cbmc-6.8.0)"`).
///
/// Returns `None` if `cbmc` isn't on `PATH` or its output can't be parsed.
/// This is a best-effort diagnostic: if `cbmc` is genuinely missing, that
/// will already surface loudly once verification tries to invoke it, so we
/// don't fail the driver here.
fn cbmc_version_on_path() -> Option<String> {
    let output = Command::new("cbmc").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.split_whitespace().next().map(str::to_string)
}

/// The contents of the `kani-dependencies` file at the root of this Kani
/// checkout, embedded into the driver binary at *compile* time.
///
/// That file is already the single source of truth consumed (via shell
/// `source`) by `scripts/check_kissat_version.sh`, `scripts/kani-regression.sh`
/// and `scripts/setup/*/install_cbmc.sh` to install/check the pinned CBMC
/// version at setup/CI time. It was previously never consulted at driver
/// *runtime* at all, so a locally installed, unpinned CBMC could silently
/// diverge from the pin while every log claimed the pinned toolchain was in
/// use.
///
/// A first version of this check read the file from disk at runtime,
/// relative to the installation root resolved via `InstallType`. That works
/// for a dev-repo checkout, but `bundle_kani` (in
/// `tools/build-kani/src/main.rs`) never copies `kani-dependencies` into the
/// release bundle it assembles, so the same read unconditionally fails on a
/// release install -- silently disabling the entire check for exactly the
/// users who can't as easily audit their own toolchain by hand. Reading the
/// file via `include_str!` instead avoids the installation-layout question
/// altogether: the pinned version is baked into the binary regardless of how
/// it ends up installed, and rustc tracks `include_str!`'s argument as a
/// build dependency, so editing `kani-dependencies` still triggers a rebuild.
const KANI_DEPENDENCIES: &str = include_str!("../../kani-dependencies");

/// The CBMC version pinned in [`KANI_DEPENDENCIES`], or `None` if that file
/// doesn't define `CBMC_VERSION` in a form [`parse_dependency_var`] can read.
fn pinned_cbmc_version() -> Option<String> {
    parse_dependency_var(KANI_DEPENDENCIES, CBMC_VERSION_VAR)
}

/// Extract `KEY="VALUE"` (or `KEY='VALUE'`, or `KEY=VALUE`) from a
/// `kani-dependencies`-style file. This mirrors the trivial shell-variable
/// assignment format that file already uses. There is no existing non-shell
/// parser for it to reuse: every other consumer is a shell script that just
/// `source`s the file directly, so this is the one place `kani-dependencies`
/// is read outside of a shell.
///
/// Both quote characters are stripped (not just `"`), even though
/// `kani-dependencies` only ever uses double quotes today: shell considers
/// `KEY='VALUE'` an equally valid assignment, and handling both here is a
/// one-line difference against a real (if currently unexercised) input the
/// file's own format permits.
fn parse_dependency_var(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    contents
        .lines()
        .find_map(|line| line.trim().strip_prefix(prefix.as_str()))
        .map(|value| value.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|value| !value.is_empty())
}

/// Print the CBMC version resolved via `PATH` as part of Kani's startup
/// output (so it lands in any log kept as evidence), and warn -- but do not
/// hard-error -- if it doesn't match the version pinned in
/// `kani-dependencies`. This is intentionally warning-only: a working but
/// unpinned CBMC installation shouldn't wedge users who haven't run
/// `install_deps.sh` against the exact pin.
fn print_cbmc_version_info() {
    let Some(found) = cbmc_version_on_path() else {
        return;
    };
    println!("CBMC {found}");

    if let Some(pinned) = pinned_cbmc_version()
        && let Some(warning) = cbmc_version_mismatch_warning(&found, &pinned)
    {
        util::warning(&warning);
    }
}

/// Build the warning message for a CBMC version mismatch, or `None` if
/// `found` matches `pinned`. Split out from [`print_cbmc_version_info`] so
/// the message itself can be tested without needing a real `cbmc` on `PATH`
/// or a real `kani-dependencies` file.
fn cbmc_version_mismatch_warning(found: &str, pinned: &str) -> Option<String> {
    if found == pinned {
        None
    } else {
        Some(format!(
            "found CBMC {found} on PATH, but Kani pins CBMC {pinned} (see `kani-dependencies`). \
             Verification results may not reflect the pinned toolchain."
        ))
    }
}

/// Print Kani release version as `Kani Rust Verifier <version> (<invocation>)`
/// where:
///  - `<version>` is the `kani-verifier` version
///  - `<invocation>` is `cargo plugin` if Kani was invoked with `cargo kani` or
///    `standalone` if it was invoked with `kani`.
fn kani_version_release(invocation_type: InvocationType) -> String {
    let invocation_str = match invocation_type {
        InvocationType::CargoKani(_) => "cargo plugin",
        InvocationType::Standalone => "standalone",
    };
    format!("{KANI_RUST_VERIFIER} {KANI_VERSION} ({invocation_str})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_dependency_var() {
        let contents = "CBMC_MAJOR=\"6\"\nCBMC_MINOR=\"8\"\nCBMC_VERSION=\"6.8.0\"\n\nKISSAT_VERSION=\"4.0.1\"\n";
        assert_eq!(parse_dependency_var(contents, "CBMC_VERSION"), Some("6.8.0".to_string()));
        assert_eq!(parse_dependency_var(contents, "KISSAT_VERSION"), Some("4.0.1".to_string()));
    }

    #[test]
    fn parses_unquoted_dependency_var() {
        let contents = "CBMC_VERSION=6.8.0\n";
        assert_eq!(parse_dependency_var(contents, "CBMC_VERSION"), Some("6.8.0".to_string()));
    }

    #[test]
    fn parses_single_quoted_dependency_var() {
        // `kani-dependencies` only ever uses double quotes today, but
        // `KEY='VALUE'` is an equally valid shell assignment.
        let contents = "CBMC_VERSION='6.8.0'\n";
        assert_eq!(parse_dependency_var(contents, "CBMC_VERSION"), Some("6.8.0".to_string()));
    }

    #[test]
    fn parses_real_kani_dependencies_file() {
        // Guards against `include_str!`'s target ever silently becoming
        // unparseable (e.g. a reformat that `parse_dependency_var` can't
        // follow).
        assert_eq!(
            parse_dependency_var(KANI_DEPENDENCIES, CBMC_VERSION_VAR),
            pinned_cbmc_version()
        );
        assert!(pinned_cbmc_version().is_some(), "kani-dependencies must define CBMC_VERSION");
    }

    #[test]
    fn missing_dependency_var_is_none() {
        let contents = "KISSAT_VERSION=\"4.0.1\"\n";
        assert_eq!(parse_dependency_var(contents, "CBMC_VERSION"), None);
    }

    #[test]
    fn empty_dependency_var_is_none() {
        let contents = "CBMC_VERSION=\"\"\n";
        assert_eq!(parse_dependency_var(contents, "CBMC_VERSION"), None);
    }

    #[test]
    fn mismatched_versions_produce_a_warning_naming_both() {
        let warning = cbmc_version_mismatch_warning("6.7.1", "6.8.0").unwrap();
        assert!(warning.contains("6.7.1"), "warning should name the found version: {warning}");
        assert!(warning.contains("6.8.0"), "warning should name the pinned version: {warning}");
        assert!(warning.contains("kani-dependencies"));
    }

    #[test]
    fn matching_versions_produce_no_warning() {
        assert_eq!(cbmc_version_mismatch_warning("6.8.0", "6.8.0"), None);
    }
}
