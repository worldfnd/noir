// This program tests:
// - the allocation of virtual arrays for array params to main
// - load instructions for such arrays
fn main(xs: [u8; 2]) -> pub u8 {
    xs[1]
}
