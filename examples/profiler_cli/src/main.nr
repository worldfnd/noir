fn main(x: u32, y: u32, a: u32) {
    let (x, y, a) = (x as Field, y as Field, a as Field);
    let z = (x - y) * a;
    z.assert_max_bit_size::<4>();
    assert(a * a == 1);
}
