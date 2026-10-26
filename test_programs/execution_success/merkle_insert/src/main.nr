fn main(
    old_root: [u8; 32],
    old_leaf: [u8; 32],
    old_hash_path: [[u8; 32]; 3],
    new_root: pub [u8; 32],
    leaf: [u8; 32],
    index: u8,
) {
    assert(old_root == compute_merkle_root(old_leaf, index, old_hash_path).to_le_bytes());

    let calculated_root = compute_merkle_root(leaf, index, old_hash_path);
    assert(Field::from_le_bytes_checked(new_root) == calculated_root);
}

fn compute_merkle_root<let N: u32>(leaf: [u8; 32], index: u8, hash_path: [[u8; 32]; N]) -> Field {
    let index_bits: [bool; N] = (index as Field).to_le_bits();
    let mut current = Field::from_le_bytes_checked(leaf);
    for i in 0..N {
        let path_bit = index_bits[i];
        let (hash_left, hash_right) = if path_bit {
            (Field::from_le_bytes_checked(hash_path[i]), current)
        } else {
            (current, Field::from_le_bytes_checked(hash_path[i]))
        };
        current = std::hash::pedersen_hash([hash_left, hash_right]);
    }
    current
}
