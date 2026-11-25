use std::convert::UnsafeFrom;
// `*p` and `*(arr[0].0)` name the same cell, so the two outputs must agree.
fn write(q: &mut &mut Field, v: Field) {
    let p = *q;
    *p = v;
}

fn main(x: u8, y: u8) -> pub (u8, u8) {
    let x = Field::from(x);
    let y = Field::from(y);
    let mut a: Field = x;
    let mut p = &mut a;
    let q = &mut p;
    let arr = [(p, q)];
    *p = y;
    write(q, 99);
    (u8::unsafe_from(*p), u8::unsafe_from(*(arr[0].0)))
}
