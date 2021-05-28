#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Install CBMC 5.30.1 for Ubuntu 20.04
wget https://github.com/diffblue/cbmc/releases/download/cbmc-5.30.1/ubuntu-20.04-cbmc-5.30.1-Linux.deb \
  && sudo dpkg -i ubuntu-20.04-cbmc-5.30.1-Linux.deb \
  && cbmc --version
