use std::convert::UnsafeFrom;
fn main(c: bool) -> pub u32 {
    let mut s = 0;
    if c {
        for i in 0..150 {
            for j in 0..150 {
                if c {
                    s += (Field::from(i));
                } else {
                    s += (Field::from(j));
                }
            }
        }
    }
    u32::unsafe_from(s)
}
