fn bar(y: Field) {
    assert(y != 0);
}

fn foo(x: Field) {
    // Safety: test
    let y = unsafe { baz(x) };
    bar(y);
}

unconstrained fn baz(x: Field) -> Field {
    x
}

fn main(x: u8, y: pub u8) {
    let x = Field::from(x);
    let y = Field::from(y);
    // Safety: test
    let x = unsafe { baz(x) };
    foo(x);
    foo(y);
    assert(x != y);
}
