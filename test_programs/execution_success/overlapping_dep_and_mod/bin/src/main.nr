use std::convert::UnsafeFrom;
// bin/main.nr

fn main() -> pub u8 {
    assert(foo::bar() + ::foo::bar() == 11);
    assert(foo::bar() == 5);
    assert(::foo::bar() == 6);
    u8::unsafe_from(foo::bar() + ::foo::bar())
}

mod foo {
    pub(crate) fn bar() -> Field {
        5
    }
}
