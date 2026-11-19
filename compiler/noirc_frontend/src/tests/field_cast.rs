//! Casts between `Field` and the integer types: where one may be written, and what it computes.

use acvm::{FieldConfig, FieldId};
use num_bigint::BigUint;

use crate::hir::comptime::ComptimeError;
use crate::hir::def_collector::dc_crate::CompilationError;
use crate::hir::type_check::TypeCheckError;
use crate::test_utils::stdlib_src;
use crate::tests::{get_program_errors, get_program_errors_for_field, get_stdlib_errors_for_field};

/// The errors of compiling `src` as the standard library under `field`, warnings left out.
fn stdlib_errors(src: &str, field: FieldId) -> Vec<CompilationError> {
    let errors = get_stdlib_errors_for_field(src, field);
    errors.into_iter().filter(CompilationError::is_error).collect()
}

fn is_denial(error: &CompilationError) -> bool {
    matches!(error, CompilationError::TypeError(TypeCheckError::FieldCastDenied { .. }))
}

/// Into `Field` the conversion reduces modulo the field, and a signed source is read as its bit
/// pattern; out of `Field` it keeps the low bits.
#[test]
fn unsafe_from_wraps_under_every_field() {
    let u64_max = BigUint::from(u64::MAX);
    let u256_max = (BigUint::from(1u8) << 256u32) - 1u8;
    let unsafe_from = stdlib_src::UNSAFE_FROM;
    for field in FieldId::ALL {
        let modulus = FieldConfig::new(field).modulus();
        let src = format!(
            "{unsafe_from}
            fn main() {{
                comptime {{
                    let wide: u64 = {u64_max};
                    assert(Field::unsafe_from(wide) == {});
                    let wider: u256 = {u256_max};
                    assert(Field::unsafe_from(wider) == {});
                    let negative: i8 = -1;
                    assert(Field::unsafe_from(negative) == 255);

                    let element: Field = 0x1234;
                    assert(u8::unsafe_from(element) == 0x34);
                    let top: Field = {};
                    assert(u256::unsafe_from(top) == {});
                    let byte: Field = 255;
                    assert(i8::unsafe_from(byte) == -1);
                }}
            }}",
            &u64_max % modulus,
            &u256_max % modulus,
            modulus - 1u8,
            modulus - 1u8,
        );
        let errors = stdlib_errors(&src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}

/// Asserts that `src`, compiled as user code, gets the denial and no other diagnostic, under
/// every field: the rule does not depend on the field.
fn assert_denied(src: &str) {
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert!(errors.len() == 1 && is_denial(&errors[0]), "{field}: {src}\n{errors:?}");
    }
}

/// Asserts that `src`, compiled as user code, gets no diagnostic at all under any field.
fn assert_allowed(src: &str) {
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert!(errors.is_empty(), "{field}: {src}\n{errors:?}");
    }
}

#[test]
fn casting_an_integer_to_field_is_denied() {
    for (typ, value) in [("u8", "1"), ("u64", "1"), ("u256", "1"), ("i8", "-1")] {
        assert_denied(&format!("fn main() {{ let x: {typ} = {value}; let _ = x as Field; }}"));
    }

    // A generic width, the value of a numeric generic and a loop index are integers too.
    assert_denied(
        "fn to_field<let N: u32>(x: u<N>) -> Field { x as Field }
        fn main(x: u8) { let _ = to_field(x); }",
    );
    assert_denied(
        "fn length<let N: u32>(_array: [u8; N]) -> Field { N as Field }
        fn main() { let _ = length([1, 2]); }",
    );
    assert_denied(
        "fn main() {
            let mut sum: Field = 0;
            for i in 0..3 { sum += i as Field; }
            assert(sum == 3);
        }",
    );
}

#[test]
fn casting_field_to_an_integer_is_denied() {
    for typ in ["u8", "i8", "u64", "u256", "i128"] {
        assert_denied(&format!("fn main() {{ let x: Field = 1; let _ = x as {typ}; }}"));
    }
}

