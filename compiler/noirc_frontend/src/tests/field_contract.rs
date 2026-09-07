//! The value rules for `Field`, written as modulus queries so they hold under every field.

// TODO: parameterize by FieldConfig and run under every field row in the default build.

use acvm::{AcirField, FieldElement};

use crate::hir::def_collector::dc_crate::CompilationError;
use crate::hir::type_check::TypeCheckError;
use crate::test_utils::get_monomorphized;
use crate::tests::{assert_no_errors, get_program_errors};

#[test]
fn field_literals_must_be_canonical() {
    let modulus = FieldElement::modulus();

    let largest = format!("fn main() {{ let _: Field = {}; }}", &modulus - 1u8);
    assert_no_errors(&largest);

    let too_large = format!("fn main() {{ let _: Field = {modulus}; }}");
    let errors = get_program_errors(&too_large);
    assert!(
        errors.iter().any(|error| matches!(
            error,
            CompilationError::TypeError(TypeCheckError::IntegerLiteralDoesNotFitItsType { .. })
        )),
        "expected the literal `{modulus}` to be rejected, got {errors:?}"
    );
}

#[test]
fn integer_literals_above_the_modulus_stay_exact() {
    let src = format!("fn main() -> pub u64 {{ {} }}", u64::MAX);
    let program = get_monomorphized(&src).unwrap().to_string();
    assert!(program.contains(&u64::MAX.to_string()), "{program}");
}

/// A signed value reaches `Field` only through an unsigned cast, since `i8 as Field` is a type error.
#[test]
fn comptime_casts_wrap_to_the_target_width() {
    let src = "fn main() {
        comptime {
            let wide: u16 = 256;
            assert((wide as u8) == 0);
            let negative: i8 = -1;
            assert((negative as u8) == 255);
            assert(((negative as u8) as Field) == 255);
        }
    }";
    assert_no_errors(src);
}

/// An unsigned type is cast to `Field` only if every one of its values is below the modulus;
/// `Field` never reduces a cast. The escape hatch is an explicit narrowing cast.
#[test]
fn casts_to_field_are_refused_when_the_type_can_exceed_the_modulus() {
    let field_bits = FieldElement::max_num_bits();
    for bits in [8u32, 16, 32, 64, 128] {
        let src = format!("fn main() {{ let x: u{bits} = 1; let _ = x as Field; }}");
        let errors = get_program_errors(&src);
        let refused = errors.iter().any(|error| {
            matches!(
                error,
                CompilationError::TypeError(TypeCheckError::IntegerTypeExceedsField { .. })
            )
        });
        if bits < field_bits {
            assert!(errors.is_empty(), "u{bits} fits below the modulus, got {errors:?}");
        } else {
            assert!(refused, "u{bits} can exceed the modulus, got {errors:?}");
        }
    }
}

/// The type checker runs before the comptime interpreter, so a `comptime` block is refused too.
#[test]
fn comptime_casts_to_field_follow_the_same_rule() {
    let src = format!(
        "fn main() {{
            comptime {{
                let x: u64 = {};
                let _ = x as Field;
            }}
        }}",
        u64::MAX
    );
    let errors = get_program_errors(&src);
    if 64 < FieldElement::max_num_bits() {
        assert!(errors.is_empty(), "{errors:?}");
    } else {
        assert!(
            errors.iter().any(|error| matches!(
                error,
                CompilationError::TypeError(TypeCheckError::IntegerTypeExceedsField { .. })
            )),
            "{errors:?}"
        );
    }
}

#[test]
fn narrowing_before_a_field_cast_is_accepted_under_every_field() {
    let src = format!(
        "fn main() {{
            let x: u128 = {};
            assert(((x as u32) as Field) == {});
            comptime {{
                let x: u128 = {};
                assert(((x as u32) as Field) == {});
            }}
        }}",
        u128::MAX,
        u32::MAX,
        u128::MAX,
        u32::MAX
    );
    assert_no_errors(&src);
}
