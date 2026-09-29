use std::path::Path;

use acvm::{FieldConfig, FieldId};
use noirc_driver::{CompileOptions, check_crate, file_manager_with_stdlib, prepare_crate};
use noirc_errors::CustomDiagnostic;
use noirc_frontend::elaborator::FrontendOptions;
use noirc_frontend::graph::CrateId;
use noirc_frontend::hir::Context;
use noirc_frontend::hir::comptime::EvaluationTracker;
use noirc_frontend::hir::def_map::{CrateDefMap, parse_file};

fn elaborate(source: &str, field: FieldId) -> Vec<String> {
    let options =
        FrontendOptions { field: FieldConfig::new(field), ..FrontendOptions::test_default() };
    diagnostics_with(source, options).into_iter().map(|(_, rendered)| rendered).collect()
}

/// A context holding the standard library and `source` as `main.nr`, ready to elaborate.
fn context_with_stdlib(source: &str) -> (Context<'static, 'static>, CrateId) {
    let root = Path::new("");
    let file_name = Path::new("main.nr");
    let mut file_manager = file_manager_with_stdlib(root);
    file_manager.add_file_with_source(file_name, source.to_owned()).unwrap();
    let parsed_files = file_manager
        .as_file_map()
        .all_file_ids()
        .map(|&file_id| (file_id, parse_file(&file_manager, file_id)))
        .collect();
    let mut context = Context::new(file_manager, parsed_files);
    let crate_id = prepare_crate(&mut context, file_name);
    // A tracker makes the interpreter answer `is_unconstrained()` from the calling context
    // rather than always `true`, so constrained branches run under `comptime` as well.
    context.evaluation_tracker = Some(EvaluationTracker::new(Default::default()));
    (context, crate_id)
}

/// Every diagnostic of elaborating `source`, rendered, with whether it is a warning. The
/// standard library's warnings are included, which `check_crate` would drop.
fn diagnostics_with(source: &str, options: FrontendOptions) -> Vec<(bool, String)> {
    let (mut context, crate_id) = context_with_stdlib(source);
    let errors = CrateDefMap::collect_defs(crate_id, &mut context, options);
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
            fn finish(self) -> Field {{ self.len as Field }}
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
                assert({not}std::compat::is_bn254());
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

#[test]
fn goldilocks_stdlib_fallbacks_preserve_values() {
    let errors = elaborate(&generic_halves_source(false), FieldId::Goldilocks);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

/// `--generic-builtins` compiles the field-generic halves on bn254 too, where they pass the same
/// checks; without it bn254's own `Hash for u64` writes one element, so the limb check fails.
/// What the mode leaves behind is at most an unused private helper of a dropped bn254 half
/// (`wrapping_mul128_hlp` of `WrappingMul for u128`): a stdlib warning `check_crate` never shows.
#[test]
fn bn254_takes_the_generic_halves_under_generic_builtins() {
    let source = generic_halves_source(true);
    let options = FrontendOptions {
        field: FieldConfig::new(FieldId::Bn254),
        generic_builtins: true,
        ..FrontendOptions::test_default()
    };
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
    let (mut context, crate_id) = context_with_stdlib(&source);
    let ((), warnings) = check_crate(&mut context, crate_id, &with_option)
        .unwrap_or_else(|errors| panic!("the generic halves pass under the option: {errors:?}"));
    assert!(warnings.is_empty(), "the stdlib's warning is not the root crate's: {warnings:?}");

    let without_option = CompileOptions { field: FieldId::Bn254, ..Default::default() };
    let (mut context, crate_id) = context_with_stdlib(&source);
    let errors = check_crate(&mut context, crate_id, &without_option)
        .expect_err("bn254's own `Hash for u64` writes one element");
    assert!(errors.iter().any(|error| error.message.contains("Assertion failed")), "{errors:?}");
}

/// The option acts on bn254's halves only. Under another field a `not(<field>)` gate excludes
/// an item the field's range cannot hold (`From<u64>` for `Field` under Goldilocks), which no
/// benchmark may bring back, so the standard library keeps compiling under every field.
#[test]
fn generic_builtins_leave_the_stdlib_compiling_under_every_field() {
    for field in FieldId::ALL {
        let options = FrontendOptions {
            field: FieldConfig::new(field),
            generic_builtins: true,
            ..FrontendOptions::test_default()
        };
        let errors: Vec<_> = diagnostics_with("fn main() {}", options)
            .into_iter()
            .filter_map(|(is_warning, rendered)| (!is_warning).then_some(rendered))
            .collect();
        assert!(errors.is_empty(), "{field}: {}", errors.join("\n"));
    }
}

#[test]
fn bn254_crypto_is_gated_by_the_configured_field() {
    let source = "fn main(x: Field) -> pub Field { std::hash::pedersen_hash([x]) }";
    for field in FieldId::ALL {
        let errors = elaborate(source, field);
        if field == FieldId::Bn254 {
            assert!(errors.is_empty(), "{}", errors.join("\n"));
        } else {
            assert!(!errors.is_empty(), "pedersen_hash should not resolve under {field}");
        }
    }
}
