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

fn main(x: u64) {
    let x = x as Field;
    let _ = foo::Trait::me(x);
    let _ = Trait::me(x);
}
