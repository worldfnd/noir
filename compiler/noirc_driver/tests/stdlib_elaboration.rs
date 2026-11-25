use std::path::Path;

use acvm::{FieldConfig, FieldId};
use noirc_driver::{
    CompileOptions, check_crate_with_comptime_io, file_manager_with_stdlib, prepare_crate,
};
use noirc_errors::CustomDiagnostic;
use noirc_frontend::elaborator::{FrontendOptions, UnstableFeature};
use noirc_frontend::graph::CrateId;
use noirc_frontend::hir::comptime::{ComptimeIo, EvaluationTracker};
use noirc_frontend::hir::def_map::{CrateDefMap, parse_file};
use noirc_frontend::hir::{CheckState, Context, ParsedFiles};
use noirc_frontend::monomorphization::monomorphize;

/// The frontend options these tests elaborate under: `field`, with enums enabled.
fn frontend_options(field: FieldId, generic_builtins: bool) -> FrontendOptions<'static> {
    FrontendOptions {
        debug_comptime_in_file: None,
        enabled_unstable_features: &[UnstableFeature::Enums],
        disable_required_unstable_features: true,
        field: FieldConfig::new(field),
        generic_builtins,
    }
}

fn elaborate(source: &str, field: FieldId) -> Vec<String> {
    let options = frontend_options(field, false);
    diagnostics_with(source, options).into_iter().map(|(_, rendered)| rendered).collect()
}

/// Where comptime code prints, with a tracker: it makes the interpreter answer
/// `is_unconstrained()` from the calling context rather than always `true`, so constrained
/// branches run under `comptime` as well.
fn tracking_io() -> ComptimeIo {
    ComptimeIo {
        evaluation_tracker: Some(EvaluationTracker::new(Default::default())),
        ..ComptimeIo::stdout()
    }
}

/// A context holding the standard library and `source` as `main.nr`, ready to elaborate.
fn context_with_stdlib(source: &str) -> (Context, CrateId) {
    let root = Path::new("");
    let file_name = Path::new("main.nr");
    let mut file_manager = file_manager_with_stdlib(root);
    file_manager.add_file_with_source(file_name, source.to_owned()).unwrap();
    let parsed_files: ParsedFiles = file_manager
        .as_file_map()
        .all_file_ids()
        .map(|&file_id| (file_id, parse_file(&file_manager, file_id)))
        .collect();
    let mut context = Context::new(file_manager, parsed_files);
    let crate_id = prepare_crate(&mut context, file_name);
    (context, crate_id)
}

/// Every diagnostic of elaborating `source`, rendered, with whether it is a warning. The
/// standard library's warnings are included, which `check_crate` would drop.
fn diagnostics_with(source: &str, options: FrontendOptions) -> Vec<(bool, String)> {
    let (mut context, crate_id) = context_with_stdlib(source);
    let (mut comptime_io, mut state) = (tracking_io(), CheckState::default());
    let errors =
        CrateDefMap::collect_defs(crate_id, &mut context, options, &mut comptime_io, &mut state);
    errors
        .iter()
        .map(|error| {
            let location = error.location();
            let path = context.file_manager.path(location.file).unwrap().display();
            let diagnostic = CustomDiagnostic::from(error);
            let notes: Vec<_> =
                diagnostic.secondaries.iter().map(|secondary| secondary.message.clone()).collect();
            let rendered = format!(
                "{path} @ {}: {} {}",
                location.span.start(),
                diagnostic.message,
                notes.join(" ")
            );
            (diagnostic.is_warning(), rendered)
        })
        .collect()
}

