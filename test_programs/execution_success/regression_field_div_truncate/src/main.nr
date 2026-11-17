use std::convert::UnsafeFrom;
// Regression: the truncate simplification incorrectly treated field division
// as integer division when reasoning about bit sizes, removing a necessary
// truncation after `array_get` optimization replaced the operand.
fn main(inp: u8) {
    let inp = Field::from(inp);
    let mut x: Field = -1;
    for _i in 0..1 {
        x = -1;
    }

    let mut y = [inp / x];

    let expected = (Field::from(u128::unsafe_from(y[0])));
    let mut actual: Field = 0;

    if (inp.lt(42)) {
        actual = (Field::from(u128::unsafe_from(y[0])));
    }

    assert_eq(actual, expected);
}
