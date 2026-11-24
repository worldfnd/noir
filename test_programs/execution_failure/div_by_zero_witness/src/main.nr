// It is expected that `y` must be equal to 0.
fn main(x: u8, y: pub u8) {
    let x = Field::from(x);
    let y = Field::from(y);
    let a: Field = x / y;
    std::println(a);
}
