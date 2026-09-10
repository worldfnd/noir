//! Validate SSA execution within the linked field and its rejection of unsupported wide integers.

use crate::ssa::ssa_gen::generate_ssa;
use acvm::FieldConfig;
use noirc_frontend::test_utils::get_monomorphized;

/// Compile Noir `src` to SSA (structural validation via `validate_ssa`) and interpret it (semantic validation: `assert`s checked over the build's field). `Ok(())` = both passed.
fn compile_and_interpret(src: &str) -> Result<(), String> {
    let program = get_monomorphized(src).map_err(|e| format!("monomorphization: {e:?}"))?;
    let ssa = generate_ssa(program).map_err(|e| format!("ssa gen/validation: {e}"))?;
    ssa.interpret(Vec::new()).map_err(|e| format!("interpret: {e}"))?;
    Ok(())
}

#[test]
fn smoke_trivial_assert() {
    compile_and_interpret("fn main() { assert(2 + 2 == 4); }").unwrap();
}

/// These constants straddle the Goldilocks modulus; SSA cannot represent them faithfully in that field.
#[test]
fn beyond_field_integer_reduced_by_noir_ssa() {
    let src = "fn main() {
        let v1: u64 = 18446744069414584320; // p - 1
        let v2: u64 = 18446744069414584322; // p + 1
        assert(v1 < v2);
    }";
    let result = compile_and_interpret(src);
    if FieldConfig::linked().fits_unsigned(64) {
        result.unwrap();
    } else {
        assert!(
            result.is_err(),
            "expected Noir's SSA to reject the beyond-field u64 constants \
             (reduced mod p, they no longer fit u64); got {result:?}"
        );
    }
}
