fn main(x: u8, y: pub u8) {
    basic_check(x as Field, y as Field);
}

#[no_predicates]
fn basic_check(x: Field, y: Field) {
    assert(x != y);
}
