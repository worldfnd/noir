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

fn main() {
    let x: Field = std::hint::black_box(1);
    let y: Field = std::hint::black_box(2);
    let first = Foo::my_default(x, y);
    assert(first.bar == x);
}
