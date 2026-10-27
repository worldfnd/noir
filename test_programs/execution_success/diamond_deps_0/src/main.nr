use dep1::call_dep1_then_dep2;
use dep2::call_dep2;
use dep2::RESOLVE_THIS;

fn main(x: u8, y: pub u8) -> pub u16 {
    let x = x as Field;
    let y = y as Field;
    (call_dep1_then_dep2(x, y) + call_dep2(x, y) + RESOLVE_THIS) as u16
}
