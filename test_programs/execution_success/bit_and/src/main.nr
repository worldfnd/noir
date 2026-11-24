use std::convert::UnsafeFrom;
// You can only do bit operations with integers.
// (Kobi/Daira/Circom/#37) https://github.com/iden3/circom/issues/37
fn main(x: u8, y: u8, a: u8, b: u8) {
    let x = Field::from(x);
    let y = Field::from(y);
    let a = Field::from(a);
    let b = Field::from(b);

    let x_as_u8 = u8::unsafe_from(x);
    let y_as_u8 = u8::unsafe_from(y);

    assert((x_as_u8 & y_as_u8) == x_as_u8);
    //bitwise and with 1 bit:
    let flag = (x == 0) & (y == 16);
    assert(flag);
    //bitwise and with odd bits:
    let a_as_u8 = u8::unsafe_from(a);
    let b_as_u8 = u8::unsafe_from(b);
    assert((a_as_u8 & b_as_u8) == a_as_u8);
}
