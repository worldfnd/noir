// A program runs on every field, so `main` takes no type that is field-specific.
#[field(bn254)]
struct Limbs {
    lo: u64,
    hi: u64,
}

fn main(x: Limbs) {
    assert(x.lo != x.hi);
}
