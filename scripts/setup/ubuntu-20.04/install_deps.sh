#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Install tools in Ubuntu 20.04 via `apt-get`
sudo apt-get --yes update \
  && sudo DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes \
  bison \
  cmake \
  ctags \
  curl \
  flex \
  g++ \
  gcc \
  git \
  gpg-agent \
  libssl-dev \
  lsb-release \
  make \
  ninja-build \
  patch \
  pkg-config \
  python-is-python3 \
  software-properties-common \
  wget \
  zlib1g \
  zlib1g-dev
