// `u24` passes the frontend at `main`, but the circuit backend lowers only u8, u16, u32, u64,
// u128, i8, i16, i32 and i64, so the driver rejects the program before generating ACIR.
fn main(x: u24) -> pub u24 {
    x + 1
}
