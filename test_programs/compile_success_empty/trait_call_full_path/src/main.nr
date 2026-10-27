mod foo {
    pub trait Trait {
        fn me(self) -> Self;
    }

    impl Trait for Field {
        fn me(self) -> Self {
            self
        }
    }
}

use foo::Trait;

fn main() {
    let x: Field = std::hint::black_box(1);
    let _ = foo::Trait::me(x);
    let _ = Trait::me(x);
}
