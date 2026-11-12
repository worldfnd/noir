fn main(input: [u8; 2]) -> pub [u8; 32] {
    poseidon::poseidon::bn254::hash_2(input.map(|x| x as Field)).to_le_bytes()
}
