fn main(x: u8) -> pub u8 {
    let tripled = Field::from(x) * 3;
    tripled as u8
}
