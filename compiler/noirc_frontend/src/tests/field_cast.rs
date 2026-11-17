//! Casts between `Field` and the integer types: where one may be written, and what it computes.

use acvm::{FieldConfig, FieldId};
use num_bigint::BigUint;

use crate::hir::def_collector::dc_crate::CompilationError;
use crate::hir::type_check::TypeCheckError;
use crate::test_utils::get_monomorphized_with_stdlib;
use crate::tests::get_stdlib_errors_for_field;

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
