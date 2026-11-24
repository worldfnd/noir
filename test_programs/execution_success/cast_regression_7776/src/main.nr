use std::convert::UnsafeFrom;
// Narrowing casts of a witness-derived value must be preserved rather than
// optimized away. See https://github.com/noir-lang/noir/issues/7776.
pub fn main(a: u8, b: u8, c: bool) -> pub (i8, u8, u64) {
    let a = Field::from(a);
    let b = Field::from(b);
    let d = a - b;
    let e = a - (c as Field);
    (i8::unsafe_from(d), u8::unsafe_from(d), u64::unsafe_from(e))
}
