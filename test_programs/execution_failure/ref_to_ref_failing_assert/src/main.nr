// The same false assertion with no array literal, so `p` and `q` stay in separate alias
// classes and the call is reached through the `points_to` chain alone.
fn write(q: &mut &mut Field, v: Field) {
    let p = *q;
    *p = v;
}

fn main(x: u8, y: u8) {
    let x = Field::from(x);
    let y = Field::from(y);
    let mut a: Field = x;
    let mut p = &mut a;
    let q = &mut p;
    *p = y;
    write(q, y * 2);
    assert(*p == y);
}
