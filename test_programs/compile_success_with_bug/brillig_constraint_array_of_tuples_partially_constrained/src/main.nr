use std::convert::UnsafeFrom;
unconstrained fn pairs_helper(x: Field) -> [(Field, Field); 2] {
    [(x, x), (x, x)]
}

fn main(x: u8) -> pub u8 {
    let x = Field::from(x);
    // Safety: only the first pair is constrained
    let pairs = unsafe { pairs_helper(x) };
    assert(pairs[0].0 == x);
    assert(pairs[0].1 == x);
    u8::unsafe_from(pairs[1].0 + pairs[1].1)
}
