//! Execution-based validation of the monomorphized AST under different fields.
//!
//! Two validation levels, with different reach:
//!
//! 1. **Mono-AST level (what Mavros consumes).** `get_monomorphized` carries numeric
//!    constants as field-agnostic `BigInt`, so even integers that exceed the field
//!    modulus survive *exactly* in the AST. This is the artifact Mavros lowers natively.
//!
//! 2. **Execution level (Noir's own SSA downstream).** Lowering to SSA runs the always-on
//!    verifier `validate_ssa` (structural) and the SSA interpreter checks `assert`s
//!    (semantic). BUT Noir represents integers as field elements in the circuit:
//!    `bigint_to_field` lowers every integer constant into the build's field, reducing it
//!    mod `p`. So this oracle is faithful only for **in-field** values (`< p`); a machine
//!    integer `>= p` reduces to a field element that no longer fits its integer type, so SSA
//!    generation rejects it (Mavros, which lowers integers natively, is unaffected). See
//!    `beyond_field_integer_reduced_by_noir_ssa`.
#![cfg(test)]

use crate::ssa::ssa_gen::generate_ssa;
use noirc_frontend::test_utils::get_monomorphized;

/// Compile Noir `src` to SSA (structural validation via `validate_ssa`) and interpret it
/// (semantic validation: `assert`s checked over the build's field). `Ok(())` = both passed.
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

/// The mono AST (Mavros's input) carries an integer literal larger than the Goldilocks
/// modulus *exactly*, on any field. This is the Mavros-relevant correctness guarantee.
#[test]
fn mono_ast_carries_beyond_field_integer_exactly() {
    // p + 1 for Goldilocks; a normal in-range u64 for bn254. Either way the AST must hold it.
    let src = "fn main() -> pub u64 { 18446744069414584322 }";
    let program = get_monomorphized(src).unwrap();
    let printed = format!("{program}");
    assert!(
        printed.contains("18446744069414584322"),
        "literal not preserved in mono AST:\n{printed}"
    );
}

/// Decisive probe for whether Noir's *execution* path keeps machine integers native.
/// `v1 = p-1` and `v2 = p+1` are distinct native u64s, but lowering to the Goldilocks field
/// reduces them mod `p` (`p-1` becomes the field element `-1`), which no longer fits `u64`, so
/// SSA generation rejects the program. Documents that Noir's SSA does not preserve beyond-field
/// integers, independent of the mono AST. Mavros is unaffected.
#[test]
fn beyond_field_integer_reduced_by_noir_ssa() {
    let src = "fn main() {
        let v1: u64 = 18446744069414584320; // p - 1
        let v2: u64 = 18446744069414584322; // p + 1
        assert(v1 < v2);
    }";
    let result = compile_and_interpret(src);
    println!("beyond_field_integer_reduced_by_noir_ssa -> {result:?}");
    #[cfg(feature = "goldilocks")]
    assert!(
        result.is_err(),
        "expected Noir's SSA to reject the beyond-field u64 constants \
         (reduced mod p, they no longer fit u64); got {result:?}"
    );
    #[cfg(not(feature = "goldilocks"))]
    result.unwrap();
}
