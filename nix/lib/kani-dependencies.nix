# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Parses Kani's `kani-dependencies` file (`KEY="value"` lines) into an attribute set.
{ lib }:
file:
let
  parseLine =
    line:
    let
      m = builtins.match ''([A-Z_]+)="([^"]*)"'' line;
    in
    lib.optional (m != null) (lib.nameValuePair (lib.elemAt m 0) (lib.elemAt m 1));
in
builtins.listToAttrs (lib.concatMap parseLine (lib.splitString "\n" (builtins.readFile file)))
