unconstrained fn helper(cond: bool, x: Field) -> Field {
    if cond {
        (x as u8) as Field
    } else {
        x.assert_max_bit_size::<8>();
        0
    }
}

fn main(cond: bool, x: pub u16) {
    //Safety: test for 'remove truncate after range check'
    let y = unsafe { helper(cond, x as Field) };
    assert(y != x as Field);
}
