//! Execution-based validation of the monomorphized AST under different fields.
//!
//! Two validation levels, with different reach:
//!
//! 1. **Mono-AST level (what Mavros consumes).** `get_monomorphized` carries numeric
//!    constants as field-agnostic `SignedField`, so even integers that exceed the field
//!    modulus survive *exactly* in the AST. This is the artifact Mavros lowers natively.
//!
//! 2. **Execution level (Noir's own SSA downstream).** Lowering to SSA runs the always-on
//!    verifier `validate_ssa` (structural) and the SSA interpreter checks `assert`s
//!    (semantic). BUT Noir represents integers as field elements in the circuit:
//!    `checked_numeric_constant` reduces every integer constant through `absolute_value()`
//!    before it reaches the interpreter. So this oracle is faithful only for **in-field**
//!    values (`< p`); machine integers `>= p` are reduced by Noir's SSA (Mavros, which
//!    lowers integers natively, is unaffected). See `beyond_field_integer_reduced_by_noir_ssa`.
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
/// `v1 = p-1`, `v2 = p+1`: native u64 has `v1 < v2`; reduced mod Goldilocks p, `v2` becomes
/// `1`, so `v1 < v2` is false. Documents that Noir's SSA reduces beyond-field integers
/// (constant lowered via `absolute_value()`), independent of the mono AST. Mavros is unaffected.
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
        "expected Noir's SSA to reduce p+1 -> 1 (so v1<v2 is false); got {result:?}"
    );
    #[cfg(not(feature = "goldilocks"))]
    result.unwrap();
}
