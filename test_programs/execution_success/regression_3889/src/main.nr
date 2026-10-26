mod Foo {
    pub(crate) struct NewType {
        pub(crate) a: u8,
    }
}

mod Bar {
    pub(crate) use crate::Foo::NewType;
    pub(crate) use crate::Foo::NewType as BarStruct;
}

mod Baz {
    pub(crate) struct Works {
        pub(crate) a: u8,
    }
    pub(crate) use crate::Bar::BarStruct;
    pub(crate) use crate::Bar::NewType;
}

fn main(works: Baz::Works, fails: Baz::BarStruct, also_fails: Bar::NewType) -> pub u8 {
    works.a + fails.a + also_fails.a
}
