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
use crate::tests::{assert_no_errors, check_errors_with_options, get_program_errors_for_field};
use crate::validity::InvalidType;

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
        let src = format!("fn main() {{ let x: u64 = {}; assert(x != 0); }}", u64::MAX);
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

    fn main(x: u32) -> pub Field {
        apply(|v| v as Field, x as u64)
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
               fn main() { assert(value::<18446744069414584320_u64 + 1_u64>() != 0); }";
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

/// The field-driven reason an entry point type is refused, looking past the aliases and struct
/// fields that contain it.
fn integer_exceeding_field(errors: &[CompilationError]) -> Option<(String, FieldId)> {
    errors.iter().find_map(|error| match error {
        CompilationError::TypeError(TypeCheckError::InvalidTypeForEntryPoint {
            invalid_type,
            ..
        }) => match invalid_type.innermost() {
            InvalidType::IntegerExceedsField { typ, field } => Some((typ.to_string(), *field)),
            _ => None,
        },
        _ => None,
    })
}

/// The lowerability reason an entry point type is refused, looking past the aliases and struct
/// fields that contain it.
fn integer_not_lowerable(errors: &[CompilationError]) -> Option<String> {
    errors.iter().find_map(|error| match error {
        CompilationError::TypeError(TypeCheckError::InvalidTypeForEntryPoint {
            invalid_type,
            ..
        }) => match invalid_type.innermost() {
            InvalidType::IntegerNotLowerable { typ } => Some(typ.to_string()),
            _ => None,
        },
        _ => None,
    })
}

/// One field element carries each integer across the entry point, as its unsigned bit pattern,
/// so a type crosses only if every pattern it can hold is below the modulus. Of the nine
/// lowerable types, bn254 and bls12_381 carry every one and Goldilocks stops at 32 bits.
#[test]
fn entry_point_integers_must_fit_below_the_modulus() {
    for field in FieldId::ALL {
        let refused: &[&str] = match field {
            FieldId::Goldilocks => &["u64", "u128", "i64"],
            FieldId::Bn254 | FieldId::Bls12_381 => &[],
        };
        for (sign, widths) in [("u", &[8u32, 16, 32, 64, 128][..]), ("i", &[8, 16, 32, 64][..])] {
            for &width in widths {
                let typ = format!("{sign}{width}");
                let fits = !refused.contains(&typ.as_str());
                for src in [
                    format!("fn main(x: {typ}) {{ assert(x == x); }}"),
                    format!("fn main(x: pub {typ}) {{ assert(x == x); }}"),
                    format!("fn main() -> pub {typ} {{ 0 }}"),
                ] {
                    let errors = get_program_errors_for_field(&src, field);
                    if fits {
                        assert!(errors.is_empty(), "{field}: `{src}`: {errors:?}");
                    } else {
                        assert_eq!(errors.len(), 1, "{field}: `{src}`: {errors:?}");
                        assert_eq!(
                            integer_exceeding_field(&errors),
                            Some((typ.clone(), field)),
                            "{field}: `{src}`: {errors:?}"
                        );
                    }
                }
            }
        }
    }
}

/// An entry point carries only the integer types the circuit backend lowers, so the `Prover.toml`
/// input language is a fixed list: any other width, however narrow, is refused at `main` under
/// every field, wherever the type holds it, while a helper keeps every width.
#[test]
fn entry_point_integers_are_the_lowerable_types() {
    for field in FieldId::ALL {
        for typ in ["u2", "u24", "u34", "u253", "u16384", "i2", "i66", "i128"] {
            for src in [
                format!("fn main(x: {typ}) {{ assert(x == x); }}"),
                format!("fn main(x: pub {typ}) {{ assert(x == x); }}"),
                format!("fn main() -> pub {typ} {{ 0 }}"),
                format!("fn main(x: [{typ}; 2]) {{ assert(x[0] == x[1]); }}"),
                format!("fn main(x: (Field, {typ})) {{ assert(x.1 == x.1); }}"),
                format!(
                    "struct Pair {{ a: Field, b: {typ} }}
                     fn main(x: Pair) {{ assert(x.b == x.b); }}"
                ),
                format!(
                    "type Word = {typ};
                     fn main(x: Word) {{ assert(x == x); }}"
                ),
            ] {
                let errors = get_program_errors_for_field(&src, field);
                assert_eq!(errors.len(), 1, "{field}: `{src}`: {errors:?}");
                assert_eq!(
                    integer_not_lowerable(&errors).as_deref(),
                    Some(typ),
                    "{field}: `{src}`: {errors:?}"
                );
            }

            let helper = format!(
                "fn helper(x: {typ}) -> {typ} {{ x }}
                 fn main(x: u8) -> pub u8 {{ helper(x as {typ}) as u8 }}"
            );
            let errors = get_program_errors_for_field(&helper, field);
            assert!(errors.is_empty(), "{field}: `{helper}`: {errors:?}");
        }
    }
}

#[test]
fn the_entry_point_diagnostic_lists_the_lowerable_types() {
    let src = "
    fn main(x: u24) {
               ^^^ Invalid type found in the entry point to a program
               ~~~ Integers the circuit backend does not lower are not valid entry point types. Found: u24
        assert(x == x);
    }
    ";
    let options = GetProgramOptions { allow_elaborator_errors: true, ..Default::default() };
    check_errors_with_options(src, false, options);

    let errors =
        get_program_errors_for_field("fn main(x: u24) { assert(x == x); }", FieldId::Bn254);
    let diagnostic = CustomDiagnostic::from(&errors[0]);
    assert_eq!(
        diagnostic.notes,
        vec![
            "Note: main and contract functions take and return only the integer types the circuit backend lowers: u8, u16, u32, u64, u128, i8, i16, i32 and i64. Widen or narrow the value inside the program."
                .to_string()
        ]
    );
}

/// The rule reaches an integer wherever the entry point's type holds it, including a width that
/// only a struct's generic argument or an arithmetic expression spells out.
#[test]
fn the_entry_point_rule_looks_through_every_aggregate() {
    let programs = [
        "fn main(x: [u64; 2]) { assert(x[0] == x[1]); }",
        "fn main(x: (Field, i64)) { assert(x.1 == x.1); }",
        "struct Pair { a: Field, b: u64 }
         fn main(x: Pair) { assert(x.b == x.b); }",
        "struct Inner { value: i64 }
         struct Outer { inner: [Inner; 2] }
         fn main(x: Outer) { assert(x.inner[0].value == x.inner[1].value); }",
        "type Word = u64;
         fn main(x: Word) { assert(x == x); }",
        "struct Wrapper<let N: u32> { inner: u<N> }
         fn main(x: Wrapper<64>) { assert(x.inner == x.inner); }",
        "fn main(x: u<2 * 32>) { assert(x == x); }",
        "fn main() -> pub (Field, [u64; 1]) { (0, [0]) }",
        "fn main() -> pub [u64; 0] { [] }",
    ];
    for src in programs {
        let errors = get_program_errors_for_field(src, FieldId::Bn254);
        assert!(errors.is_empty(), "bn254: `{src}`: {errors:?}");

        let errors = get_program_errors_for_field(src, FieldId::Goldilocks);
        assert_eq!(errors.len(), 1, "goldilocks: `{src}`: {errors:?}");
        assert!(
            matches!(integer_exceeding_field(&errors), Some((_, FieldId::Goldilocks))),
            "goldilocks: `{src}`: {errors:?}"
        );
    }
}

/// Only values that cross the entry point are carried in one field element and spelled as a
/// lowerable type: helpers, folded functions, test and fuzz functions and values computed
/// inside the circuit keep every width.
#[test]
fn the_entry_point_rule_leaves_other_functions_alone() {
    let src = "
        fn helper(x: u64, y: u24) -> u64 { x + y as u64 }

        #[fold]
        fn folded(x: u64, y: u24) -> u64 { x * 2 + y as u64 }

        #[test]
        fn tested(x: u64, y: u24) { assert(x == x); assert(y == y); }

        #[fuzz]
        fn fuzzed(x: u64, y: u24) { assert(x == x); assert(y == y); }

        fn main(x: u32, y: Field, z: bool) -> pub u32 {
            let wide: u128 = (x as u128) << 64;
            let sum = helper(x as u64, x as u24) + folded(x as u64, x as u24) + ((wide >> 64) as u64);
            assert(y == y);
            assert(z == z);
            sum as u32
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}

/// A contract function is an entry point too; a library method in the contract is not.
#[test]
fn contract_functions_follow_the_entry_point_rule() {
    let src = "
        contract Wallet {
            pub fn balance(x: u64) -> pub u64 { x }

            #[contract_library_method]
            pub fn double(x: u64) -> u64 { x * 2 }
        }
    ";
    let errors = get_program_errors_for_field(src, FieldId::Bn254);
    assert!(errors.is_empty(), "bn254: {errors:?}");

    let errors = get_program_errors_for_field(src, FieldId::Goldilocks);
    assert_eq!(errors.len(), 2, "goldilocks: {errors:?}");
    for error in &errors {
        assert!(
            matches!(
                integer_exceeding_field(std::slice::from_ref(error)),
                Some((typ, FieldId::Goldilocks)) if typ == "u64"
            ),
            "goldilocks: {errors:?}"
        );
    }

    let src = "
        contract Wallet {
            pub fn balance(x: u24) -> pub u24 { x }

            #[contract_library_method]
            pub fn double(x: u24) -> u24 { x * 2 }
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert_eq!(errors.len(), 2, "{field}: {errors:?}");
        for error in &errors {
            assert_eq!(
                integer_not_lowerable(std::slice::from_ref(error)).as_deref(),
                Some("u24"),
                "{field}: {errors:?}"
            );
        }
    }
}

#[test]
fn the_entry_point_diagnostic_names_the_field_and_the_widest_width() {
    let src = "
    struct Pair {
           ~~~~ Struct Pair has an invalid entry point type
        a: Field,
        b: u64,
        ~ Field b has an invalid entry point type
        ~ Integers wider than 63 bits are not valid entry point types under goldilocks. Found: u64
    }

    fn main(x: Pair, y: i64) {
               ^^^^ Invalid type found in the entry point to a program
               ~~~~ This type has an invalid entry point type inside it
                        ^^^ Invalid type found in the entry point to a program
                        ~~~ Integers wider than 63 bits are not valid entry point types under goldilocks. Found: i64
        assert(x.b == x.b);
        assert(y == y);
    }
    ";
    let options = GetProgramOptions {
        allow_elaborator_errors: true,
        ..GetProgramOptions::for_field(FieldId::Goldilocks)
    };
    check_errors_with_options(src, false, options);

    let errors =
        get_program_errors_for_field("fn main(x: u64) { assert(x == x); }", FieldId::Goldilocks);
    let diagnostic = CustomDiagnostic::from(&errors[0]);
    assert_eq!(
        diagnostic.notes,
        vec![
            "Note: under goldilocks, an integer that main or a contract function takes or returns must be at most 63 bits wide, so that each of its values is below the field modulus. Split a wider integer into narrower ones."
                .to_string()
        ]
    );
}
