trait MyDefault {
    fn my_default(x: Field, y: Field) -> Self;
}

struct Foo {
    bar: Field,
    array: [Field; 2],
}

impl MyDefault for Foo {
    fn my_default(x: Field, y: Field) -> Self {
        Self { bar: x, array: [x, y] }
    }
}

fn main(x: u64, y: u64) {
    let x = x as Field;
    let y = y as Field;
    let first = Foo::my_default(x, y);
    assert(first.bar == x);
}
