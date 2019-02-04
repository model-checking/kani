#! /usr/bin/env bash

# This file is used to generate the docs for both variants the `proptest` crate
# as well as the Proptest Book and deposit them in the appropriate place to be
# hosted on GH pages.
#
# Note that the scripts it calls use absolete paths.

set -eux
(cd proptest && ./gen-docs.sh std)
(cd proptest && ./gen-docs.sh nostd)
(cd book && ./gen-docs.sh)
