//! Field rules across configurations, each compiled and evaluated in one binary.

use acvm::{FieldConfig, FieldId};
use noirc_errors::CustomDiagnostic;
use num_bigint::BigUint;

use crate::hir::def_collector::dc_crate::CompilationError;
use crate::hir::type_check::TypeCheckError;
use crate::test_utils::{
    GetProgramOptions, get_monomorphized, get_monomorphized_for_field, get_program_with_options,
    stdlib_src,
};
use crate::tests::{assert_no_errors, get_program_errors_for_field};

#[test]
fn field_literals_must_be_canonical() {
    for field in FieldId::ALL {
        let modulus = FieldConfig::new(field).modulus();

        let largest = format!("fn main() {{ let _: Field = {}; }}", modulus - 1u8);
        let errors = get_program_errors_for_field(&largest, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");

        for too_large in [
            format!("fn main() {{ let _: Field = {modulus}; }}"),
            format!(
                "fn value<let N: Field>() -> Field {{ N }}
                 fn main() -> pub Field {{ value::<{modulus}_Field>() }}"
            ),
        ] {
            let errors = get_program_errors_for_field(&too_large, field);
            assert!(
                errors.iter().any(|error| matches!(
                    error,
                    CompilationError::TypeError(TypeCheckError::IntegerLiteralDoesNotFitItsType { range, .. })
                        if range == &format!("0..{modulus}")
                )),
                "{field}: expected the literal `{modulus}` to be rejected, got {errors:?}"
            );
        }
    }
}

#[test]
fn integer_literals_above_the_modulus_stay_exact() {
    for field in FieldId::ALL {
        let src = format!("fn main() -> pub u64 {{ {} }}", u64::MAX);
        let program = get_monomorphized_for_field(&src, field).unwrap().to_string();
        assert!(program.contains(&u64::MAX.to_string()), "{field}: {program}");
    }
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
    for field in FieldId::ALL {
        let config = FieldConfig::new(field);
        for bits in [8u32, 16, 32, 64, 128] {
            let src = format!("fn main() {{ let x: u{bits} = 1; let _ = x as Field; }}");
            let errors = get_program_errors_for_field(&src, field);
            let refused = errors.iter().any(|error| {
                matches!(
                    error,
                    CompilationError::TypeError(TypeCheckError::IntegerTypeExceedsField { .. })
                )
            });
            if config.fits_unsigned(bits) {
                assert!(
                    errors.is_empty(),
                    "{field}: u{bits} fits below the modulus, got {errors:?}"
                );
            } else {
                assert!(refused, "{field}: u{bits} can exceed the modulus, got {errors:?}");
            }
        }
    }
}

/// The type checker runs before the comptime interpreter, so a `comptime` block is refused too.
#[test]
fn comptime_casts_to_field_follow_the_same_rule() {
    for field in FieldId::ALL {
        let accepted = FieldConfig::new(field).fits_unsigned(64);
        let src = format!(
            "fn main() {{
                comptime {{
                    let x: u64 = {};
                    let _ = x as Field;
                }}
            }}",
            u64::MAX
        );
        let errors = get_program_errors_for_field(&src, field);
        if accepted {
            assert!(errors.is_empty(), "{field}: {errors:?}");
        } else {
            assert!(
                errors.iter().any(|error| matches!(
                    error,
                    CompilationError::TypeError(TypeCheckError::IntegerTypeExceedsField { .. })
                )),
                "{field}: {errors:?}"
            );
        }
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

/// The type checker sees the cast while the source is still a type variable, so the rule is
/// applied again once inference has bound it, before the cast reaches the monomorphized AST.
#[test]
fn casts_to_field_through_an_inferred_type_follow_the_same_rule() {
    let src = "fn apply<T>(f: fn(T) -> Field, x: T) -> Field { f(x) }

    fn main(x: u64) -> pub Field {
        apply(|v| v as Field, x)
    }";
    for field in FieldId::ALL {
        let result = get_monomorphized_for_field(src, field);
        if FieldConfig::new(field).fits_unsigned(64) {
            assert!(result.is_ok(), "{field}: {result:?}");
        } else {
            let error = result.expect_err("u64 can exceed the modulus");
            let diagnostic = CustomDiagnostic::from(error);
            assert!(
                diagnostic.message.contains("can exceed the field modulus"),
                "{field}: {}",
                diagnostic.message
            );
        }
    }
}

#[test]
fn signed_casts_to_field_through_an_inferred_type_are_refused() {
    let src = "fn apply<T>(f: fn(T) -> Field, x: T) -> Field { f(x) }

    fn main(x: i8) -> pub Field {
        apply(|v| v as Field, x)
    }";
    let error = get_monomorphized(src).expect_err("a signed source is never cast to Field");
    let diagnostic = CustomDiagnostic::from(error);
    assert!(
        diagnostic.message.contains("Only unsigned integer types may be casted to Field"),
        "{}",
        diagnostic.message
    );
}

#[test]
fn comptime_modulus_builtins_describe_the_configured_field() {
    for field in FieldId::ALL {
        let config = FieldConfig::new(field);
        let le_bytes = config.modulus().to_bytes_le();
        let last = le_bytes.len() - 1;
        let src = format!(
            "{}
        fn main() {{
            comptime {{
                assert(modulus_num_bits() == {});
                let le_bytes = modulus_le_bytes();
                assert(le_bytes[0] == {});
                assert(le_bytes[{last}] == {});
                let be_bits = modulus_be_bits();
                assert(be_bits[0]);
            }}
        }}",
            stdlib_src::MODULUS,
            config.num_bits(),
            le_bytes[0],
            le_bytes[last],
        );
        let options =
            GetProgramOptions { root_and_stdlib: true, ..GetProgramOptions::for_field(field) };
        let errors = get_program_with_options(&src, options).2;
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}

#[test]
fn comptime_field_arithmetic_wraps_under_the_configured_field() {
    for field in FieldId::ALL {
        let largest = FieldConfig::new(field).modulus() - 1u8;
        let src = format!(
            "fn main() {{
                comptime {{
                    let largest: Field = {largest};
                    assert(largest + 1 == 0);
                    assert(0 - 1 == largest);
                    assert(largest * largest == 1);
                    assert(2 * (1 / 2) == 1);
                    assert(-largest == 1);
                }}
            }}"
        );
        let errors = get_program_errors_for_field(&src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}

#[test]
fn a_comptime_field_value_reaches_the_program_in_the_configured_field() {
    let largest = FieldConfig::new(FieldId::Goldilocks).modulus() - 1u8;
    let src = format!("fn main() -> pub Field {{ comptime {{ {largest} + 1 }} }}");

    for field in FieldId::ALL {
        let result =
            if field == FieldId::Goldilocks { BigUint::ZERO } else { largest.clone() + 1u8 };
        let program = get_monomorphized_for_field(&src, field).unwrap().to_string();
        assert_eq!(
            program.trim(),
            format!("fn main$f0() -> pub Field {{\n    {result}\n}}"),
            "{field}"
        );
    }
}

#[test]
fn type_level_field_arithmetic_wraps_under_the_configured_field() {
    for field in FieldId::ALL {
        let largest = FieldConfig::new(field).modulus() - 1u8;
        let src = format!(
            "fn value<let N: Field>() -> Field {{ N }}
             fn main() -> pub Field {{ value::<{largest}_Field + 1_Field>() }}"
        );
        let program = get_monomorphized_for_field(&src, field).unwrap().to_string();
        assert!(program.contains("fn value$f1() -> Field {\n    0\n}"), "{field}: {program}");
    }
}

#[test]
fn type_level_integer_arithmetic_works_under_every_field() {
    let src = "fn value<let N: u64>() -> u64 { N }
               fn main() -> pub u64 { value::<18446744069414584320_u64 + 1_u64>() }";
    for field in FieldId::ALL {
        let program = get_monomorphized_for_field(src, field).unwrap().to_string();
        assert!(program.contains("18446744069414584321"), "{field}: {program}");
    }
}

#[test]
fn comptime_crypto_uses_the_configured_field() {
    let programs = [
        ("poseidon2_permutation", "
            #[foreign(poseidon2_permutation)]
            fn permute(input: [Field; 4]) -> [Field; 4] {}
            fn main() { comptime { let result = permute([0, 1, 2, 3]);
                assert(result[0] == 0x01bd538c2ee014ed5141b29e9ae240bf8db3fe5b9a38629a9647cf8d76c01737);
            } }
        "),
        ("derive_pedersen_generators", "
            struct Point { x: Field, y: Field }
            #[builtin(derive_pedersen_generators)]
            fn generators(domain: [u8; 1], start: u32) -> [Point; 1] {}
            fn main() { comptime { let point = generators([0], 0)[0];
                assert(point.x != 0);
                assert(point.y * point.y == point.x * point.x * point.x - 17);
            } }
        "),
    ];
    for (builtin, source) in programs {
        for field in FieldId::ALL {
            let options =
                GetProgramOptions { root_and_stdlib: true, ..GetProgramOptions::for_field(field) };
            let errors = get_program_with_options(source, options).2;
            if field == FieldId::Bn254 {
                assert!(errors.is_empty(), "{builtin}: {errors:?}");
            } else {
                assert!(errors.iter().any(|error| matches!(error,
                    CompilationError::InterpreterError(crate::hir::comptime::InterpreterError::Unimplemented { item, .. })
                        if item.starts_with(builtin)
                )), "{field}, {builtin}: {errors:?}");
            }
        }
    }
}
