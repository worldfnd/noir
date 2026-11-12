fn main(input: [u8; 2]) -> pub [u8; 32] {
    poseidon::poseidon2::Poseidon2::hash(input.map(|x| x as Field), input.len()).to_le_bytes()
}
