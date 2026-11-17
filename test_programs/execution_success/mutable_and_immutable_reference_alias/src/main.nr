use std::convert::UnsafeFrom;
unconstrained fn main() -> pub u8 {
    let mut value = 1;
    u8::unsafe_from(foo(&mut value, &value))
}

unconstrained fn foo(v0: &mut Field, v1: &Field) -> Field {
    let _ = *v1;
    *v0 = 2;
    *v1
}
