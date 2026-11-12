# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Kani from its release bundle: the same files `cargo kani setup` installs into `~/.kani`, with
# the `toolchain` link pointing at the rust-overlay toolchain instead of a rustup one.
{
  autoPatchelfHook,
  bundles,
  fetchurl,
  lib,
  makeWrapper,
  rust-toolchain,
  stdenv,
  version,
}:
let
  target = stdenv.hostPlatform.rust.rustcTarget;
in
stdenv.mkDerivation {
  pname = "kani";
  inherit version;

  src = fetchurl {
    url = "https://github.com/model-checking/kani/releases/download/kani-${version}/kani-${version}-${target}.tar.gz";
    hash =
      bundles.${target}
        or (throw "kani: no release bundle of Kani ${version} for ${target} recorded; run `nix run .#update-versions -- ${version}`");
  };

  nativeBuildInputs = [ makeWrapper ] ++ lib.optional stdenv.hostPlatform.isLinux autoPatchelfHook;
  # On Linux, the bundled binaries need the C++ standard library (CBMC) and `librustc_driver`
  # (kani-compiler). On macOS, they find the latter through `@loader_path/../toolchain/lib`.
  buildInputs = lib.optionals stdenv.hostPlatform.isLinux [
    rust-toolchain
    stdenv.cc.cc.lib
  ];

  dontConfigure = true;
  dontBuild = true;
  # The bundle holds precompiled Rust libraries that must stay as they are.
  dontStrip = true;

  # `kani-driver` treats the parent of its own `bin/` as the installation directory.
  installPhase = ''
    runHook preInstall

    mkdir -p $out/lib $out/bin
    cp -r . $out/lib/kani
    ln -s ${rust-toolchain} $out/lib/kani/toolchain

    for bin in kani cargo-kani; do
      makeWrapper $out/lib/kani/bin/kani-driver $out/bin/$bin \
        --argv0 $bin \
        --prefix PATH : $out/lib/kani/bin
    done

    runHook postInstall
  '';

  # The bundle records the compiler it was built with; anything else cannot load `kani-compiler`.
  doInstallCheck = true;
  installCheckPhase = ''
    runHook preInstallCheck

    expected=$(cat $out/lib/kani/rustc-version)
    actual=$(${rust-toolchain}/bin/rustc --version)
    if [[ $expected != "$actual" ]]; then
      echo "Kani ${version} was built with '$expected', but the toolchain is '$actual'" >&2
      exit 1
    fi
    $out/bin/kani --version

    runHook postInstallCheck
  '';

  meta = {
    description = "Bit-precise model checker for Rust";
    homepage = "https://github.com/model-checking/kani";
    license = with lib.licenses; [
      asl20
      mit
    ];
    mainProgram = "kani";
    platforms = [
      "aarch64-darwin"
      "aarch64-linux"
      "x86_64-darwin"
      "x86_64-linux"
    ];
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
  };
}
