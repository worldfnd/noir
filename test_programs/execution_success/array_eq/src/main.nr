// Checks both array equality (`a == b`) and inequality (`a != c`).
fn main(a: [u8; 32], b: [u8; 32], c: [u8; 32]) {
    assert(a == b);
    assert(a != c);
}
