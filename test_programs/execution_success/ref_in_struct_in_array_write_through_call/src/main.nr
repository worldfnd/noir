use std::convert::UnsafeFrom;
// A struct holding a `&mut` flattens to the same SSA as a `&mut &mut`: `&mut h` reaches
// `cell`'s memory in two steps, and `arr` holds both references.
struct Holder {
    cell: &mut Field,
}

fn write(h: &mut Holder, v: Field) {
    *h.cell = v;
}

fn main(x: u8, y: u8) -> pub u8 {
    let x = Field::from(x);
    let y = Field::from(y);
    let mut a: Field = x;
    let cell = &mut a;
    let mut h = Holder { cell };
    let arr = [(cell, &mut h)];
    *cell = y;
    write(&mut h, y * 2);
    u8::unsafe_from(*(arr[0].0))
}
