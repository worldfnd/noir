// `u24` is a legal type inside a program, but not one `main` can take or return: an entry point
// carries only the integer types the circuit backend lowers.
fn main(x: u24) -> pub u24 {
    x + 1
}
