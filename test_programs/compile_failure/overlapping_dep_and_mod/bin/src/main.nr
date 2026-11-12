fn main() -> pub u32 {
    assert(foo::bar() + foo::baz() == 3);
    assert(foo::bar() == 1);
    assert(foo::baz() == 2);
    foo::bar() + foo::baz()
}

mod foo {
    pub(crate) fn bar() -> u32 {
        1
    }
}
