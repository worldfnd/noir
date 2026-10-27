global FN_TUPLE: (fn() -> Field, fn() -> Field) = (f1, f2);

fn main(y: u8) {
    let y = y as Field;
    assert((FN_TUPLE.0)() != y);
    assert((FN_TUPLE.1)() == y);
}

fn f1() -> Field {
    1
}

fn f2() -> Field {
    2
}
