// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Try with: kani ignore-main.rs --default-unwind 3-Z unstable-options --cbmc-args --object-bits 11
// With kissat as the solver (--external-sat-solver /path/to/kissat) this takes ~5mins

pub const MAX_PAGE_COMPACT_BUFFER: usize = 2048;

pub(crate) fn compact_page_frame_numbers(v: &mut [u32]) -> Vec<(u32, u32)> {
    if v.is_empty() {
        return vec![];
    }

    // Since the total number of pages that can be
    // received at once is `MAX_PAGE_COMPACT_BUFFER`,
    // this sort does not change the complexity of handling
    // an inflation.
    v.sort_unstable();

    // Since there are at most `MAX_PAGE_COMPACT_BUFFER` pages, setting the
    // capacity of `result` to this makes sense.
    let mut result = Vec::with_capacity(MAX_PAGE_COMPACT_BUFFER);

    // The most recent range of pages is [previous..previous + length).
    let mut previous = v[0];
    let mut length = 1;

    for page_frame_number in &v[1..] {
        // Check if the current page frame number is adjacent to the most recent page range.
        if *page_frame_number == previous + length {
            // If so, extend that range.
            length += 1;
        } else {
            // Otherwise, push (previous, length) to the result vector.
            result.push((previous, length));
            // And update the most recent range of pages.
            previous = *page_frame_number;
            length = 1;
        }
    }

    // Don't forget to push the last range to the result.
    result.push((previous, length));

    result
}

fn expand(ranges: Vec<(u32, u32)>) -> Vec<u32> {
    let mut v: Vec<u32> = Vec::new();
    for (start, len) in ranges {
        v.extend(start..=(start + len - 1));
    }
    return v;
}

#[kani::proof]
fn main() {
    let mut input = vec![0; 2];
    for i in 0..input.len() {
        input[i] = kani::any();
        if input[i] == u32::MAX {
            return;
        }
    }
    let output = compact_page_frame_numbers(&mut input);
    for (_start, len) in output.iter() {
        assert!(1 <= *len);
    }
    assert!(output.len() <= input.len());
    let expanded_output = expand(output);
    let i: usize = kani::any();
    if i < expanded_output.len() {
        assert!(expanded_output[i] == input[i]);
    }
}
