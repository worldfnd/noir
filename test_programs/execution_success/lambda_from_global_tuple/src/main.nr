global FN_TUPLE: (fn() -> Field, fn() -> Field) = (f1, f2);

fn main(y: u8) {
    assert((FN_TUPLE.0)() != y as Field);
    assert((FN_TUPLE.1)() == y as Field);
}

fn f1() -> Field {
    1
}

fn f2() -> Field {
    2
}
