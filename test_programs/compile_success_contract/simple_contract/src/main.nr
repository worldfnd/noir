contract Foo {
    fn double(x: u32) -> pub u32 {
        x * 2
    }
    fn triple(x: u32) -> pub u32 {
        x * 3
    }
    fn quadruple(x: u32) -> pub u32 {
        x * 4
    }
    // Regression for issue #3344
    #[contract_library_method]
    pub fn foo(x: u8) -> u8 {
        x
    }
}
