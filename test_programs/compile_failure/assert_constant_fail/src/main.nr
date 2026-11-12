use std::assert_constant;

fn main(x: u32) {
    foo(5, x as Field);
}

fn foo(constant: Field, non_constant: Field) {
    assert_constant(constant);
    assert_constant(non_constant);
}
