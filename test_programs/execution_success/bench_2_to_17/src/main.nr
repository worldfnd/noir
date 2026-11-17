global len: u32 = 2450 * 2;
fn main(x: u8) {
    let ped_input = [Field::from(x); len];
    let mut val = poseidon::poseidon2::Poseidon2::hash(ped_input, len);
    assert(val != 0);
}
