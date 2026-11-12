use poseidon;

fn main(x1: [u8; 2], y1: pub [u8; 32], x2: [u8; 4], y2: pub [u8; 32]) {
    let hash1 = poseidon::poseidon::bn254::hash_2(x1.map(|x| x as Field));
    assert(hash1 == Field::from_le_bytes_checked(y1));

    let hash2 = poseidon::poseidon::bn254::hash_4(x2.map(|x| x as Field));
    assert(hash2 == Field::from_le_bytes_checked(y2));
}
