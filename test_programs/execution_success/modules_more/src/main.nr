mod foo;
// An example of the module system
fn main(x: u8, y: u8) {
    assert(Field::from(x) != foo::bar::from_bar(Field::from(y)));
}
