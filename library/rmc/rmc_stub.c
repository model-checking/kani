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

////////////////////////////////////////////////////////////////////////
// HashSet stub implementation
//
// PoC code which implements a HashSet stub for HashSet<u32>
// Functions in this module are exported as FFI to work with c_vec.rs which 
// acts like the Rust frontend.
////////////////////////////////////////////////////////////////////////

// It should have been possible to implement the stub using uninterpreted 
// functions. But we run into issues when the consumer of this API needs to 
// dynamically initialize HashSets. This would involve a verification problem
// with an unknown number of uninterpreted functions which is not valid.
//
// For the sake of documentation, below is a possible way to implement this
// idea.

// uint32_t __CPROVER_uninterpreted_f(uint32_t);
// 
// struct set {
// 	int counter;
// };
// 
// struct set g_s = { 0 };
// 
// uint32_t ffi_insert(uint32_t value) {
// 	__CPROVER_assume(__CPROVER_uninterpreted_f(g_s.counter) == value);
// 	g_s.counter += 1;
// 
// 	return 1;
// }
// 
// uint32_t ffi_contains(uint32_t value) {
// 	__CPROVER_bool condition = 0;
// 	for (int i = 0; i < g_s.counter; i++) {
// 		condition = condition || (__CPROVER_uninterpreted_f(i) == value);
// 	}
// 
// 	return condition;
// }

// The SENTINEL value is chosen to be outside the range of the hash function
// For this HashSet, the domain is uint32_t and the range is uint32_t. The 
// SENTINEL value is -1 which is in the range int32_t.
const SENTINEL = -1;

// This function needs to be reimplemented depending on the value being 
// stored in the hashset.
uint32_t hasher(uint32_t value) {
	return value;
}

// Here the domain is in the range of uint32_t but we store the hash value range
// as part of a larger domain.
typedef struct {
	int32_t* domain;
	uint32_t counter;
} hashset;

hashset* hashset_new() {
	hashset* set = (hashset *) malloc(sizeof(hashset));
	
	// Ideally, we should be able to work with arrays of arbitrary size but
	// 1. CBMC does not currently handle unbounded arrays correctly.
	//	  Issue: https://github.com/diffblue/cbmc/issues/6261
	//
	// 2. With UINT32_MAX we get error "array too large for flattening"
	//
	// Currently working with 4096 as the size. 
	// NOTE: This is unsound.
	//
	// set->domain = calloc(UINT32_MAX, sizeof(int32_t));
	set->domain = calloc(4096, sizeof(int32_t));

	set->domain[0] = SENTINEL;

	return set;
}

uint32_t hashset_insert(hashset* s, uint32_t value) {
	uint32_t hash = hasher(value);

	if ((hash == 0 && s->domain[hash] != SENTINEL) || s->domain[hash] != 0) {
		return 0;
	}

	s->domain[hash] = value;
	return 1;
}

uint32_t hashset_contains(hashset* s, uint32_t value) {
	uint32_t hash = hasher(value);

	if ((hash == 0 && s->domain[hash] != SENTINEL) || s->domain[hash] != 0) {
		return 1;
	}

	return 0;
}

uint32_t hashset_remove(hashset* s, uint32_t value) {
	uint32_t hash = hasher(value);

	if ((hash == 0 && s->domain[hash] == SENTINEL) || s->domain[hash] == 0) {
		return 0;
	}

	if (hash == 0) {
		s->domain[hash] = SENTINEL;
	} else {
		s->domain[hash] = 0;
	}

	return 1;
}
