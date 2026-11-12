# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# `nix run .#update-versions`: records Kani releases in `nix/versions.json`.
# Uses the caller's `nix` for prefetching.
{
  coreutils,
  curl,
  git,
  gnused,
  jq,
  writeShellApplication,
  yq-go,
}:
writeShellApplication {
  name = "update-versions";
  runtimeInputs = [
    coreutils
    curl
    git
    gnused
    jq
    yq-go
  ];
  text = builtins.readFile ./update-versions.sh;
}
