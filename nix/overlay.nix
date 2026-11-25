# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Adds to nixpkgs:
#  * `kaniPackages`: the tools for this checkout, read from `rust-toolchain.toml` and
#    `kani-dependencies`;
#  * `kaniReleases.<version>` (and `kaniReleases.latest`): the same tools for each Kani release
#    recorded in `versions.json` by `nix run .#update-versions`, plus that release's `kani`.
# Requires rust-overlay to be applied first.
final: prev:
let
  inherit (final) lib;

  versions = lib.importJSON ./versions.json;

  # A version that was not recorded fails loudly instead of building the wrong version.
  source =
    name: version:
    versions.sources.${name}.${version}
      or (throw "kani: no source hash for ${name} ${version}; run `nix run .#update-versions`");

  # The tools Kani `version` needs. `toolchain` is the `[toolchain]` table of `rust-toolchain.toml`.
  # `bundles` maps Rust targets to the hashes of that release's bundles; only releases have them.
  mkKaniPackages =
    {
      bundles ? null,
      cbmcVersion,
      kissatVersion,
      toolchain,
      version,
    }:
    lib.makeScope final.newScope (self: {
      inherit version;

      toolchainChannel = toolchain.channel;

      rust-toolchain = final.rust-bin.fromRustupToolchain toolchain;

      rustup-shim = self.callPackage ./pkgs/rustup-shim.nix { };

      cbmc =
        let
          cbmcSource = source "cbmc" cbmcVersion;
        in
        prev.cbmc.overrideAttrs (old: {
          version = cbmcVersion;
          src = final.fetchFromGitHub {
            owner = "diffblue";
            repo = "cbmc";
            tag = "cbmc-${cbmcVersion}";
            inherit (cbmcSource) hash;
          };
          # The CaDiCaL version this CBMC version downloads at build time.
          srccadical = final.fetchFromGitHub {
            owner = "arminbiere";
            repo = "cadical";
            tag = "rel-${cbmcSource.cadical}";
            hash = source "cadical" cbmcSource.cadical;
          };
          # nixpkgs' second patch points CBMC's CaDiCaL and glucose downloads at local sources, but
          # only matches the CaDiCaL version of nixpkgs' CBMC. Do the same for any version.
          patches = lib.filter (
            p: !lib.hasSuffix "0002-Do-not-download-sources-in-cmake.patch" (toString p)
          ) old.patches;
          prePatch = (old.prePatch or "") + ''
            sed -i \
              -e 's|URL https://github.com/BrunoDutertre/glucose-syrup/archive/.*\.tar\.gz|SOURCE_DIR @srcglucose@|' \
              -e 's|URL https://github.com/arminbiere/cadical/archive/rel-.*\.tar\.gz|SOURCE_DIR @srccadical@|' \
              -e '/SOURCE_DIR @src\(cadical\|glucose\)@/,/)/{/URL_\(MD5\|HASH\)/d}' \
              src/solvers/CMakeLists.txt
          '';
        });

      # Older releases do not use Kissat.
      kissat =
        if kissatVersion == null then
          null
        else
          prev.kissat.overrideAttrs (old: {
            version = kissatVersion;
            src = final.fetchFromGitHub {
              owner = "arminbiere";
              repo = "kissat";
              tag = "rel-${kissatVersion}";
              hash = source "kissat" kissatVersion;
            };
          });

      dev-shell = self.callPackage ./pkgs/dev-shell.nix { };
    }
    // lib.optionalAttrs (bundles != null) {
      kani = self.callPackage ./pkgs/kani-bin.nix { inherit bundles; };
    });

  dependencies = import ./lib/kani-dependencies.nix { inherit lib; } ../kani-dependencies;

  releases = lib.mapAttrs (
    version: release:
    mkKaniPackages {
      inherit version;
      inherit (release) bundles toolchain;
      cbmcVersion = release.cbmc;
      kissatVersion = release.kissat;
    }
  ) versions.releases;

  latest = lib.last (lib.sort lib.versionOlder (lib.attrNames releases));
in
{
  kaniPackages = mkKaniPackages {
    cbmcVersion = dependencies.CBMC_VERSION;
    kissatVersion = dependencies.KISSAT_VERSION or null;
    toolchain = (lib.importTOML ../rust-toolchain.toml).toolchain;
    version = (lib.importTOML ../Cargo.toml).package.version;
  };

  kaniReleases = releases // lib.optionalAttrs (releases != { }) { latest = releases.${latest}; };

  kani-update-versions = final.callPackage ./pkgs/update-versions.nix { };
}
