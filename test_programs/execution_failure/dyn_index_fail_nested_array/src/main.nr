struct Foo {
    a: u8,
    b: u8,
}

fn main(mut x: [Foo; 3], y: pub u32) {
    assert(x[y + 2].a == 5);
}
