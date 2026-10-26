// bin/main.nr

fn main() -> pub u8 {
    assert(foo::bar() + ::foo::bar() == 11);
    assert(foo::bar() == 5);
    assert(::foo::bar() == 6);
    (foo::bar() + ::foo::bar()) as u8
}

mod foo {
    pub(crate) fn bar() -> Field {
        5
    }
}
