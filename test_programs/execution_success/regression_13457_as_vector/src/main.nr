use std::convert::UnsafeFrom;
// The division by zero and the `push_back` onto the vector made by `as_vector` sit inside a
// branch that `flag = false` never takes, so the program must solve and return 0.
fn main(flag: bool, arr: [u8; 2]) -> pub u8 {
    let arr = arr.map(|v| Field::from(v));
    let mut out = 0;
    if flag {
        let zero: u32 = 0;
        let q = 7 / zero;
        let v = arr.as_vector().push_back(Field::from(q));
        out = v[0];
    }
    u8::unsafe_from(out)
}