#[test]
fn numeric_traits_and_limits_work_for_configured_fields() {
    let source = "
        use std::cmp::Ord;
        use std::default::Default;
        use std::ops::{Add, Sub, Mul, Div, Rem, Neg, Not, BitOr, BitAnd, BitXor, Shl, Shr};

        fn exercise<T>(a: T, b: T) -> T
        where
            T: Add + Sub + Mul + Div + Rem + Not + BitOr + BitAnd + BitXor + Shl + Shr + Ord + Default + Eq,
        {
            let c = a.add(b).sub(b).mul(b).div(b).rem(b).bitxor(b);
            let c = c.not().not().bitor(b).bitand(c).shl(b).shr(b);
            if c.cmp(T::default()) == std::cmp::Ordering::less() { b } else { c }
        }

        fn main() {
            comptime {
                assert(exercise(1 as u8, 2 as u8) == 3);
                assert(exercise(1 as u34, 2 as u34) == 3);
                assert(exercise(1 as u256, 2 as u256) == 3);
                assert(exercise(1 as i128, 2 as i128) == 3);
                assert(exercise(1 as i34, 2 as i34) == 3);
                assert((1 as i34).neg() == -1);
                assert(u128::max_value() == 340282366920938463463374607431768211455);
                assert(u34::max_value() == 17179869183);
                assert(u34::min_value() == 0);
                assert(u34::bits() == 34);
                assert(i34::max_value() == 8589934591);
                assert(i34::min_value() == -8589934592);
                assert(i128::max_value() == 170141183460469231731687303715884105727);
                assert(i128::min_value() == -170141183460469231731687303715884105728);
                assert(i16384::bits() == 16384);
                assert(u2::max_value() == 3);
                assert(u2::min_value() == 0);
                assert(i2::max_value() == 1);
                assert(i2::min_value() == -2);
                assert(u2::bits() == 2);
                assert(u3::max_value() == 7);
                assert(i3::min_value() == -4);
            }
        }
    ";
    for field in FieldId::ALL {
        let errors = elaborate(source, field);
        assert!(errors.is_empty(), "{field}: {}", errors.join("\n"));
    }
}

/// `Type::as_integer` describes an integer type of every width the language has: `IntegerType`
/// holds the width in a `u32`, which the widest type needs.
#[test]
fn as_integer_describes_an_integer_type_of_every_width() {
    let source = "
        fn main() {
            comptime {
                let narrow = quote { u34 }.as_type().as_integer().unwrap();
                assert(!narrow.is_signed());
                assert(narrow.bits() == 34);
                let wide = quote { i<256> }.as_type().as_integer().unwrap();
                assert(wide.is_signed());
                assert(wide.bits() == 256);
                assert(quote { u16384 }.as_type().as_integer().unwrap().bits() == 16384);
                assert(quote { Field }.as_type().as_integer().is_none());
            }
        }
    ";
    for field in FieldId::ALL {
        let errors = elaborate(source, field);
        assert!(errors.is_empty(), "{field}: {}", errors.join("\n"));
    }
}

