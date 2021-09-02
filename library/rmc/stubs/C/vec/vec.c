// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <stdint.h>
#include <assert.h>
#include <stdlib.h>

#define MAX_MALLOC_SIZE 4096

////////////////////////////////////////////////////////////////////////
// Vec stub implementation
//
// PoC code which implements a Vector stub.
// Functions in this module are exported as FFI to work with c_vec.rs which 
// acts like the Rust frontend.
////////////////////////////////////////////////////////////////////////

// A Vector has a pointer to a memory allocation, length and capacity.
typedef struct {
	uint32_t* mem;
	size_t len;
	size_t capacity;
} vec;

// Ideally, we would like to get around the issue of resizing altogether since
// CBMC supports unbounded arrays. This is however blocked.
// Tracking issue: https://github.com/diffblue/cbmc/issues/6261
//
// vec* ffi_new() {
//	  vec* v = (vec *) malloc(sizeof(vec));
// 	  v->len = 0;
// 	  v->capacity = __CPROVER_constant_infinity_uint;
// 	  v->mem = (uint32_t *) malloc(__CPROVER_constant_infinity_uint);
// 	  return v;
// }

// The grow operation resizes the vector and copies its original contents into 
// a new allocation. This implementation doubles the capacity, but in theory
// it could be implemented as a sized_grow() which takes in a new size.
void vec_grow(vec* v) {
	uint32_t* new_mem = (uint32_t *) realloc(v->mem, v->capacity * 2 * sizeof(uint32_t));
	v->mem = new_mem;
	v->capacity = v->capacity * 2;
}

vec* vec_new() {
	vec *v = (vec *) malloc(sizeof(vec));
	// Default size is MAX_MALLOC_SIZE
	v->mem = (uint32_t *) malloc(MAX_MALLOC_SIZE * sizeof(uint32_t));
	v->len = 0;
	v->capacity = MAX_MALLOC_SIZE;
	return v;
}

vec* vec_with_capacity(size_t capacity) {
	vec *v = (vec *) malloc(sizeof(vec));
	v->mem = (uint32_t *) malloc(capacity * sizeof(uint32_t));
	v->len = 0;
	v->capacity = capacity;
	return v;
}

void vec_push(vec* v, uint32_t elem) {
	// If we have already reached capacity, resize the vector before
	// pushing in new elements
	if (v->len == v->capacity) {
		vec_grow(v);
	}

	v->mem[v->len] = elem;
	v->len += 1;
}

uint32_t vec_pop(vec* v) {
	assert(v->len > 0);
	v->len -= 1;
	return v->mem[v->len];
}

void vec_append(vec* v1, vec* v2) {
	size_t i = 0;
	for (i = 0; i < v2->len; i++) {
		vec_push(v1, v2->mem[i]);
	}
	v1->len = v1->len + v2->len;
}

uint32_t vec_len(vec* v) {
	return v->len;
}

uint32_t vec_cap(vec* v) {
	return v->capacity;
}


