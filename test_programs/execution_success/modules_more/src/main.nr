mod foo;
// An example of the module system
fn main(x: u8, y: u8) {
    assert(x as Field != foo::bar::from_bar(y as Field));
}
