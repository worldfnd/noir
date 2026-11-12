contract TestContract {
    use test::module::foo;

    fn constructor(param: u32, pub_param: pub u32) -> pub ([u8; 32], u32) {
        (foo::bar(param as Field).to_le_bytes(), param + pub_param)
    }

    fn someFunction() -> pub u32 {
        42
    }
}