/// Checks that hold when the field-generic halves of `Field::lt`, the wide `Hash` impls and the
/// wrapping ops are the ones compiled; `is_bn254` says which field the checks expect to run under.
fn generic_halves_source(is_bn254: bool) -> String {
    let not = if is_bn254 { "" } else { "!" };
    format!(
        "
        use std::hash::{{Hash, Hasher}};
        use std::ops::{{WrappingAdd, WrappingSub, WrappingMul}};

        struct Limbs {{ values: [Field; 4], len: u32 }}
        impl Hasher for Limbs {{
            fn finish(self) -> Field {{ Field::from(self.len) }}
            fn write(&mut self, input: Field) {{
                self.values[self.len] = input;
                self.len += 1;
            }}
        }}

        unconstrained fn check_unconstrained_order() {{
            assert(Field::lt(0, -1));
            assert(!Field::lt(-1, 0));
        }}

        fn main() {{
            comptime {{
                assert({not}(std::field::modulus_num_bits() == 254));
                let max64 = u64::max_value();
                let max128 = u128::max_value();
                assert(max64.wrapping_add(1) == 0);
                assert(max128.wrapping_add(max128) == max128 - 1);
                assert(0u64.wrapping_sub(1) == max64);
                assert(0u128.wrapping_sub(max128) == 1);
                assert(5u64.wrapping_add(7) == 12);
                assert(7u128.wrapping_sub(5) == 2);
                assert(max64.wrapping_mul(max64) == 1);
                assert(max128.wrapping_mul(max128) == 1);
                assert((1u128 << 64).wrapping_mul(1u128 << 64) == 0);
                assert(i64::max_value().wrapping_add(1) == i64::min_value());
                assert(i64::min_value().wrapping_sub(1) == i64::max_value());
                assert(i64::min_value().wrapping_mul(-1) == i64::min_value());

                let max8 = u8::max_value();
                let max16 = u16::max_value();
                let max32 = u32::max_value();
                assert(max8.wrapping_add(1) == 0);
                assert(0u8.wrapping_sub(1) == max8);
                assert(max8.wrapping_mul(max8) == 1);
                assert(max16.wrapping_add(max16) == max16 - 1);
                assert(0u16.wrapping_sub(max16) == 1);
                assert(max16.wrapping_mul(max16) == 1);
                assert(max32.wrapping_add(1) == 0);
                assert(max32.wrapping_add(max32) == max32 - 1);
                assert(0u32.wrapping_sub(1) == max32);
                assert(0u32.wrapping_sub(max32) == 1);
                assert(max32.wrapping_mul(max32) == 1);
                assert(0x10000u32.wrapping_mul(0x10000) == 0);
                assert(0x12345678u32.wrapping_mul(0x9abcdef0) == 0x242d2080);

                let mut limbs = Limbs {{ values: [0; 4], len: 0 }};
                max64.hash(&mut limbs);
                assert(limbs.len == 2);
                assert(limbs.values == [0xffffffff, 0xffffffff, 0, 0]);
                limbs = Limbs {{ values: [0; 4], len: 0 }};
                0x112233445566778899aabbccddeeff00u128.hash(&mut limbs);
                assert(limbs.len == 4);
                assert(limbs.values == [0xddeeff00, 0x99aabbcc, 0x55667788, 0x11223344]);
                limbs = Limbs {{ values: [0; 4], len: 0 }};
                (-1i64).hash(&mut limbs);
                assert(limbs.values == [0xffffffff, 0xffffffff, 0, 0]);

                assert(Field::lt(0, 1));
                assert(!Field::lt(1, 0));
                assert(!Field::lt(42, 42));
                assert(Field::lt(1, 0x100));
                assert(Field::lt(0x100, -1));
                assert(!Field::lt(-1, 0));
                assert(Field::lt(-2, -1));
                assert(!Field::lt(-1, -2));
                check_unconstrained_order();
            }}
        }}
    "
    )
}

/// The monomorphized program of `source`, compiled against the stdlib under `field`.
fn monomorphized(source: &str, field: FieldId) -> String {
    let (context, crate_id) = context_with_stdlib(source);
    let options = CompileOptions { field, ..Default::default() };
    let (context, result) =
        check_crate_with_comptime_io(context, crate_id, &options, &mut tracking_io());
    result.expect("the program compiles");
    let main = context.get_main_function(&crate_id).expect("main is defined");
    monomorphize(main, &context.def_interner, false).unwrap().to_string()
}

/// Wrapping arithmetic names `Field` only where that takes the fewest R1CS rows: `u128`
/// multiplication under bn254, and `u32` under the other fields, whose modulus holds the sum and
/// the product of two `u32` while the type twice as wide does not fit it. Everything else is
/// integer code.
#[test]
fn integer_wrapping_arithmetic_names_field_only_where_it_is_cheaper() {
    let bn254_halves = [("Mul", 128)];
    for field in FieldId::ALL {
        for op in ["Add", "Sub", "Mul"] {
            for bits in [8, 16, 32, 64, 128] {
                let method = op.to_lowercase();
                let source = format!(
                    "use std::ops::Wrapping{op};
                    fn main(x: u{bits}, y: u{bits}) -> pub u{bits} {{ x.wrapping_{method}(y) }}"
                );
                let program = monomorphized(&source, field);
                let expected = if field == FieldId::Bn254 {
                    bn254_halves.contains(&(op, bits))
                } else {
                    bits == 32
                };
                assert_eq!(
                    program.contains("Field"),
                    expected,
                    "{field}: wrapping_{method} on u{bits}\n{program}"
                );
            }
        }
    }
}

