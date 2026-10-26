// Tests a very simple program.
//
// The features being tested are:
// Binary addition, multiplication, division, constant modulo
// x = 3, y = 4, z = 5
fn main(x: u8, y: u8, z: u8) -> pub u8 {
    //cast
    assert(y as Field as u8 % 2 == 0);

    let a = x as Field + x as Field; // 3 + 3 = 6
    let b = a - y as Field; // 6 - 4 = 2
    let c = b * z as Field; // 2 * 5 = 10
    let d = c / a; // 10 / 6 (This uses field inversion, so we test it by multiplying by `a`)
    (d * a) as u8
}
