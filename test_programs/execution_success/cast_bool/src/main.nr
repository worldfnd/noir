fn main(x: u8, y: u8) {
    let z = x == y;
    let t = z as u8;
    assert(t == 1);
}
