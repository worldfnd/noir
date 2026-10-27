fn main(x: u8, y: u8, salt: u8, out_x: [u8; 32], out_y: [u8; 32], out_hash: [u8; 32]) {
    let x = x as Field;
    let y = y as Field;
    let salt = salt as Field;

    let res = std::hash::pedersen_commitment([x, y]);
    assert(res.x == Field::from_le_bytes_checked(out_x));
    assert(res.y == Field::from_le_bytes_checked(out_y));

    let res_hash = std::hash::pedersen_hash_with_separator([x, y], 0);
    assert_eq(res_hash, Field::from_le_bytes_checked(out_hash));

    assert(res_hash != res.x);

    let raw_data = [x, y];
    let mut state = 0;
    for i in 0..2 {
        state = state * 8 + raw_data[i];
    }
    state += salt;
    let hash = std::hash::pedersen_commitment([state]);
    assert(std::hash::pedersen_commitment([43]).x == hash.x);
}
