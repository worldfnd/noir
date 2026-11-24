use std::convert::UnsafeFrom;
unconstrained fn main(a: bool) -> pub u8 {
    if ((!a) > (!a)) {
        u8::unsafe_from(-(-(a as Field)))
    } else {
        u8::unsafe_from(-(-(a as Field)))
    }
}
