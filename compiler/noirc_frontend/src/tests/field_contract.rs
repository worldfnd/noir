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
