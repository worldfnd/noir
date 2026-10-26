unconstrained fn main(
    x: u8,
    y: u8,
    salt: u8,
    out_x: [u8; 32],
    out_y: [u8; 32],
    out_hash: [u8; 32],
) {
    let res = std::hash::pedersen_commitment_with_separator([x as Field, y as Field], 0);
    assert(res.x == Field::from_le_bytes_checked(out_x));
    assert(res.y == Field::from_le_bytes_checked(out_y));

    let res_hash = std::hash::pedersen_hash_with_separator([x as Field, y as Field], 0);
    assert_eq(res_hash, Field::from_le_bytes_checked(out_hash));

    assert(res_hash != res.x);

    let raw_data = [x as Field, y as Field];
    let mut state = 0;
    for i in 0..(2 as u32) {
        state = state * 8 + raw_data[i];
    }
    state += salt as Field;
    let hash = std::hash::pedersen_commitment_with_separator([state], 0);
    assert(std::hash::pedersen_commitment_with_separator([43], 0).x == hash.x);
}
