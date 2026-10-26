global TWO: Field = 2;
global THREE: Field = 3;

fn main(x: u8, y: pub u8) {
    // Safety: testing context
    unsafe {
        entry_point_one(x as Field, y as Field);
        entry_point_two(x as Field, y as Field);
    }
}

unconstrained fn entry_point_one(x: Field, y: Field) {
    let z = TWO + x + y;
    assert(z == 3);
    inner_func(x, y);
}

// Identical to `entry_point_one`
unconstrained fn entry_point_two(x: Field, y: Field) {
    let z = TWO + x + y;
    assert(z == 3);
    inner_func(x, y);
}

unconstrained fn inner_func(x: Field, y: Field) {
    let z = TWO + x + y;
    assert(z == 3);
    nested_inner_func(x, y);
}

unconstrained fn nested_inner_func(x: Field, y: Field) {
    let z = THREE + x + y;
    assert(z == 4);
}
