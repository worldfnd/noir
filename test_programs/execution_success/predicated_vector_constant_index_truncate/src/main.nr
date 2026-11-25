// The in-bounds counterpart of `predicated_vector_constant_index`: with three items the
// disabled-branch read still lands on the wrong slots, but stays inside the block, so instead
// of an out-of-bounds failure a leaf declared `u8` ends up holding the *next* item's `Field`.
//
// `S` flattens to `Field, u8, u8`. The read of `s[1].b` should land on slots 4 and 5; biasing
// the already-resolved constant offset by `b`'s fallback offset sends it to slots 5 and 6, and
// slot 6 is `x[2].a`. `& 15` lowers to a 4-bit truncate, which cannot be satisfied by an
// arbitrary field element — so a value the program never reads decides whether the circuit
// solves, on a branch that `c = false` never takes.
struct S {
    a: Field,
    b: [u8; 2],
}

// `S` as `main` takes it: an entry point has no `Field` parameters.
struct InputS {
    a: u32,
    b: [u8; 2],
}

fn main(x: [InputS; 3], i: u32, c: bool, v: u8) -> pub u8 {
    let x = x.map(|s: InputS| S { a: Field::from(s.a), b: s.b });
    let v = Field::from(v);
    let mut s: [S] = x.as_vector();
    s[i] = S { a: v, b: [0, 0] };
    let mut r: u8 = 0;
    if c {
        r = s[1].b[1] & 15;
    }
    r
}
