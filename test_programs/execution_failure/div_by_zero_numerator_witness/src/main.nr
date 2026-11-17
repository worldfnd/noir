fn main(x: u8) {
    let x = Field::from(x);
    let a: Field = x / 0;
    std::println(a);
}
