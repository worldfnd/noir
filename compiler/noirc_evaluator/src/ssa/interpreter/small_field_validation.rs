//! Validate SSA execution within the linked field and its rejection of unsupported wide integers.

use crate::{errors::RuntimeError, ssa::ssa_gen::generate_ssa};
use acvm::{FieldConfig, FieldId};
use noirc_frontend::test_utils::get_monomorphized;

#[test]
fn smoke_trivial_assert() {
    let program = get_monomorphized("fn main() { assert(2 + 2 == 4); }").unwrap();
    generate_ssa(program).unwrap().interpret(Vec::new()).unwrap();
}

/// A literal below the linked modulus is carried exactly, even into a type wider than the field;
/// one at or above it is refused before the bounds of its integer type are checked.
#[test]
fn wide_integer_literals_and_patterns_report_their_boundary_errors() {
    let modulus = FieldConfig::new(FieldId::Goldilocks).modulus().clone();
    for literal in [&modulus - 1u8, modulus.clone(), &modulus + 1u8] {
        for src in [
            format!("fn main() {{ let v: u64 = {literal}; assert(v != 0); }}"),
            format!(
                "fn main() {{ let v: u64 = 0;
                    assert(match v {{ {literal} => false, _ => true, }});
                }}"
            ),
        ] {
            let program = get_monomorphized(&src).unwrap();
            let result = generate_ssa(program);
            if FieldConfig::linked().fits_unsigned(64) || literal < modulus {
                result.unwrap().interpret(Vec::new()).unwrap();
            } else {
                assert!(
                    matches!(result, Err(RuntimeError::IntegerExceedsField { value, .. })
                        if value == literal.to_string()),
                    "{src}"
                );
            }
        }
    }
}
