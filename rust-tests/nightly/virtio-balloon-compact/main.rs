// --unwind 3 --unwinding-assertions
//
// Example from Firecracker balloon device
// We test the functional correctness of the compact function
// Outstanding issues:
//   - Padstone-4988 (vector sort not supported)
//   - Padstone-4856 (vector tuple iteration)

#[allow(dead_code)]

pub const MAX_PAGES_IN_DESC: usize = 256;

pub(crate) fn compact_page_frame_numbers(v: &mut Vec<u32>) -> Vec<(u32, u32)> {
    if v.is_empty() {
        return vec![];
    }

    // Since the total number of pages that can be
    // received at once from a single descriptor is `MAX_PAGES_IN_DESC`,
    // this sort does not change the complexity of handling
    // an inflation.
    // v.sort(); //< Padstone-4988 (vector sort not supported)

    // Since there are at most `MAX_PAGES_IN_DESC` pages, setting the
    // capacity of `result` to this makes sense.
    let mut result = Vec::with_capacity(MAX_PAGES_IN_DESC);

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

// Padstone-4856 this version of expand required instead of fn above
fn rmc_expand(ranges: Vec<(u32, u32)>) -> Vec<u32> {
    let mut i = 0;
    let mut v: Vec<u32> = Vec::new();
    while i < ranges.len() {
        let (start, len) = ranges[i];
        for j in start..=(start + len - 1) {
            v.push(j)
        }
        i += 1;
    }
    return v;
}

fn __nondet<T>() -> T {
    unimplemented!()
}

fn main() {
    let mut input = vec![__nondet(); 2];
    for i in 0..input.len() {
        if input[i] == u32::MAX {
            return;
        }
    }
    let output = compact_page_frame_numbers(&mut input);
    for (_start, len) in output.iter() {
        assert!(1 <= *len);
    }
    assert!(output.len() <= input.len());
    let expanded_output = rmc_expand(output);
    let i: usize = __nondet();
    if i < expanded_output.len() {
        assert!(expanded_output[i] == input[i]);
    }
}
