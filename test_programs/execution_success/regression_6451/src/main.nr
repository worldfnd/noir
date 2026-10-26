fn main(x: u8) {
    // Regression test for #6451
    let y = unsafe { empty(x as Field) };
    let mut value = 0;
    let term1 = x as Field * x as Field - x as Field * y;
    std::as_witness(term1);
    value += term1;
    let term2 = x as Field * x as Field - y * x as Field;
    value += term2;
    value.assert_max_bit_size::<1>();

    // Regression test for #6447 (Aztec Packages issue #9488)
    // Safety: testing context
    let y = unsafe { empty(x as Field + 1) };
    let z = y + x as Field + 1;
    let z1 = z + y;
    assert(z + z1 != 3);
    let w = y + 2 * x as Field + 3;
    assert(w + z1 != z);
}

unconstrained fn empty(_: Field) -> Field {
    0
}
