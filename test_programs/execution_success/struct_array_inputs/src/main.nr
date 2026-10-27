struct Foo {
    bar: u8,
    baz: u8,
}

fn main(foos: [Foo; 3]) -> pub u8 {
    foos[2].bar + foos[2].baz
}
