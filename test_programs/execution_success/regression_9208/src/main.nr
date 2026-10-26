global G_A: [Field] = [].as_vector();
fn main(a: u32) -> pub [u8; 32] {
    if a == 10 {
        G_A[1_u32 % G_A.len()].to_le_bytes()
    } else if a == 20 {
        (a as Field).to_le_bytes()
    } else {
        (0 - 1).to_le_bytes()
    }
}
