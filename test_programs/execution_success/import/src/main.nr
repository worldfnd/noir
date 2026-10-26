mod import;
use crate::import::hello;

fn main(x: u8, y: u8) {
    let _k = std::hash::pedersen_commitment([x as Field]);
    let _l = hello(x as Field);

    assert(x as Field != import::hello(y as Field));
}
