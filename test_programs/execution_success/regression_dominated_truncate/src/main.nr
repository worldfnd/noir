use std::convert::UnsafeFrom;
unconstrained fn helper(cond: bool, x: Field) -> Field {
    if cond {
        Field::from(u8::unsafe_from(x))
    } else {
        x.assert_max_bit_size::<8>();
        0
    }
}

fn main(cond: bool, x: pub u16) {
    let x = Field::from(x);
    //Safety: test for 'remove truncate after range check'
    let y = unsafe { helper(cond, x) };
    assert(y != x);
}
