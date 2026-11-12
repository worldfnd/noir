// docs:start:pedersen-hash
fn main(x: u8, y: u8, expected_hash: [u8; 32]) {
    let hash = std::hash::pedersen_hash([x as Field, y as Field]);
    assert_eq(hash.to_le_bytes(), expected_hash);
}
// docs:end:pedersen-hash
