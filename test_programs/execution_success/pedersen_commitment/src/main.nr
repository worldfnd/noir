// docs:start:pedersen-commitment
fn main(x: u8, y: u8, expected_x: [u8; 32], expected_y: [u8; 32]) {
    let commitment = std::hash::pedersen_commitment([x as Field, y as Field]);
    assert_eq(commitment.x.to_le_bytes(), expected_x);
    assert_eq(commitment.y.to_le_bytes(), expected_y);
}
// docs:end:pedersen-commitment
