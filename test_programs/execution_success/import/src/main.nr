mod import;
use crate::import::hello;

fn main(x: u8, y: u8) {
    let x = Field::from(x);
    let y = Field::from(y);
    let _k = std::hash::pedersen_commitment([x]);
    let _l = hello(x);

    assert(x != import::hello(y));
}
