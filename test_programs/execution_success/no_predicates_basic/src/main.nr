fn main(x: u8, y: pub u8) {
    basic_check(Field::from(x), Field::from(y));
}

#[no_predicates]
fn basic_check(x: Field, y: Field) {
    assert(x != y);
}
