use std::convert::UnsafeFrom;
fn main() {
    comptime {
        let x: u64 = 1;
        let y = Field::from(x);
        let _ = u128::unsafe_from(y);
    }
}
