fn main() {
    let x: Field = std::hint::black_box(1);
    // This is a regression test for #2450.
    // The compiler should recognize that the `(x as u32)` instructions are duplicates and so have the same output.
    assert(x as u32 == x as u32);
}
