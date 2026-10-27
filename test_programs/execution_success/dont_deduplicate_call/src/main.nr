fn main(x: u8, active: bool) {
    let x = x as Field;
    let mut x1 = false;
    // Safety:)
    unsafe {
        if active {
            x1 = foo(x);
        }

        let x2 = foo(x);
        assert(x2);
    }
}

unconstrained fn foo(_x: Field) -> bool {
    true
}
