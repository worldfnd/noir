global G_A: [Field] = [].as_vector();
fn main(a: u32) -> pub [u8; 32] {
    let result = if a == 10 {
        G_A[1_u32 % G_A.len()]
    } else if a == 20 {
        (a as Field)
    } else {
        -1
    };
    result.to_le_bytes()
}
