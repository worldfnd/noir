use std::convert::UnsafeFrom;
fn main(x: u64) {
    let x = Field::from(x);
    // This is a regression test for #2450.
    // The compiler should recognize that the `(x as u32)` instructions are duplicates and so have the same output.
    assert(u32::unsafe_from(x) == u32::unsafe_from(x));
}
