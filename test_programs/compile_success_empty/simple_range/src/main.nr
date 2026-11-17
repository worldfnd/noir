use std::convert::UnsafeFrom;
// Tests a very simple program.
//
// The features being tested is casting to an integer
fn main(x: u64) {
    let x = Field::from(x);
    let _z = u32::unsafe_from(x);
}
