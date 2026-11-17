use std::convert::UnsafeFrom;
fn read_first(r: &[Field; 3]) -> Field {
    r[0]
}

fn dispatch(f: fn(&[Field; 3]) -> Field, x: &mut [Field; 3]) -> Field {
    f(x)
}

fn main(cond: bool) -> pub u8 {
    let mut arr = [5, 6, 7];
    let r = if cond { read_first } else { read_first };
    u8::unsafe_from(dispatch(r, &mut arr))
}
