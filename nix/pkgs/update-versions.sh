# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Records the Rust toolchain, CBMC and Kissat versions of Kani releases in `nix/versions.json`,
# along with the hashes of their release bundles, of those CBMC and Kissat versions' sources, and
# of the CaDiCaL version each CBMC version builds with.
#
# Each release is read from its tag (`kani-<version>`), not from the working tree: after a release,
# `main` keeps the released version in `Cargo.toml` while its toolchain moves on.

usage() {
  cat <<EOF
Usage: update-versions [<version>...]

  (no arguments)  record the version in Cargo.toml
  <version>...    record the given versions, e.g. 0.67.0
EOF
}

repo=$(git rev-parse --show-toplevel)
versions_file="$repo/nix/versions.json"
cd "$repo"

case "${1:-}" in
  -h | --help)
    usage
    exit 0
    ;;
  "") versions=("$(yq -p toml -oy '.package.version' Cargo.toml)") ;;
  *) versions=("$@") ;;
esac

if [[ -f $versions_file ]]; then
  data=$(cat "$versions_file")
else
  data='{"releases": {}, "sources": {}}'
fi

# Prints `{"<target>": "<hash>", ...}` for the release bundles of Kani `<version>`. Uses the
# digests GitHub publishes for release assets, so the bundles are only downloaded if one is missing.
bundle_hashes() {
  local version=$1 release name digest hash bundles='{}'
  local curl_args=(-fsSL -H "Accept: application/vnd.github+json")
  if [[ -n ${GITHUB_TOKEN:-} ]]; then
    curl_args+=(-H "Authorization: Bearer $GITHUB_TOKEN")
  fi
  release=$(curl "${curl_args[@]}" "https://api.github.com/repos/model-checking/kani/releases/tags/kani-$version")
  for target in "${targets[@]}"; do
    name="kani-$version-$target.tar.gz"
    if ! digest=$(jq -er --arg n "$name" '.assets[] | select(.name == $n) | .digest // ""' <<<"$release"); then
      echo "  no bundle for $target" >&2
      continue
    fi
    if [[ $digest == sha256:* ]]; then
      hash=$(nix hash convert --hash-algo sha256 --to sri "${digest#sha256:}")
    else
      echo "  downloading $name (GitHub has no digest for it)" >&2
      hash=$(nix store prefetch-file --json \
        "https://github.com/model-checking/kani/releases/download/kani-$version/$name" | jq -r .hash)
    fi
    bundles=$(jq --arg t "$target" --arg h "$hash" '.[$t] = $h' <<<"$bundles")
  done
  echo "$bundles"
}

# The targets Kani publishes release bundles for.
targets=(aarch64-apple-darwin aarch64-unknown-linux-gnu x86_64-apple-darwin x86_64-unknown-linux-gnu)

# Adds the source hash of `<name> <version>` from `<flake-ref>`, unless it is already recorded.
record_source() {
  local name=$1 version=$2 ref=$3 hash
  if jq -e --arg n "$name" --arg v "$version" '.sources[$n][$v]' <<<"$data" >/dev/null; then
    return
  fi
  echo "  prefetching $ref"
  hash=$(nix flake prefetch --json "$ref" | jq -r .hash)
  data=$(jq --arg n "$name" --arg v "$version" --arg h "$hash" '.sources[$n][$v] = $h' <<<"$data")
}

# Like `record_source`, for CBMC: also records the CaDiCaL version it downloads at build time.
record_cbmc() {
  local version=$1 ref="github:diffblue/cbmc/cbmc-$1" prefetched cadical
  if ! jq -e --arg v "$version" '.sources.cbmc[$v]' <<<"$data" >/dev/null; then
    echo "  prefetching $ref"
    prefetched=$(nix flake prefetch --json "$ref")
    cadical=$(sed -n 's|.*arminbiere/cadical/archive/rel-\(.*\)\.tar\.gz.*|\1|p' \
      "$(jq -r .storePath <<<"$prefetched")/src/solvers/CMakeLists.txt" | sort -u)
    if [[ -z $cadical || $cadical == *$'\n'* ]]; then
      echo "error: could not find the CaDiCaL version CBMC $version uses" >&2
      exit 1
    fi
    data=$(jq --arg v "$version" --arg h "$(jq -r .hash <<<"$prefetched")" --arg c "$cadical" \
      '.sources.cbmc[$v] = {cadical: $c, hash: $h}' <<<"$data")
  fi
  cadical=$(jq -r --arg v "$version" '.sources.cbmc[$v].cadical' <<<"$data")
  record_source cadical "$cadical" "github:arminbiere/cadical/rel-$cadical"
}

# The checkout (`kaniPackages`) may use CBMC or Kissat versions that no release uses yet.
cbmc=$(sed -n 's/^CBMC_VERSION="\(.*\)"$/\1/p' kani-dependencies)
kissat=$(sed -n 's/^KISSAT_VERSION="\(.*\)"$/\1/p' kani-dependencies)
echo "checkout: CBMC ${cbmc:-none}, Kissat ${kissat:-none}"
if [[ -n $cbmc ]]; then
  record_cbmc "$cbmc"
fi
if [[ -n $kissat ]]; then
  record_source kissat "$kissat" "github:arminbiere/kissat/rel-$kissat"
fi

for version in "${versions[@]}"; do
  tag="kani-$version"
  if ! git rev-parse --verify --quiet "$tag^{commit}" >/dev/null; then
    echo "error: no tag $tag; Kani releases are read from their tag (try \`git fetch --tags\`)" >&2
    exit 1
  fi
  if ! toolchain_toml=$(git show "$tag:rust-toolchain.toml" 2>/dev/null); then
    echo "skipping $version: $tag has no rust-toolchain.toml" >&2
    continue
  fi
  dependencies=$(git show "$tag:kani-dependencies" 2>/dev/null || true)

  toolchain=$(yq -p toml -o json -I0 '.toolchain' <<<"$toolchain_toml")
  cbmc=$(sed -n 's/^CBMC_VERSION="\(.*\)"$/\1/p' <<<"$dependencies")
  kissat=$(sed -n 's/^KISSAT_VERSION="\(.*\)"$/\1/p' <<<"$dependencies")

  echo "$version: $(jq -r .channel <<<"$toolchain"), CBMC ${cbmc:-none}, Kissat ${kissat:-none}"
  bundles=$(bundle_hashes "$version")

  data=$(jq --arg v "$version" --argjson b "$bundles" --argjson t "$toolchain" --arg c "$cbmc" \
    --arg k "$kissat" '
    .releases[$v] = {
      bundles: $b,
      cbmc: (if $c == "" then null else $c end),
      kissat: (if $k == "" then null else $k end),
      toolchain: $t
    }' <<<"$data")

  if [[ -n $cbmc ]]; then
    record_cbmc "$cbmc"
  fi
  if [[ -n $kissat ]]; then
    record_source kissat "$kissat" "github:arminbiere/kissat/rel-$kissat"
  fi
done

jq -S . <<<"$data" >"$versions_file.tmp"
mv "$versions_file.tmp" "$versions_file"
echo "Updated ${versions_file#"$repo"/}"
