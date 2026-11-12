use crate1::MyTrait;

fn main(x: u8, y: pub u8) {
    let mut V = crate2::MyStruct { Q: x as Field };
    V.Add10();
    assert(V.Q == y as Field);
}
