fn main() -> pub u8 {
    // Safety: testing context
    let result = unsafe { foo() };
    assert(result == 1);
    result
}

unconstrained fn foo() -> u8 {
    bar()
}

unconstrained fn bar() -> u8 {
    while true {}
    1
}
