// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <stdint.h>
#include <assert.h>
#include <stdlib.h>

// This HashSet stub implementation is supposed to work with c_hashset.rs.
// Please refer to that file for an introduction to the idea of a HashSet and
// some other implemenntation details. Public methods defined in c_hashset.rs
// act as wrappers around methods implemented here.

// As noted before, this HashSet implementation is specifically for inputs which
// are u16. The details below can be extended to larger sets if necessary. The 
// domain of the output is i16.
//
// The hash function that we choose is the identity function.
// For all input x, hasher(x) = x. For our case, this satisfies all the
// requirements of an ideal hash function - it is 1:1, and the range is a subset
// of the chosen output domain - which allows us access to SENTINEL values.
//
// An important thing to note here is that the hash function can be
// appropriately modified depending on the type of the input value which is
// stored in the hashset. As an example, if the HashSet needs to store a tuple
// of integers, say <u32, u32>, the hash function can be modified to be:
//
// hash((x, y)) = prime * x + y;
//
// Although this value can be greater than the chosen output domain, the
// function is still sound if the value wraps around because it guarantees a
// unique output for a given pair of x and y.
//
// Another way to think about this problem could be through the lens of
// uninterpreted functions where : if x == y => f(x) == f(y). Exploring this can
// be future work. The idea would be to implement a HashSet similar to that seen
// in functional programming languages.
//
// For the purpose of a HashSet, we dont necessarily need a SENTINEL outside the
// range of the hashing function because of the way we design the HashSet
// operations.
const int16_t SENTINEL = 1;

uint16_t hasher(uint16_t value) {
	return value;
}

// We initialize all values of the domain to be 0 by initializing it with
// calloc. This lets us get around the problem of looping through all elements
// to initialize them individually with a special value.
//
// The domain array is to be interpreted such that 
// if domain[index] != 0, value such that hash(value) = index is present.
//
// However, this logic does not work for the value 0. For this, we choose the
// SENTINEL value to initialize that element. 
typedef struct {
	int16_t* domain;
} hashset;

// Ideally, this approach is much more suitable if we can work with arrays of
// arbitrary size, specifically infinity. This would allow us to define hash
// functions for any type because the output domain can be considered to be
// infinite. However, CBMC currently does not handle unbounded arrays correctly.
// Please see: https://github.com/diffblue/cbmc/issues/6261. Even in that case,
// we might run into theoretical limitations of how solvers handle uninterpreted
// symbols such as unbounded arrays. For the case of this API, the consumer can
// request for an arbitrary number of HashSets which can be dynamically chosen.
// As a result, the solver cannot know apriori how many unbounded arrays it
// needs to initialize which might lead to errors.
//
// Firecracker uses HashSet<u32> (src/devices/src/virtio/vsock/unix/muxer.rs).
// But for working with u32s, we run into the problem that the entire domain
// cannot be allocated through malloc. We run into the error "array too large
// for flattening". For that reason, we choose to work with u16 to demonstrate
// the feasability of this approach. However, it should be extensible to other
// integer types.
hashset* hashset_new() {
	hashset* set = (hashset *) malloc(sizeof(hashset));
	// Initializes value all indexes to be 0, indicating that those elements are
	// not present in the HashSet.
	set->domain = calloc(UINT16_MAX, sizeof(uint16_t));
	// For 0, choose another value to achieve the same.
	set->domain[0] = SENTINEL;
	return set;
}

// For insert, we need to first check if the value exists in the HashSet. If it
// does, we immediately return a 0 (false) value back.
//
// If it doesnt, then we mark that element of the domain array with the value to 
// indicate that this element has been inserted. For element 0, we mark it with
// the SENTINEL.
//
// To check if a value exists, we simply check if domain[hash] != 0 and
// in the case of 0 if domain[0] != SENTINEL.
uint32_t hashset_insert(hashset* s, uint16_t value) {
	uint16_t hash = hasher(value);

	if ((hash == 0 && s->domain[hash] != SENTINEL) || 
		(hash !=0 && s->domain[hash] != 0)) {
		return 0;
	}

	s->domain[hash] = value;
	return 1;
}

// We perform a similar check here as described in hashset_insert(). We do not
// duplicate code so as to not compute the hash twice. This can be improved.
uint32_t hashset_contains(hashset* s, uint16_t value) {
	uint16_t hash = hasher(value);

	if ((hash == 0 && s->domain[hash] != SENTINEL) || 
		(hash != 0 && s->domain[hash] != 0)) {
		return 1;
	}

	return 0;
}

// We check if the element exists in the array. If it does not, we return a 0
// (false) value back. If it does, we mark it with 0 and in the case of 0, we
// mark it with the SENTINEL and return 1.
uint32_t hashset_remove(hashset* s, uint16_t value) {
	uint16_t hash = hasher(value);

	if ((hash == 0 && s->domain[hash] == SENTINEL) || 
		(hash !=0 && s->domain[hash] == 0)) {
		return 0;
	}

	if (hash == 0) {
		s->domain[hash] = SENTINEL;
	} else {
		s->domain[hash] = 0;
	}

	return 1;
}
