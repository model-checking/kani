// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <stdint.h>
#include <assert.h>
#include <string.h>
#include <stdio.h>
#include <stdarg.h>

uint32_t rust_add1(uint32_t i);

uint32_t external_c_assertion(uint32_t x) {
    assert(rust_add1(x) == x + 1);
    return 0;
}
