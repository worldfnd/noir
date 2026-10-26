unconstrained fn main() -> pub u8 {
    let mut value = 1;
    foo(&mut value, &value) as u8
}

unconstrained fn foo(v0: &mut Field, v1: &Field) -> Field {
    let _ = *v1;
    *v0 = 2;
    *v1
}
