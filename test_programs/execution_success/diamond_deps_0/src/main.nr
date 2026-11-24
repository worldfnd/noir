use dep1::call_dep1_then_dep2;
use dep2::call_dep2;
use dep2::RESOLVE_THIS;
use std::convert::UnsafeFrom;

fn main(x: u8, y: pub u8) -> pub u16 {
    let x = Field::from(x);
    let y = Field::from(y);
    u16::unsafe_from(call_dep1_then_dep2(x, y) + call_dep2(x, y) + RESOLVE_THIS)
}
