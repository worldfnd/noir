use crate::tests::{assert_no_errors, check_errors};

#[test]
fn cast_suffixed_negative_literal_to_integer_does_not_warn() {
    let src = r#"
        fn main() {
            let _ = -1i8 as i8;
        }
    "#;
    assert_no_errors(src);
}

#[test]
fn do_not_eagerly_error_on_cast_on_type_variable() {
    let src = r#"
    pub fn foo<T, U>(x: T, f: fn(T) -> U) -> U {
        f(x)
    }

    fn main() {
        let x: u8 = 1;
        let _: u16 = foo(x, |x| x as u16);
    }
    "#;
    assert_no_errors(src);
}

#[test]
fn error_on_cast_over_type_variable() {
    let src = r#"
    pub fn foo<T, U>(f: fn(T) -> U, x: T, ) -> U {
        f(x)
    }

    fn main() {
        let x = "a";
        let _: Field = foo(|x| x as Field, x);
                                           ^ Expected type Field, found type str<1>
    }
    "#;
    check_errors(src);
}

#[test]
fn cast_numeric_to_bool() {
    let src = "
    fn main() {
        let x: u64 = 1;
        let _ = x as bool;
                ^^^^^^^^^ Cannot cast `u64` as `bool`
                ~~~~~~~~~ Compare with zero instead: ` != 0`
    }
    ";
    check_errors(src);
}

#[test]
fn cast_field_and_integers_to_bool() {
    let src = "
    fn main() {
        let x = 1;
        let _ = x as bool;
                ^^^^^^^^^ Cannot cast `Field` as `bool`
                ~~~~~~~~~ Compare with zero instead: ` != 0`

        let x: i32 = 1;
        let _ = x as bool;
                ^^^^^^^^^ Cannot cast `i32` as `bool`
                ~~~~~~~~~ Compare with zero instead: ` != 0`

        let x: u64 = 1;
        let _ = x as bool;
                ^^^^^^^^^ Cannot cast `u64` as `bool`
                ~~~~~~~~~ Compare with zero instead: ` != 0`
    }
    ";
    check_errors(src);
}

#[test]
fn cast_numeric_to_bool_comptime() {
    let src = "
    fn main() {
        comptime {
            let x: u64 = 1;
            let _ = x as bool;
                    ^^^^^^^^^ Cannot cast `u64` as `bool`
                    ~~~~~~~~~ Compare with zero instead: ` != 0`
        }
    }
    ";
    check_errors(src);
}

#[test]
fn u1_type_is_removed() {
    let src = r#"
        fn main() {
            let _x: u1 = 0;
                    ^^ `u1` has been removed, use `bool` instead
        }
    "#;
    check_errors(src);
}
