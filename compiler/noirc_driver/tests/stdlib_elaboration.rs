use std::path::Path;

use acvm::{FieldConfig, FieldId};
use noirc_driver::{file_manager_with_stdlib, prepare_crate};
use noirc_errors::CustomDiagnostic;
use noirc_frontend::elaborator::FrontendOptions;
use noirc_frontend::hir::Context;
use noirc_frontend::hir::comptime::EvaluationTracker;
use noirc_frontend::hir::def_map::{CrateDefMap, parse_file};

fn elaborate(source: &str, field: FieldId) -> Vec<String> {
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
    let options =
        FrontendOptions { field: FieldConfig::new(field), ..FrontendOptions::test_default() };
    let errors = CrateDefMap::collect_defs(crate_id, &mut context, options);
    errors
        .iter()
        .map(|error| {
            let location = error.location();
            let path = context.file_manager.path(location.file).unwrap().display();
            let diagnostic = CustomDiagnostic::from(error);
            let notes: Vec<_> =
                diagnostic.secondaries.iter().map(|secondary| secondary.message.clone()).collect();
            format!(
                "{path} @ {}: {} {}",
                location.span.start(),
                diagnostic.message,
                notes.join(" ")
            )
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

#[test]
fn goldilocks_stdlib_fallbacks_preserve_values() {
    let source = "
        use std::hash::{Hash, Hasher};
        use std::ops::{WrappingAdd, WrappingSub, WrappingMul};

        struct Limbs { values: [Field; 4], len: u32 }
        impl Hasher for Limbs {
            fn finish(self) -> Field { self.len as Field }
            fn write(&mut self, input: Field) {
                self.values[self.len] = input;
                self.len += 1;
            }
        }

        unconstrained fn check_unconstrained_order() {
            assert(Field::lt(0, -1));
            assert(!Field::lt(-1, 0));
        }

        fn main() {
            comptime {
                assert(!std::compat::is_bn254());
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

                let mut limbs = Limbs { values: [0; 4], len: 0 };
                max64.hash(&mut limbs);
                assert(limbs.len == 2);
                assert(limbs.values == [0xffffffff, 0xffffffff, 0, 0]);
                limbs = Limbs { values: [0; 4], len: 0 };
                0x112233445566778899aabbccddeeff00u128.hash(&mut limbs);
                assert(limbs.len == 4);
                assert(limbs.values == [0xddeeff00, 0x99aabbcc, 0x55667788, 0x11223344]);
                limbs = Limbs { values: [0; 4], len: 0 };
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
            }
        }
    ";
    let errors = elaborate(source, FieldId::Goldilocks);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
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
