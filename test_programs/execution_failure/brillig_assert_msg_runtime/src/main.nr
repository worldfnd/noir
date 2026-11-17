use std::convert::UnsafeFrom;
fn main(x: u8) {
    let x = Field::from(x);
    // Safety: testing context
    assert(1 == unsafe { conditional(x) });
}

unconstrained fn conditional(x: Field) -> Field {
    let z = u8::unsafe_from(x) + 20;
    assert_eq(z, 25, f"Expected 25 but got {z}");
    assert(x == 10, f"Expected x to equal 10, but got {x}");
    1
}
