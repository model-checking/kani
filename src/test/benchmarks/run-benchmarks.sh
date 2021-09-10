#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# A simple script to run all benchmarks against the RMCVec abstraction
for file in $(ls Vector) 
do
	time rmc --use-abs --abs-type rmc Vector/$file
done
