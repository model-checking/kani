#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

current_toolchain_date=$(grep ^channel rust-toolchain.toml | sed 's/.*nightly-\(.*\)"/\1/')
echo "current_toolchain_date=$current_toolchain_date" >> $GITHUB_ENV

current_toolchain_epoch=$(date --date $current_toolchain_date +%s)
next_toolchain_date=$(date --date "@$(($current_toolchain_epoch + 86400))" +%Y-%m-%d)
echo "next_toolchain_date=$next_toolchain_date" >> $GITHUB_ENV

if gh issue list -S \
  "Toolchain upgrade to nightly-$next_toolchain_date failed" \
  --json number,title | grep title ; then
echo "next_step=none" >> $GITHUB_ENV

elif ! git ls-remote --exit-code origin toolchain-$next_toolchain_date ; then
echo "next_step=create_pr" >> $GITHUB_ENV

# Modify rust-toolchain file
sed -i "/^channel/ s/$current_toolchain_date/$next_toolchain_date/" rust-toolchain.toml

git diff
git clone --filter=tree:0 https://github.com/rust-lang/rust rust.git
cd rust.git
current_toolchain_hash=$(curl https://static.rust-lang.org/dist/$current_toolchain_date/channel-rust-nightly-git-commit-hash.txt)
echo "current_toolchain_hash=$current_toolchain_hash" >> $GITHUB_ENV

next_toolchain_hash=$(curl https://static.rust-lang.org/dist/$next_toolchain_date/channel-rust-nightly-git-commit-hash.txt)
echo "next_toolchain_hash=$next_toolchain_hash" >> $GITHUB_ENV

EOF=$(dd if=/dev/urandom bs=15 count=1 status=none | base64)
echo "git_log<<$EOF" >> $GITHUB_ENV

git log --oneline $current_toolchain_hash..$next_toolchain_hash | \
  sed 's#^#https://github.com/rust-lang/rust/commit/#' >> $GITHUB_ENV
echo "$EOF" >> $GITHUB_ENV

cd ..
rm -rf rust.git
if ! cargo build-dev ; then
  echo "next_step=create_issue" >> $GITHUB_ENV
else
  if ! ./scripts/kani-regression.sh ; then
    echo "next_step=create_issue" >> $GITHUB_ENV
  fi
fi
else
  echo "next_step=none" >> $GITHUB_ENV
fi
