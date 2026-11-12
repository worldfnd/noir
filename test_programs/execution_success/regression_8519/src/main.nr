fn main(a: u64) -> pub u128 {
    let c = -(a as Field);
    let c = c as u128;
    assert_eq(c, 53438638232309528388129535304893203149);
    c
}