/// A `u64` wraps in `u128` under every field: a field whose modulus a `u128` can reach is one a
/// `u64` reaches already, so the wider type crosses no modulus the operands had not crossed.
#[test]
fn wrapping_arithmetic_on_u64_widens_under_every_field() {
    for field in FieldId::ALL {
        for method in ["add", "sub", "mul"] {
            let source = format!(
                "use std::ops::{{WrappingAdd, WrappingMul, WrappingSub}};
                fn main(x: u64, y: u64) -> pub u64 {{ x.wrapping_{method}(y) }}"
            );
            let program = monomorphized(&source, field);
            assert!(program.contains("as u128"), "{field}: wrapping_{method}\n{program}");
        }
    }
}

/// The limb multiplication of a `u128` divides by a constant. A divisor passed in as an argument
/// is a variable to a backend that does not specialize the callee, and costs it far more.
#[test]
fn wrapping_u128_multiplication_divides_by_a_constant() {
    let source = "use std::ops::WrappingMul;
        fn main(x: u128, y: u128) -> pub u128 { x.wrapping_mul(y) }";
    let program = monomorphized(source, FieldId::Goldilocks);
    // A local is printed as `name$l<id>`, a global as `name$g<id>`.
    let by_a_local = program
        .lines()
        .filter(|line| {
            line.split(" / ").chain(line.split(" % ")).skip(1).any(|divisor| {
                divisor
                    .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
                    .next()
                    .is_some_and(|name| name.contains("$l"))
            })
        })
        .collect::<Vec<_>>();
    assert!(by_a_local.is_empty(), "{by_a_local:?}\n{program}");
}

/// An `Ordering` is one of three small integers, so comparing integers through `Ord` involves
/// no `Field`.
#[test]
fn ordering_holds_no_field() {
    let source = "
        #[derive(Eq, Ord)]
        struct Version { major: u32, minor: u32 }

        fn main(a: u32, b: u32) -> pub bool {
            let (older, newer) = (Version { major: a, minor: b }, Version { major: b, minor: a });
            (older < newer) & (older.cmp(newer) == std::cmp::Ordering::less())
        }";
    for field in FieldId::ALL {
        let program = monomorphized(source, field);
        assert!(!program.contains("Field"), "{field}:\n{program}");
    }
}

/// The standard library has no item that says which field it is compiled for.
#[test]
fn the_standard_library_does_not_name_its_field() {
    let source = "fn main() { assert(std::compat::is_bn254()); }";
    for field in FieldId::ALL {
        let errors = elaborate(source, field);
        assert!(
            errors.iter().any(|error| error.contains("Could not resolve 'compat' in path")),
            "{field}: {errors:?}"
        );
    }
}

