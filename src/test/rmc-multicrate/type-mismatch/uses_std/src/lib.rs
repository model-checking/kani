// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub fn bar(r: std::ops::Range<usize>) -> std::ops::Range<usize> {
    std::ops::Range { start: r.start + 5, end: r.end + 5 }
}