/// The source may be a type variable where the cast is written. It is judged as what inference
/// binds it to, or as the `Field` it defaults to.
#[test]
fn the_denial_follows_inference() {
    assert_denied(
        "fn apply<T>(f: fn(T) -> Field, x: T) -> Field { f(x) }
        fn main(x: u8) { let _ = apply(|v| v as Field, x); }",
    );
    assert_denied(
        "fn main() {
            let mut sum = 0;
            sum += 1;
            let _ = sum as u32;
        }",
    );
    assert_denied(
        "fn main() {
            let limit = 10;
            let _ = limit as u32;
        }",
    );
}

#[test]
fn the_denial_sees_through_aliases_and_self() {
    assert_denied(
        "type Element = Field;
        fn main(x: u8) { let _ = x as Element; }",
    );
    assert_denied(
        "trait Low { fn low(self) -> u8; }
        impl Low for Field { fn low(self) -> u8 { self as u8 } }
        fn main() { let x: Field = 1; let _ = x.low(); }",
    );
}

/// An integer literal that fits its target is that number under every field, whether or not
/// it is written with the `Field` type it defaults to.
#[test]
fn a_literal_that_fits_its_target_may_be_cast() {
    assert_allowed(
        "fn main() {
            let _ = 5 as u8;
            let _ = 255 as u8;
            let _ = 0 as u64;
            let _ = 127 as i8;
            let _ = 5_Field as u8;
            let _ = 13 as Field;
        }",
    );
}

/// A literal outside its target would be cut down to the target's low bits, which for a
/// negative one are the low bits of the modulus.
#[test]
fn a_literal_outside_its_target_is_denied() {
    for cast in ["256 as u8", "128 as i8", "-1 as u8", "-1 as i8"] {
        assert_denied(&format!("fn main() {{ let _ = {cast}; }}"));
    }

    let errors = get_program_errors("fn main() { let _ = -1 as i8; }");
    let diagnostic = noirc_errors::CustomDiagnostic::from(&errors[0]);
    let secondary = &diagnostic.secondaries[0].message;
    assert!(secondary.contains("`-1i8`"), "{secondary}");
}

/// Every legal width holds `0..=3` unsigned and `0..=1` signed, so such a literal is the same
/// number at whatever width a generic is bound to.
#[test]
fn a_literal_that_fits_every_width_may_be_cast_to_a_generic_width() {
    let program = |cast: &str, typ: &str| {
        format!(
            "fn literal<let W: u32>() -> {typ}<W> {{ {cast} }}
            fn main() {{ let _ = literal::<8>(); }}"
        )
    };
    for (cast, typ) in [("0 as u::<W>", "u"), ("3 as u::<W>", "u"), ("1 as i::<W>", "i")] {
        assert_allowed(&program(cast, typ));
    }
    for (cast, typ) in [("4 as u::<W>", "u"), ("2 as i::<W>", "i"), ("-1 as i::<W>", "i")] {
        assert_denied(&program(cast, typ));
    }
}

/// The note under the one error of compiling `src` as user code.
fn denial_note(src: &str) -> String {
    let errors = get_program_errors(src);
    assert!(errors.len() == 1 && is_denial(&errors[0]), "{src}\n{errors:?}");
    let diagnostic = noirc_errors::CustomDiagnostic::from(&errors[0]);
    diagnostic.secondaries[0].message.clone()
}

/// A value that is a `Field` only because nothing gave it another type is fixed by giving it
/// one, not by converting it. A value declared a `Field` is converted.
#[test]
fn the_denial_of_a_defaulted_value_suggests_giving_it_a_type() {
    assert_eq!(
        denial_note("fn main() { let limit = 10; let _ = limit as u32; }"),
        "`limit` is a `Field` only because nothing gives it another type: declare it with one, such as `u32`",
    );
    assert_eq!(
        denial_note("fn main() { let s = [1, 2, 3]; let _ = s[0] as u8; }"),
        "This value is a `Field` only because nothing gives it another type: declare it with one, such as `u8`",
    );
    // Bound to `Field` by a use after the cast, the value is a `Field` by more than default.
    assert_eq!(
        denial_note("fn main() { let x = 10; let _ = x as u32; let _f: Field = x; }"),
        "Use `TryFrom` to get `None` when the value does not fit, or `UnsafeFrom` to wrap; both are in `std::convert`",
    );
    assert_eq!(
        denial_note("fn main() { let x: Field = 10; let _ = x as u32; }"),
        "Use `TryFrom` to get `None` when the value does not fit, or `UnsafeFrom` to wrap; both are in `std::convert`",
    );
}

