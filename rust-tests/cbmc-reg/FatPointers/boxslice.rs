/// Casts boxed array to boxed slice (example taken from rust documentation)
use std::str;

fn main() {
    // This vector of bytes is used to initialize a Box<[u8; 4]>
    let sparkle_heart = vec![240, 159, 146, 150];

    // This transformer produces a Box<[u8>]
    let _sparkle_heart = str::from_utf8(&sparkle_heart);
}
