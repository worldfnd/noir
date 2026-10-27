// You can only do bit operations with integers.
// (Kobi/Daira/Circom/#37) https://github.com/iden3/circom/issues/37
fn main(x: u8, y: u8, a: u8, b: u8) {
    assert((x & y) == x);
    //bitwise and with 1 bit:
    let flag = (x == 0) & (y == 16);
    assert(flag);
    //bitwise and with odd bits:
    assert((a & b) == a);
}
