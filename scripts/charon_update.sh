#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# This script is part of our CI nightly job to bump the Charon pin (the `charon` submodule, used by
# the LLBC backend) to Charon's latest `nightly-*` tag. It updates the submodule and `Cargo.lock`,
# runs the LLBC regression, and tells the workflow what to do next via `$GITHUB_ENV`:
#
# - `next_step=create_pr`: the bump passed `kani-llbc-regression.sh`; open a PR;
# - `next_step=create_issue`: it failed and no failure issue is open; file one;
# - `next_step=comment_issue`: it failed and the failure issue (`failure_issue`) is open; add a
#   comment for the new tag instead of filing another issue;
# - `next_step=none`: nothing to do (already at the latest tag, the PR branch exists, or the open
#   failure issue already mentions this tag).
#
# Charon tags nearly every day, so failures are tracked in a single rolling issue rather than one
# per tag.

set -eu

failure_title="Automatic Charon upgrade failed"
charon_url=$(git config -f .gitmodules submodule.charon.url)

current_commit=$(git ls-tree HEAD charon | awk '{print $3}')
# Only accept tags of the expected shape: the name ends up in branch names, titles and commands.
next_tag=$(git ls-remote --tags --refs "$charon_url" 'nightly-*' | sed 's#.*refs/tags/##' | \
  grep -E '^nightly-[0-9]{4}\.[0-9]{2}\.[0-9]{2}$' | sort | tail -1)
if [ -z "$next_tag" ]; then
  echo "No nightly-YYYY.MM.DD tag found at $charon_url"
  exit 1
fi
# Fetch the tag with enough history to relate it to the current pin.
git -C charon fetch --quiet --filter=tree:0 "$charon_url" "refs/tags/$next_tag:refs/tags/$next_tag"
next_commit=$(git -C charon rev-parse "$next_tag^{commit}")
current_tag=$(git -C charon describe --tags --exact-match "$current_commit" 2>/dev/null || true)
current_name=${current_tag:-${current_commit:0:10}}
{
  echo "current_charon=$current_name"
  echo "current_charon_commit=$current_commit"
  echo "next_charon=$next_tag"
  echo "next_charon_commit=$next_commit"
} >> "$GITHUB_ENV"

echo "------ Start upgrade ------"
echo "- current: $current_name ($current_commit)"
echo "- next: $next_tag ($next_commit)"
echo "---------------------------"

failure_issue=$(gh issue list --state open --search "\"$failure_title\" in:title" \
  --json number,title --jq ".[] | select(.title == \"$failure_title\") | .number" | head -1)
echo "failure_issue=$failure_issue" >> "$GITHUB_ENV"

if [ "$next_commit" = "$current_commit" ]; then
  echo "Skip update: already at $next_tag"
  echo "next_step=none" >> "$GITHUB_ENV"
  exit 0
fi
if git ls-remote --exit-code origin "charon-$next_tag" > /dev/null; then
  echo "Skip update: found existing branch charon-$next_tag"
  echo "next_step=none" >> "$GITHUB_ENV"
  exit 0
fi
if [ -n "$failure_issue" ] && \
    gh issue view "$failure_issue" --json body,comments --jq '.body, .comments[].body' | \
      grep -qF "$next_tag"; then
  echo "Skip update: issue #$failure_issue already reports $next_tag"
  echo "next_step=none" >> "$GITHUB_ENV"
  exit 0
fi

# The (first-parent, i.e. merged-PR) log of the update, for the PR or issue, capped so that the
# text stays well below GitHub's size limit for issue and PR bodies.
max_log=200
log=$(git -C charon log --oneline --first-parent "$current_commit..$next_commit")
count=$(echo "$log" | wc -l)
EOF=$(dd if=/dev/urandom bs=15 count=1 status=none | base64)
{
  echo "charon_log<<$EOF"
  echo "Full comparison: https://github.com/AeneasVerif/charon/compare/$current_commit...$next_commit"
  echo
  echo "$log" | head -n "$max_log" | \
    sed 's#^\([0-9a-f]*\) #https://github.com/AeneasVerif/charon/commit/\1 #'
  if [ "$count" -gt "$max_log" ]; then
    echo "... and $((count - max_log)) more"
  fi
  echo "$EOF"
} >> "$GITHUB_ENV"

git -C charon checkout --quiet "$next_commit"
cargo update -p charon
git diff --stat

if ./scripts/kani-llbc-regression.sh; then
  echo "next_step=create_pr" >> "$GITHUB_ENV"
elif [ -n "$failure_issue" ]; then
  echo "next_step=comment_issue" >> "$GITHUB_ENV"
else
  echo "next_step=create_issue" >> "$GITHUB_ENV"
fi
