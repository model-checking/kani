// When a SINGLE-character string literal is used where a char should be,
// suggest changing to single quotes.

// Testing both single-byte and multi-byte characters, as we should handle both.

// run-rustfix

fn main() {
    let _: char = "a"; //~ ERROR mismatched types
    let _: char = "人"; //~ ERROR mismatched types
}
