//! Integer widths are numbers: every width the language has resolves, binds, and monomorphizes
//! the same way under every field configuration, in one binary.

use acvm::{FieldConfig, FieldId};
use num_bigint::{BigInt, BigUint};
use num_traits::One;

use crate::hir::def_collector::dc_crate::CompilationError;
use crate::hir::resolution::errors::ResolverError;
use crate::hir::type_check::TypeCheckError;
use crate::monomorphization::ast::Type as MonomorphizedType;
use crate::monomorphization::errors::MonomorphizationError;
use crate::shared::{MAX_INTEGER_WIDTH, Signedness};
use crate::test_utils::{GetProgramOptions, get_monomorphized_for_field, get_program_with_options};
use crate::tests::{get_program_errors, get_program_errors_for_field};

/// Widths the circuit backend does not lower, from the narrowest the language has to the widest.
const UNLOWERABLE_WIDTHS: [u32; 7] = [2, 3, 33, 66, 130, 256, MAX_INTEGER_WIDTH];

fn unsupported_widths(errors: &[CompilationError]) -> Vec<(Signedness, u32)> {
    errors
        .iter()
        .filter_map(|error| match error {
            CompilationError::ResolverError(ResolverError::UnsupportedIntegerWidth {
                signedness,
                bits,
                ..
            }) => Some((*signedness, *bits)),
            _ => None,
        })
        .collect()
}

