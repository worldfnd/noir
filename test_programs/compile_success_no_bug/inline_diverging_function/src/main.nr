fn main() -> pub u8 {
    // Safety: testing context
    let result = unsafe { foo() };
    assert(result == 1);
    result as u8
}

unconstrained fn foo() -> Field {
    bar()
}

unconstrained fn bar() -> Field {
    while true {}
    1
}
