// It is expected that `y` must be equal to 0.
fn main(x: u8, y: pub u8) { let x = x as Field; let y = y as Field;
    let a: Field = x / y;
    std::println(a);
}