#[test]
fn goldilocks_stdlib_fallbacks_preserve_values() {
    let errors = elaborate(&generic_halves_source(false), FieldId::Goldilocks);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

/// `--generic-builtins` compiles the field-generic halves on bn254 too, where they pass the same
/// checks; without it bn254's own `Hash for u64` writes one element, so the limb check fails.
/// What the mode leaves behind is at most an unused private helper of a dropped bn254 half
/// (`bn254::wrapping_mul128` of `WrappingMul for u128`): a stdlib warning `check_crate` never shows.
#[test]
fn bn254_takes_the_generic_halves_under_generic_builtins() {
    let source = generic_halves_source(true);
    let options = frontend_options(FieldId::Bn254, true);
    let (warnings, errors): (Vec<_>, Vec<_>) =
        diagnostics_with(&source, options).into_iter().partition(|(is_warning, _)| *is_warning);
    let errors: Vec<_> = errors.into_iter().map(|(_, rendered)| rendered).collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    let warnings: Vec<_> = warnings.into_iter().map(|(_, rendered)| rendered).collect();
    assert!(
        warnings
            .iter()
            .all(|warning| warning.starts_with("std/") && warning.contains("unused function")),
        "the option leaves at most unused private helpers behind in the stdlib: {}",
        warnings.join("\n")
    );

    // The interpreter locates a failed `assert` at its condition.
    let limb_check = source.find("limbs.len == 2").expect("the limb check is in the source");
    let errors = elaborate(&source, FieldId::Bn254);
    assert!(
        errors.len() == 1
            && errors[0].starts_with(&format!("main.nr @ {limb_check}: Assertion failed")),
        "bn254 hashes a u64 in one write without the option: {}",
        errors.join("\n")
    );
}

/// `nargo --generic-builtins` reaches the frontend through `CompileOptions`: `check_crate`
/// accepts the generic halves' checks under bn254 with the option and refuses them without it.
#[test]
fn the_compile_option_reaches_the_frontend() {
    let source = generic_halves_source(true);
    let with_option =
        CompileOptions { field: FieldId::Bn254, generic_builtins: true, ..Default::default() };
    let (context, crate_id) = context_with_stdlib(&source);
    let (_, result) =
        check_crate_with_comptime_io(context, crate_id, &with_option, &mut tracking_io());
    let ((), warnings) = result
        .unwrap_or_else(|errors| panic!("the generic halves pass under the option: {errors:?}"));
    assert!(warnings.is_empty(), "the stdlib's warning is not the root crate's: {warnings:?}");

    let without_option = CompileOptions { field: FieldId::Bn254, ..Default::default() };
    let (context, crate_id) = context_with_stdlib(&source);
    let (_, result) =
        check_crate_with_comptime_io(context, crate_id, &without_option, &mut tracking_io());
    let errors = result.expect_err("bn254's own `Hash for u64` writes one element");
    assert!(errors.iter().any(|error| error.message.contains("Assertion failed")), "{errors:?}");
}

/// The option acts on bn254's halves only. Under another field a `not(<field>)` gate excludes
/// an item the field's range cannot hold (`From<u64>` for `Field` under Goldilocks), which no
/// benchmark may bring back, so the standard library keeps compiling under every field.
#[test]
fn generic_builtins_leave_the_stdlib_compiling_under_every_field() {
    for field in FieldId::ALL {
        let options = frontend_options(field, true);
        let errors: Vec<_> = diagnostics_with("fn main() {}", options)
            .into_iter()
            .filter_map(|(is_warning, rendered)| (!is_warning).then_some(rendered))
            .collect();
        assert!(errors.is_empty(), "{field}: {}", errors.join("\n"));
    }
}

#[test]
fn bn254_crypto_is_gated_by_the_configured_field() {
    let source = "fn main() { let _ = std::hash::pedersen_hash([1]); }";
    for field in FieldId::ALL {
        let errors = elaborate(source, field);
        if field == FieldId::Bn254 {
            assert!(errors.is_empty(), "{}", errors.join("\n"));
        } else {
            assert!(!errors.is_empty(), "pedersen_hash should not resolve under {field}");
        }
    }
}

/// `UnsafeFrom` wraps: into `Field` modulo the field, a signed source by its bit pattern, and out
/// of `Field` to the low bits of the target.
#[test]
fn unsafe_from_reduces_and_truncates_at_the_boundaries() {
    use num_bigint::BigUint;

    let u64_max = BigUint::from(u64::MAX);
    let u256_max = (BigUint::from(1u8) << 256u32) - 1u8;
    for field in FieldId::ALL {
        let modulus = FieldConfig::new(field).modulus();
        let source = format!(
            "
            use std::convert::UnsafeFrom;

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
            }}
            ",
            &u64_max % modulus,
            &u256_max % modulus,
            modulus - 1u8,
            modulus - 1u8,
        );
        let errors = elaborate(&source, field);
        assert!(errors.is_empty(), "{field}: {}", errors.join("\n"));
    }
}

/// The conversions can be named in full, with no import, in run-time and compile-time code.
#[test]
fn qualified_unsafe_from_path_resolves_in_runtime_and_comptime() {
    let source = "
        fn widen(x: u64) -> Field {
            <Field as std::convert::UnsafeFrom<u64>>::unsafe_from(x)
        }

        fn narrow(x: Field) -> u8 {
            <u8 as std::convert::UnsafeFrom<Field>>::unsafe_from(x)
        }

        fn main(x: u64) -> pub u8 {
            comptime {
                assert(widen(3) == 3);
                assert(narrow(259) == 3);
            }
            narrow(widen(x))
        }
    ";
    for field in FieldId::ALL {
        let errors = elaborate(source, field);
        assert!(errors.is_empty(), "{field}: {}", errors.join("\n"));
    }
}

