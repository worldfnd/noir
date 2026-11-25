use std::convert::UnsafeFrom;
unconstrained fn halve_helper(x: Field) -> Field {
    x / 2
}

fn halve(x: Field) -> Field {
    // Safety: constrained below
    let y = unsafe { halve_helper(x) };
    assert(y * 2 == x);
    y
}

unconstrained fn third_helper(x: Field) -> Field {
    x / 3
}

fn main(a: u8, b: u8) -> pub u8 {
    let a = Field::from(a);
    let b = Field::from(b);
    let p = halve(a);
    let q = halve(b);
    // Safety: constrained below
    let r = unsafe { third_helper(q) };
    assert(r * 3 == q);
    u8::unsafe_from(p + r)
}