/// What a `comptime` block evaluates to is a literal in the program, and is judged as one.
#[test]
fn comptime_blocks_cannot_launder_a_field_value() {
    assert_denied(
        "fn main() {
            let _ = comptime { let x: Field = 300; x } as u8;
        }",
    );
    assert_allowed(
        "fn main() {
            let _ = comptime { let x: Field = 200; x } as u8;
        }",
    );
}

#[test]
fn casts_that_do_not_cross_the_field_boundary_are_untouched() {
    assert_allowed(
        "fn main(x: u8, b: bool) {
            let f: Field = 1;
            let _ = b as Field;
            let _ = f as Field;
            let _ = x as u64;
            let _ = x as i8;
            let _ = b as u8;
        }",
    );
}

#[test]
fn comptime_code_is_denied_too() {
    assert_denied(
        "fn main() {
            comptime {
                let x: u8 = 1;
                let _ = x as Field;
            }
        }",
    );
    assert_denied(
        "comptime fn low(x: Field) -> u8 { x as u8 }
        fn main() {
            comptime { let _ = low(1); }
        }",
    );
    assert_denied(
        "comptime global BYTE: u8 = { let x: Field = 1; x as u8 };
        fn main() {
            comptime { let _ = BYTE; }
        }",
    );
}

/// Code a macro or an attribute writes is judged in the crate it lands in.
#[test]
fn macro_output_is_judged_where_it_lands() {
    assert_denied(
        "comptime fn to_field(x: Quoted) -> Quoted { quote { $x as Field } }
        fn main(x: u8) { let _ = to_field!(quote { x }); }",
    );

    let errors = get_program_errors(
        "comptime fn with_helper(_f: FunctionDefinition) -> Quoted {
            quote { pub fn helper(x: u8) -> Field { x as Field } }
        }
        #[with_helper]
        fn main() {}",
    );
    assert!(
        matches!(
            errors.as_slice(),
            [CompilationError::ComptimeError(ComptimeError::ErrorRunningAttribute { error, .. })]
                if is_denial(error)
        ),
        "{errors:?}"
    );
}

/// The user-code side of `a_cast_source_bound_after_a_macro_argument_is_still_checked`.
#[test]
fn a_cast_in_a_macro_argument_is_denied_once_its_source_is_bound() {
    assert_denied(
        "comptime fn quoted(_x: u8) -> Quoted { quote { 0 } }

        comptime fn caller() -> Field {
            let x = 3;
            let _ = quoted!(x as u8);
            x
        }

        fn main() {
            comptime { let _ = caller(); }
        }",
    );
}

/// What a macro call expands to is not a literal where the cast is written, in run-time code
/// or in `comptime` code, so its integer defaults to `Field` and the cast is denied.
#[test]
fn a_macro_result_cast_is_denied_in_run_time_and_comptime_code() {
    assert_denied(
        "comptime fn quote_one() -> Quoted { quote { 1 } }
        fn main() { let _: i32 = quote_one!() as i32; }",
    );
    assert_denied(
        "comptime fn quote_one() -> Quoted { quote { 1 } }
        fn main() {
            comptime { let _: i32 = quote_one!() as i32; }
        }",
    );
}

/// No crate may write the cast: the standard library converts through its own builtin.
#[test]
fn the_standard_library_is_denied_the_cast_too() {
    let src = "fn main(x: u8) {
        let f = x as Field;
        let _ = f as u8;
    }";
    for field in FieldId::ALL {
        let errors = stdlib_errors(src, field);
        assert!(errors.iter().all(is_denial) && errors.len() == 2, "{field}: {errors:?}");
    }
}