/// `TryFrom` is `Some` exactly when the value fits, and then holds the value: into `Field`
/// below the modulus, out of `Field` within the target's range.
#[test]
fn try_from_is_some_exactly_when_the_value_fits() {
    use num_bigint::BigUint;

    let u64_max = BigUint::from(u64::MAX);
    for field in FieldId::ALL {
        let modulus = FieldConfig::new(field).modulus();
        let top = modulus - 1u8;
        let source = format!(
            "
            use std::convert::TryFrom;

            fn main() {{
                comptime {{
                    let wide: u64 = {u64_max};
                    assert(Field::try_from(wide).is_some() == {});
                    let below: u256 = {top};
                    assert(Field::try_from(below) == Option::some({top}));
                    let at: u256 = {modulus};
                    assert(Field::try_from(at).is_none());
                    let negative: i8 = -1;
                    assert(Field::try_from(negative).is_none());
                    let positive: i8 = 127;
                    assert(Field::try_from(positive) == Option::some(127));

                    let byte: Field = 255;
                    assert(u8::try_from(byte) == Option::some(255));
                    let over: Field = 256;
                    assert(u8::try_from(over).is_none());
                    let top: Field = {top};
                    assert(u256::try_from(top) == Option::some({top}));
                    let most: Field = 127;
                    assert(i8::try_from(most) == Option::some(127));
                    let past: Field = 128;
                    assert(i8::try_from(past).is_none());
                }}
            }}
            ",
            &u64_max < modulus,
        );
        let errors = elaborate(&source, field);
        assert!(errors.is_empty(), "{field}: {}", errors.join("\n"));
    }
}

/// Where `From` converts every value of the source, the denial of a cast into `Field` names it
/// rather than the conversions for a value that may not fit. Goldilocks has no `From<u64>`.
#[test]
fn the_denial_of_a_cast_into_field_suggests_from_where_every_value_fits() {
    for field in FieldId::ALL {
        let suggests_from = |typ: &str| {
            let source = format!(
                "fn widen(x: {typ}) -> Field {{ x as Field }}
                fn main() {{ let _ = widen(0); }}"
            );
            let errors = elaborate(&source, field);
            assert_eq!(errors.len(), 1, "{field} {typ}: {}", errors.join("\n"));
            errors[0].contains(&format!("Use `Field::from`, which converts every `{typ}` exactly"))
        };
        let wide_fits = field != FieldId::Goldilocks;
        for (typ, expected) in [
            ("u8", true),
            ("u32", true),
            ("u64", wide_fits),
            ("u128", wide_fits),
            ("u256", false),
            ("i8", false),
        ] {
            assert_eq!(suggests_from(typ), expected, "{field} {typ}");
        }
    }
}

/// `Expr::resolve` type checks a quoted expression inside whatever function it is given. A cast
/// is denied wherever it is resolved, a standard library function included.
#[test]
fn a_cast_resolved_inside_the_stdlib_is_denied_where_it_is_spliced() {
    let source = "
        comptime fn launder() -> Quoted {
            let inside = quote { std::mem::zeroed }
                .as_expr()
                .unwrap()
                .resolve(Option::none())
                .as_function_definition()
                .unwrap();
            let cast = quote { 5_u32 as Field }.as_expr().unwrap().resolve(Option::some(inside));
            quote { $cast }
        }

        fn main() {
            let _: Field = launder!();
        }
    ";
    // The same cast resolved in the function that splices it.
    let in_place = source.replace("Option::some(inside)", "Option::none()");
    for field in FieldId::ALL {
        for source in [source, in_place.as_str()] {
            let errors = elaborate(source, field);
            let denials =
                errors.iter().filter(|error| error.contains("Cannot cast `u32` as `Field`"));
            assert_eq!(denials.count(), 1, "{field}: {}", errors.join("\n"));
        }
    }
}
