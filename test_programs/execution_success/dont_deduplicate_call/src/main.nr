fn main(x: u8, active: bool) {
    let mut x1 = false;
    // Safety:)
    unsafe {
        if active {
            x1 = foo(x as Field);
        }

        let x2 = foo(x as Field);
        assert(x2);
    }
}

unconstrained fn foo(_x: Field) -> bool {
    true
}