fn literal_ranges(errors: &[CompilationError]) -> Vec<String> {
    errors
        .iter()
        .filter_map(|error| match error {
            CompilationError::TypeError(TypeCheckError::IntegerLiteralDoesNotFitItsType {
                range,
                ..
            }) => Some(range.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn named_and_parametric_widths_are_the_same_type_in_every_position() {
    let src = "
        struct Wrapper<let N: u32> { inner: u<N>, named: u34 }
        trait Width { fn width(self) -> u32; }
        impl Width for u<34> { fn width(self) -> u32 { 34 } }
        fn widen<let N: u32>(x: u<N>) -> u<2 * N> { x as u::<2 * N> }
        fn annotated(x: u::<34>) -> i<66> {
            let y: u<34> = x;
            let z: u34 = y;
            (z as u::<66>) as i66
        }
        fn main(x: u34) -> pub u68 {
            let wrapper = Wrapper::<34> { inner: x, named: x };
            assert(wrapper.inner == wrapper.named);
            assert(x.width() == 34);
            assert(annotated(x) == 1);
            widen(x)
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");

        let program = get_monomorphized_for_field(src, field).unwrap();
        let main = &program.functions[0];
        assert_eq!(
            main.parameters[0].3.as_ref(),
            &MonomorphizedType::Integer(Signedness::Unsigned, 34)
        );
        assert_eq!(main.return_type, MonomorphizedType::Integer(Signedness::Unsigned, 68));
        assert!(program.to_string().contains("u68"), "{field}: {program}");
    }
}

#[test]
fn every_legal_width_is_nameable_including_i128_and_the_widest() {
    let src = "
        fn main() {
            let a: i128 = -1;
            let b: u16384 = 1;
            let c: i16384 = -1;
            let d: u<16384> = 1;
            let e: u8 = 1;
            let f: i32 = -1;
            assert(a == -1);
            assert(b == d);
            assert(c == -1);
            assert(e == 1);
            assert(f == -1);
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}

#[test]
fn widths_the_language_does_not_have_are_refused_where_they_are_written() {
    for (name, signedness, bits) in [
        ("u16385", Signedness::Unsigned, 16385),
        ("i<16385>", Signedness::Signed, 16385),
        ("i0", Signedness::Signed, 0),
        ("u<0>", Signedness::Unsigned, 0),
    ] {
        let src = format!("fn main() {{ let _: {name} = 0; }}");
        let errors = get_program_errors(&src);
        assert_eq!(unsupported_widths(&errors), vec![(signedness, bits)], "{name}: {errors:?}");
    }

    // A leading zero is not an integer type name.
    let errors = get_program_errors("fn main() { let _: u08 = 0; }");
    assert!(unsupported_widths(&errors).is_empty(), "{errors:?}");
    assert!(!errors.is_empty());
}

#[test]
fn a_generic_width_is_checked_once_it_is_bound() {
    let src = "
        fn shifted<let N: u32>() -> u<N + 16384> { 0 }
        fn main() {
            let _ = shifted::<1>();
        }
    ";
    for field in FieldId::ALL {
        assert!(get_program_errors_for_field(src, field).is_empty());
        let error = get_monomorphized_for_field(src, field).unwrap_err();
        assert!(
            matches!(
                error,
                MonomorphizationError::UnsupportedIntegerWidth {
                    signedness: Signedness::Unsigned,
                    bits: 16385,
                    ..
                }
            ),
            "{field}: {error:?}"
        );
    }
}

#[test]
fn literal_bounds_hold_at_every_width() {
    for bits in UNLOWERABLE_WIDTHS {
        for signedness in [Signedness::Unsigned, Signedness::Signed] {
            let name = format!("{}{bits}", signedness.type_name_prefix());
            let (min, max) = match signedness {
                Signedness::Unsigned => (BigInt::ZERO, (BigInt::one() << bits) - 1),
                Signedness::Signed => {
                    let half = BigInt::one() << (bits - 1);
                    (-half.clone(), half - 1)
                }
            };
            let range = format!("{min}..={max}");

            for field in FieldId::ALL {
                for value in [&min, &max] {
                    let src = format!("fn main() {{ let _: {name} = {value}; }}");
                    let errors = get_program_errors_for_field(&src, field);
                    assert!(errors.is_empty(), "{field}, {name} = {value}: {errors:?}");
                }
                for value in [&min - BigInt::one(), &max + BigInt::one()] {
                    let src = format!("fn main() {{ let _: {name} = {value}; }}");
                    let errors = get_program_errors_for_field(&src, field);
                    // One past the widest unsigned type is also one past the lexer's ceiling,
                    // which reports it before any type is known.
                    let ceiling: BigUint = BigUint::one() << MAX_INTEGER_WIDTH;
                    let lexes = *value.magnitude() < ceiling;
                    if lexes {
                        assert_eq!(
                            literal_ranges(&errors),
                            vec![range.clone()],
                            "{field}, {name} = {value}"
                        );
                    } else {
                        assert!(
                            errors
                                .iter()
                                .any(|error| matches!(error, CompilationError::ParseError(_))),
                            "{field}, {name} = {value}: {errors:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn comptime_arithmetic_and_casts_work_at_wide_widths() {
    let widest_max = (BigInt::one() << MAX_INTEGER_WIDTH) - 1;
    let src = format!(
        "
        fn main() {{
            comptime {{
                let a: u34 = 17179869183;
                let b: u66 = a as u66;
                assert(b == 17179869183);
                let c: u66 = b * 4;
                assert(c == 68719476732);
                let d: u34 = (c + 1) as u::<34>;
                assert(d == 17179869181);
                let e: i130 = -1;
                let f: u130 = e as u130;
                assert(f == 1361129467683753853853498429727072845823);
                let g: u256 = 1 << 255;
                assert(g == 57896044618658097711785492504343953926634992332820282019728792003956564819968);
                let h: i16384 = -1;
                assert((h as u16384) == {widest_max});
                assert(((h as u16384) & 1) == 1);
            }}
        }}
    "
    );
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(&src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }

    let overflow = "
        fn main() {
            comptime {
                let a: u34 = 17179869183;
                let _ = a + 1;
            }
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(overflow, field);
        assert!(
            errors.iter().any(|error| error.to_string().to_lowercase().contains("overflow")),
            "{field}: {errors:?}"
        );
    }
}

#[test]
fn integer_operators_unify_widths() {
    let src = "
        fn add<let N: u32>(a: u<N>, b: u<N>) -> u<N> { a + b }
        fn commuted<let N: u32>(a: u<N + 1>) -> u<1 + N> { a }
        fn main() {
            assert(add(1 as u34, 2) == 3);
            assert(commuted::<33>(5 as u34) == 5);
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }

    let mismatch = "
        fn main() {
            let a: u34 = 1;
            let b: u66 = 2;
            let _ = a + b;
        }
    ";
    let errors = get_program_errors(mismatch);
    assert!(
        errors.iter().any(|error| matches!(
            error,
            CompilationError::TypeError(TypeCheckError::IntegerBitWidth { bit_width_x, bit_width_y, .. })
                if bit_width_x.to_string() == "34" && bit_width_y.to_string() == "66"
        )),
        "{errors:?}"
    );
}

#[test]
fn a_generic_impl_on_an_integer_family_binds_the_width() {
    let src = "
        trait Width { fn width(self) -> u32; }
        impl<let N: u32> Width for u<N> { fn width(self) -> u32 { N } }
        impl<let N: u32> Width for i<N> { fn width(self) -> u32 { N + 1000 } }
        fn main() {
            assert((1 as u8).width() == 8);
            assert((1 as u34).width() == 34);
            assert((1 as i64).width() == 1064);
            assert((1 as i256).width() == 1256);
        }
    ";
    for field in FieldId::ALL {
        let errors = get_program_errors_for_field(src, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");
        let program = get_monomorphized_for_field(src, field).unwrap();
        assert!(program.to_string().contains("u34"), "{field}: {program}");
    }
}

#[test]
fn as_integer_reports_the_width_as_a_u32() {
    let src = "
        struct Option<T> { _is_some: bool, _value: T }
        impl<T> Option<T> { fn unwrap(self) -> T { assert(self._is_some); self._value } }
        impl Quoted {
            #[builtin(quoted_as_type)]
            comptime fn as_type(self) -> Type {}
        }
        impl Type {
            #[builtin(type_as_integer)]
            comptime fn as_integer(self) -> Option<(bool, u32)> {}
        }
        fn main() {
            comptime {
                let (signed, bits) = quote { u34 }.as_type().as_integer().unwrap();
                assert(!signed);
                assert(bits == 34);
                let (signed, bits) = quote { i<256> }.as_type().as_integer().unwrap();
                assert(signed);
                assert(bits == 256);
                let (_, bits) = quote { u16384 }.as_type().as_integer().unwrap();
                let widest: u32 = bits;
                assert(widest == 16384);
                assert(quote { Field }.as_type().as_integer()._is_some == false);
            }
        }
    ";
    for field in FieldId::ALL {
        let options =
            GetProgramOptions { root_and_stdlib: true, ..GetProgramOptions::for_field(field) };
        let errors = get_program_with_options(src, options).2;
        assert!(errors.is_empty(), "{field}: {errors:?}");
    }
}

#[test]
fn the_width_of_an_integer_family_is_a_u32_generic() {
    let errors = get_program_errors("fn narrow<let N: u8>(x: u<N>) -> u<N> { x } fn main() {}");
    assert!(
        errors.iter().any(|error| matches!(
            error,
            CompilationError::TypeError(TypeCheckError::TypeKindMismatch { .. })
        )),
        "{errors:?}"
    );

    let errors = get_program_errors("fn dependent<let N: u32, let M: u<N>>() {} fn main() {}");
    assert!(
        errors.iter().any(|error| matches!(
            error,
            CompilationError::ResolverError(ResolverError::UnsupportedNumericGenericType(_))
        )),
        "{errors:?}"
    );
}

#[test]
fn a_generic_width_at_an_entry_point_is_refused_before_monomorphization() {
    let errors = get_program_errors("fn main<let N: u32>(x: u<N>) { assert(x == x); }");
    assert!(!errors.is_empty());
}

#[test]
fn casts_to_field_follow_the_field_rule_at_every_width() {
    for (bits, generic) in [(66, false), (66, true), (256, false), (256, true)] {
        let src = if generic {
            format!(
                "fn to_field<let N: u32>(x: u<N>) -> Field {{ x as Field }}
                 fn main(x: u{bits}) -> pub Field {{ to_field(x) }}"
            )
        } else {
            format!("fn main(x: u{bits}) -> pub Field {{ x as Field }}")
        };
        for field in FieldId::ALL {
            let fits = FieldConfig::new(field).fits_unsigned(bits);
            if generic {
                // The type checker sees `u<N>`, so the rule is applied by the monomorphizer.
                assert!(get_program_errors_for_field(&src, field).is_empty());
                let result = get_monomorphized_for_field(&src, field);
                assert_eq!(result.is_ok(), fits, "{field}, u{bits} through a generic: {result:?}");
                if !fits {
                    assert!(matches!(
                        result.unwrap_err(),
                        MonomorphizationError::InvalidFieldCast {
                            err: TypeCheckError::IntegerTypeExceedsField { .. },
                            ..
                        }
                    ));
                }
            } else {
                let errors = get_program_errors_for_field(&src, field);
                let refused = errors.iter().any(|error| {
                    matches!(
                        error,
                        CompilationError::TypeError(TypeCheckError::IntegerTypeExceedsField { .. })
                    )
                });
                assert_eq!(refused, !fits, "{field}, u{bits}: {errors:?}");
            }
        }
    }
}

#[test]
fn the_downsizing_warning_reaches_widths_above_128_bits() {
    // An untyped literal in a cast is a `Field` first, so it must stay below the modulus: only
    // a field wider than the target width can carry a value the warning is about.
    let field = FieldId::Bn254;
    let two_to_200 = BigInt::one() << 200;
    let src = format!("fn main() {{ let _ = {two_to_200} as u130; }}");
    let errors = get_program_errors_for_field(&src, field);
    assert!(
        errors.iter().any(|error| matches!(
            error,
            CompilationError::TypeError(TypeCheckError::DownsizingCast { .. })
        )),
        "{errors:?}"
    );

    let two_to_129 = BigInt::one() << 129;
    let src = format!("fn main() {{ let _ = {two_to_129} as u130; }}");
    let errors = get_program_errors_for_field(&src, field);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_match_on_a_wide_integer_still_needs_a_catch_all() {
    let errors = get_program_errors("fn main(x: u256) -> pub u8 { match x { 0 => 1, _ => 3, } }");
    assert!(errors.is_empty(), "{errors:?}");

    let errors = get_program_errors("fn main(x: u256) -> pub u8 { match x { 0 => 1, } }");
    assert!(
        errors.iter().any(|error| matches!(
            error,
            CompilationError::TypeError(TypeCheckError::MissingCases { cases, .. })
                if cases.iter().any(|case| case == "_")
        )),
        "{errors:?}"
    );
}

/// The type checker cannot bound a literal whose type has a generic width, so the monomorphizer
/// applies the bound once the width is a number rather than letting the literal through.
#[test]
fn a_literal_of_a_generic_width_is_bounded_once_the_width_is_bound() {
    let src = "
        fn largest<let N: u32>() -> u<N> { 300 }
        fn main() {
            assert(largest::<16>() == 300);
            let _ = largest::<8>();
        }
    ";
    for field in FieldId::ALL {
        assert!(get_program_errors_for_field(src, field).is_empty());
        let error = get_monomorphized_for_field(src, field).unwrap_err();
        assert!(
            matches!(
                &error,
                MonomorphizationError::IntegerLiteralDoesNotFitItsType {
                    err: TypeCheckError::IntegerLiteralDoesNotFitItsType { range, .. },
                    ..
                } if range == "0..=255"
            ),
            "{field}: {error:?}"
        );
    }

    let fits = "
        fn largest<let N: u32>() -> u<N> { 255 }
        fn main() { assert(largest::<8>() == 255); }
    ";
    for field in FieldId::ALL {
        assert!(get_monomorphized_for_field(fits, field).is_ok(), "{field}");
    }
}

/// A negated literal is one literal whatever its size, so a `Field` literal is bounded by its
/// magnitude: `-1` is `p - 1`, while `-p` has no canonical representative.
#[test]
fn negative_field_literals_are_bounded_by_their_magnitude() {
    for field in FieldId::ALL {
        let modulus = FieldConfig::new(field).modulus();
        let largest = format!("fn main() {{ let _: Field = -{}; }}", modulus - 1u8);
        let errors = get_program_errors_for_field(&largest, field);
        assert!(errors.is_empty(), "{field}: {errors:?}");

        let too_large = format!("fn main() {{ let _: Field = -{modulus}; }}");
        let errors = get_program_errors_for_field(&too_large, field);
        assert_eq!(literal_ranges(&errors), vec![format!("0..{modulus}")], "{field}: {errors:?}");
    }
}

/// The integer families are consulted the way the named primitive types are, after the
/// program's own items, so a type named `u` keeps its meaning and `i` stays a plain identifier.
#[test]
fn the_integer_families_come_after_the_programs_own_items() {
    let src = "
        struct u<T> { inner: T }
        fn main() {
            let value: u<Field> = u { inner: 1 };
            assert(value.inner == 1);
            let i = 5;
            for u in 0..i {
                assert(u < i);
            }
        }
    ";
    let errors = get_program_errors(src);
    assert!(errors.is_empty(), "{errors:?}");
}
