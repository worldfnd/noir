fn main(x: u8, y: pub u8) {
    let x = Field::from(x);
    let y = Field::from(y);
    assert(x != y, f"Expected x != y, but got both equal {x}");
    assert(x != y);
    let z = x + y;
    assert(z != y, f"Expected z != y, but got both equal {z}");
    assert_eq(x, y, f"Expected x == y, but x is {x} and y is {y}");
}
