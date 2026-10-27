// Tests a very simple program.
//
// The features being tested is casting to an integer
fn main() {
    let x: Field = std::hint::black_box(1);
    let _z = x as u32;
}
