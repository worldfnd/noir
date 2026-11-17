//! Casts between `Field` and the integer types: where one may be written, and what it computes.

use acvm::{FieldConfig, FieldId};
use num_bigint::BigUint;

use crate::hir::comptime::ComptimeError;
use crate::hir::def_collector::dc_crate::CompilationError;
use crate::hir::type_check::TypeCheckError;
use crate::test_utils::get_monomorphized_with_stdlib;
use crate::tests::{get_program_errors, get_program_errors_for_field, get_stdlib_errors_for_field};

/// The standard library's wrapping conversion, as `convert.nr` defines it.
const UNSAFE_FROM: &str = "
    pub trait UnsafeFrom<T> {
        fn unsafe_from(input: T) -> Self;
    }

    impl<let N: u32> UnsafeFrom<u<N>> for Field {
        fn unsafe_from(input: u<N>) -> Field {
            input as Field
        }
    }

    impl<let N: u32> UnsafeFrom<i<N>> for Field {
        fn unsafe_from(input: i<N>) -> Field {
            input as u::<N> as Field
        }
    }

    impl<let N: u32> UnsafeFrom<Field> for u<N> {
        fn unsafe_from(input: Field) -> u<N> {
            input as Self
        }
    }

    impl<let N: u32> UnsafeFrom<Field> for i<N> {
        fn unsafe_from(input: Field) -> i<N> {
            input as Self
        }
    }
";

/// The errors of compiling `src` as the standard library under `field`, warnings left out.
fn stdlib_errors(src: &str, field: FieldId) -> Vec<CompilationError> {
    let errors = get_stdlib_errors_for_field(src, field);
    errors.into_iter().filter(CompilationError::is_error).collect()
}

fn exceeds_field(errors: &[CompilationError]) -> bool {
    errors.iter().any(|error| {
        matches!(error, CompilationError::TypeError(TypeCheckError::IntegerTypeExceedsField { .. }))
    })
}

fn is_denial(error: &CompilationError) -> bool {
    matches!(error, CompilationError::TypeError(TypeCheckError::FieldCastDenied { .. }))
}

fn signed_to_field(errors: &[CompilationError]) -> bool {
    errors.iter().any(|error| {
        matches!(error, CompilationError::TypeError(TypeCheckError::UnsupportedFieldCast { .. }))
    })
}

/// A closure's parameter is still a type variable where the cast is written, so the rule is
/// applied once inference has bound it.
#[test]
fn a_cast_source_bound_by_inference_is_checked_by_the_type_checker() {
    let unsigned = "fn apply<T>(f: fn(T) -> Field, x: T) -> Field { f(x) }

    fn main(x: u32) {
        let _ = apply(|v| v as Field, x as u64);
    }";
    for field in FieldId::ALL {
        let errors = stdlib_errors(unsigned, field);
        let fits = FieldConfig::new(field).fits_unsigned(64);
        assert_eq!(exceeds_field(&errors), !fits, "{field}: {errors:?}");
        assert_eq!(errors.is_empty(), fits, "{field}: {errors:?}");
    }

    let signed = "fn apply<T>(f: fn(T) -> Field, x: T) -> Field { f(x) }

    fn main(x: i8) {
        let _ = apply(|v| v as Field, x);
    }";
    let errors = stdlib_errors(signed, FieldId::linked());
    assert!(signed_to_field(&errors), "{errors:?}");
}

/// A macro argument is checked in a context of its own, which ends before the variable it casts
/// is bound.
#[test]
fn a_cast_source_bound_after_a_macro_argument_is_still_checked() {
    let src = "comptime fn quoted(_x: Field) -> Quoted { quote { 0 } }

    comptime fn caller() -> i8 {
        let x = 3;
        let _ = quoted!(x as Field);
        x
    }

    fn main() {
        comptime {
            let _ = caller();
        }
    }";
    let errors = stdlib_errors(src, FieldId::linked());
    assert!(signed_to_field(&errors), "{errors:?}");
}

/// Some width of `u<N>` can exceed any modulus, so the cast is refused whatever `N` is bound to.
/// The one exception is the body of the standard library's `UnsafeFrom` impls.
#[test]
fn a_width_generic_cast_to_field_is_refused_outside_unsafe_from() {
    let helper = "fn to_field<let N: u32>(x: u<N>) -> Field { x as Field }

    fn main(x: u8) {
        let _ = to_field(x);
    }";
    let through_the_trait = format!(
        "{UNSAFE_FROM}
        fn main(x: u8) {{
            let _ = Field::unsafe_from(x);
        }}"
    );
    for field in FieldId::ALL {
        let errors = stdlib_errors(helper, field);
        assert!(exceeds_field(&errors), "{field}: {errors:?}");

        let errors = stdlib_errors(&through_the_trait, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");

        // A trait of that name outside the standard library is user code like any other.
        let errors = get_program_errors_for_field(&through_the_trait, field);
        assert!(errors.iter().all(is_denial) && errors.len() == 4, "{field}: {errors:?}");
    }
}

/// Into `Field` the conversion reduces modulo the field, and a signed source is read as its bit
/// pattern; out of `Field` it keeps the low bits.
#[test]
fn unsafe_from_wraps_under_every_field() {
    let u64_max = BigUint::from(u64::MAX);
    let u256_max = (BigUint::from(1u8) << 256u32) - 1u8;
    for field in FieldId::ALL {
        let modulus = FieldConfig::new(field).modulus();
        let src = format!(
            "{UNSAFE_FROM}
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

/// The conversions are nothing but the casts they are written with, which is what a backend
/// reading the monomorphized program lowers.
#[test]
fn unsafe_from_monomorphizes_to_plain_casts() {
    let src = "
    fn main(x: u64, y: i8, z: u8) -> pub u8 {
        let sum = Field::unsafe_from(x) + Field::unsafe_from(y);
        u8::unsafe_from(sum) + z
    }";
    let program = get_monomorphized_with_stdlib(src, &[UNSAFE_FROM]).unwrap();
    insta::assert_snapshot!(program, @r"
    fn main$f0(x$l0: u64, y$l1: i8, z$l2: u8) -> pub u8 {
        let sum$l3 = (unsafe_from$f1(x$l0) + unsafe_from$f2(y$l1));
        (unsafe_from$f3(sum$l3) + z$l2)
    }
    fn unsafe_from$f1(input$l4: u64) -> Field {
        (input$l4 as Field)
    }
    fn unsafe_from$f2(input$l5: i8) -> Field {
        ((input$l5 as u8) as Field)
    }
    fn unsafe_from$f3(input$l6: Field) -> u8 {
        (input$l6 as u8)
    }
    ");
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

#[test]
fn the_stdlib_may_cast() {
    let src = "fn main(x: u8) {
        let f = x as Field;
        let _ = f as u8;
    }";
    for field in FieldId::ALL {
        let errors = stdlib_errors(src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}
