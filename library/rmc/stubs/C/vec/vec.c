// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <stdint.h>
#include <assert.h>
#include <stdlib.h>

// This Vector stub implementation is supposed to work with c_vec.rs. Please
// refer to that file for a detailed explanation about the workings of this 
// abstraction. Public methods implemented in c_vec.rs act as wrappers around 
// methods implemented here.

// __CPROVER_max_malloc_size is dependent on the number of offset bits used to
// represent a pointer variable. By default, this is chosen to be 56, in which
// case the max_malloc_size is 2 ** (offset_bits - 1). We could go as far as to
// assign the default capacity to be the max_malloc_size but that would be overkill.
// Instead, we choose a high-enough value 2 ** (31 - 1). Another reason to do
// this is that it would be easier for the solver to reason about memory if multiple
// Vectors are initialized by the abstraction consumer.
#define DEFAULT_CAPACITY 1073741824
#define MAX_MALLOC_SIZE 18014398509481984

// A Vector is a dynamically growing array type with contiguous memory. We track
// allocated memory, the length of the Vector and the capacity of the
// allocation.

// As can be seen from the pointer to mem (unint32_t*), we track memory in terms
// of words. The current implementation works only if the containing type is
// u32. This was specifically chosen due to a use case seen in the Firecracker 
// codebase. This structure is used to communicate over the FFI boundary.
// Future work:
// Ideally, the pointer to memory would be uint8_t* - representing that we treat
// memory as an array of bytes. This would allow us to be generic over the type
// of the element contained in the Vector. In that case, we would have to treat
// every sizeof(T) bytes as an indivdual element and cast memory accordingly.
typedef struct {
	uint32_t* mem;
	size_t len;
	size_t capacity;
} vec;

// The grow operation resizes the vector and copies its original contents into a
// new allocation. This is one of the more expensive operations for the solver
// to reason about and one way to get around this problem is to use a large
// allocation size. We also implement sized_grow which takes a argument
// definining the minimum number of additional elements that need to be fit into
// the Vector memory. This aims to replicate behavior as seen in the Rust
// standard library where the size of the vector is decided based on the
// following equation:
// new_capacity = max(capacity * 2, capacity + additional).
// Please refer to method amortized_grow in raw_vec.rs in the Standard Library
// for additional information.
// The current implementation performance depends on CBMCs performance about
// reasoning about realloc. If CBMC does better, do would we in the case of
// this abstraction.
//
// One important callout to make here is that because we allocate a large enough
// buffer, we cant reason about buffer overflow bugs. This is because the
// allocated memory will (most-likely) always have enough space allocated after
// the required vec capacity.
//
// Future work:
// Ideally, we would like to get around the issue of resizing altogether since
// CBMC supports unbounded arrays. In that case, we would allocate memory of
// size infinity and work with that. For program verification, this would
// optimize a lot of operations since the solver does not really have to worry
// about the bounds of memory. The appropriate constant for capacity would be 
// __CPROVER_constant_infinity_uint but this is currently blocked due to
// incorrect translation of the constant: https://github.com/diffblue/cbmc/issues/6261.
//
// Another way to approach this problem would be to implement optimizations in
// the realloc operation of CBMC. Rather than allocating a new memory block and
// copying over elements, we can track only the end pointer of the memory and
// shift it to track the new length. Since this behavior is that of the
// allocator, the consumer of the API is blind to it.
void vec_grow(vec* v) {
	size_t new_cap = v->capacity * 2;
	if (new_cap > MAX_MALLOC_SIZE) {
		// Panic if the new size requirement is greater than max size that can
		// be allocated through malloc.
		assert(0);
	}

	uint32_t* new_mem = (uint32_t *) realloc(v->mem, new_cap * sizeof(*v->mem));
	v->mem = new_mem;
	v->capacity = new_cap;
}

void vec_sized_grow(vec* v, size_t additional) {
	size_t min_cap = v->capacity + additional;
	size_t grow_cap = v->capacity * 2;
	// This resembles the Rust Standard Library behavior.
	size_t new_cap = min_cap > grow_cap ? min_cap : grow_cap;
	if (new_cap > MAX_MALLOC_SIZE) {
		// Panic if the new size requirement is greater than max size that can
		// be allocated through malloc.
		assert(0);
	}

	uint32_t* new_mem = (uint32_t *) realloc(v->mem, new_cap * sizeof(*v->mem));
	v->mem = new_mem;
	v->capacity = new_cap;
}

vec* vec_new() {
	vec *v = (vec *) malloc(sizeof(vec));
	// Default size is DEFAULT_CAPACITY. We compute the maximum number of
	// elements to ensure that allocation size is aligned.
	size_t max_elements = DEFAULT_CAPACITY / sizeof(*v->mem);
	v->mem = (uint32_t *) malloc(max_elements * sizeof(*v->mem));
	v->len = 0;
	v->capacity = max_elements;
	// Return a pointer to the allocated vec structure, which is used in future
	// callbacks.
	return v;
}

vec* vec_with_capacity(size_t capacity) {
	vec *v = (vec *) malloc(sizeof(vec));
	if (capacity > MAX_MALLOC_SIZE) {
		// Panic if the new size requirement is greater than max size that can
		// be allocated through malloc.
		assert(0);
	}

	v->mem = (uint32_t *) malloc(capacity * sizeof(*v->mem));
	v->len = 0;
	v->capacity = capacity;
	return v;
}

void vec_push(vec* v, uint32_t elem) {
	// If we have already reached capacity, resize the Vector before
	// pushing in new elements.
	if (v->len == v->capacity) {
		// Ensure that we have capacity to hold atleast one more element
		vec_sized_grow(v, 1);
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
	// Reserve enough space before adding in new elements.
	vec_sized_grow(v1, v2->len);
	size_t i = 0;
	for (i = 0; i < v2->len; i++) {
		vec_push(v1, v2->mem[i]);
	}
	v1->len = v1->len + v2->len;
}

size_t vec_len(vec* v) {
	return v->len;
}

size_t vec_cap(vec* v) {
	return v->capacity;
}
