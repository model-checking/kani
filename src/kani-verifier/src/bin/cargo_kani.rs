// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;

fn main() -> Result<()> {
    kani_verifier::proxy("cargo-kani")
}
