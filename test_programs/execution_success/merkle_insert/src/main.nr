fn main(
    old_root: [u8; 32],
    old_leaf: [u8; 32],
    old_hash_path: [[u8; 32]; 3],
    new_root: pub [u8; 32],
    leaf: [u8; 32],
    index: u8,
) {
    let old_root = Field::from_le_bytes_checked(old_root);
    let old_leaf = Field::from_le_bytes_checked(old_leaf);
    let old_hash_path = old_hash_path.map(|node| Field::from_le_bytes_checked(node));
    let new_root = Field::from_le_bytes_checked(new_root);
    let leaf = Field::from_le_bytes_checked(leaf);
    let index = index as Field;

    assert(old_root == compute_merkle_root(old_leaf, index, old_hash_path));

    let calculated_root = compute_merkle_root(leaf, index, old_hash_path);
    assert(new_root == calculated_root);
}

fn compute_merkle_root<let N: u32>(leaf: Field, index: Field, hash_path: [Field; N]) -> Field {
    let index_bits: [bool; N] = index.to_le_bits();
    let mut current = leaf;
    for i in 0..N {
        let path_bit = index_bits[i];
        let (hash_left, hash_right) = if path_bit {
            (hash_path[i], current)
        } else {
            (current, hash_path[i])
        };
        current = std::hash::pedersen_hash([hash_left, hash_right]);
    }
    current
}
