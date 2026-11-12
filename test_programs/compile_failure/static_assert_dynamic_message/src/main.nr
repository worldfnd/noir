use std::static_assert;

fn main(x: u32) {
    let string = f"Assertion failed: {x}";
    static_assert(true, string);
}
