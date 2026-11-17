use std::convert::UnsafeFrom;
// Regression test for issue "V-NSCA-VUL-003: Missing ArraySet case in Removing Unreachable Functions pass"
// found in Veridise Audit.

fn main(x: u8, y: pub u8) -> pub u16 {
    assert(x != y);
    let mut f = [my_fun];
    f[0] = my_fun2;
    u16::unsafe_from((f[0])(Field::from(x)))
}

fn my_fun(x: Field) -> Field {
    x + 1
}

fn my_fun2(x: Field) -> Field {
    x + 2
}
