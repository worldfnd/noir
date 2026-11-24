use std::convert::UnsafeFrom;
fn main() -> pub bool {
    let d = ((false as Field) / (false as Field));
    (u128::unsafe_from(d)) == 0
}
