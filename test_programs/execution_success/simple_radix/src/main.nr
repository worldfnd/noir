// Simple program to test to_radix
fn main(x: u8) {
    let bits: [bool; 3] = (Field::from(x)).to_le_bits();
    assert(!bits[0]);
    assert(bits[1]);
    assert(!bits[2]);
}
