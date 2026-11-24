use std::convert::TryFrom;

// Every `u32` is a field element, so the checked conversion is the cast and nothing more.
fn main(x: u32) {
    let element = Field::try_from(x).unwrap();
    assert(element == Field::from(x));
}
