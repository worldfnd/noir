use std::convert::UnsafeFrom;
fn main(a: u64) -> pub u128 {
    let c = -(Field::from(a));
    let c = u128::unsafe_from(c);
    assert_eq(c, 53438638232309528388129535304893203149);
    c
}
