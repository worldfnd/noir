// The data bus is not supported: inputs and returns are plain or `pub`.
fn main(x: call_data(0) u32, y: u32) -> return_data u32 {
    x + y
}
