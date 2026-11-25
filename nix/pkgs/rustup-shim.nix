# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Kani's development build assumes rustup:
#  * `kani-compiler/build.rs` reads `RUSTUP_HOME` and `RUSTUP_TOOLCHAIN` and adds
#    `$RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN/lib` to the rpath;
#  * `kani-driver` runs `cargo +$RUSTUP_TOOLCHAIN ...` from a development checkout.
# This provides a fake `RUSTUP_HOME` whose only toolchain is the rust-overlay one, and a `cargo`
# that accepts (and drops) the matching `+toolchain` argument.
{
  bash,
  runCommand,
  rust-toolchain,
  toolchainChannel,
}:
runCommand "kani-rustup-shim-${toolchainChannel}"
  {
    passthru = { inherit toolchainChannel; };
  }
  ''
    mkdir -p $out/bin $out/rustup-home/toolchains
    ln -s ${rust-toolchain} $out/rustup-home/toolchains/${toolchainChannel}

    cat > $out/bin/cargo <<'EOF'
    #!${bash}/bin/bash
    case "''${1:-}" in
      +${toolchainChannel}) shift ;;
      +*)
        echo "kani-rustup-shim: only toolchain ${toolchainChannel} is available, got $1" >&2
        exit 1
        ;;
    esac
    exec ${rust-toolchain}/bin/cargo "$@"
    EOF
    chmod +x $out/bin/cargo
  ''
