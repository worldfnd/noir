//! Tests for the size limits enforced on entry point parameters and returns during
//! monomorphization:
//! - each parameter, and the return, holds at most `MAX_ELEMENTS` flattened elements, so that
//!   later stages (Brillig array allocation) don't have to cope with sizes that approach
//!   `u32::MAX`;
//! - the parameters of `main` together, and separately its return, take fewer than
//!   `MAX_INPUT_BYTES` (2^32) bytes in the byte encoding, where an integer of width `w` takes
//!   `ceil(w / 8)` bytes, a `bool` one byte and a `str<N>` N bytes.
//!
//! The element limit is checked first. The byte tests reach 2^32 bytes without hitting it through
//! wide integers: a `u16384` takes 2048 bytes, so 2^21 of them take 2^32 bytes.

use crate::hir::Context;
use crate::hir::def_map::ModuleId;
use crate::monomorphization::ast::Program;
use crate::monomorphization::{errors::MonomorphizationError, monomorphize};
use crate::node_interner::FuncId;
use crate::test_utils::{
    get_monomorphized, get_monomorphized_with_stdlib, get_program, stdlib_src,
};
use crate::tests::check_monomorphization_error;

#[test]
fn rejects_large_array_input_to_main() {
    let src = r#"
    fn main(_arr: [u8; 4294967295]) {}
            ^^^^ An input parameter has 4294967295 elements which exceeds the limit of 16777216
    "#;
    check_monomorphization_error(src);
}

#[test]
fn rejects_large_array_input_to_unconstrained_main() {
    let src = r#"
    unconstrained fn main(_arr: [u8; 4294967295]) {}
                          ^^^^ An input parameter has 4294967295 elements which exceeds the limit of 16777216
    "#;
    check_monomorphization_error(src);
}

/// A nested array whose element product overflows `u32` must produce a clean error rather
/// than panicking in the `u32`-based `Type::entry_point_field_count`.
#[test]
fn rejects_nested_array_input_overflowing_u32() {
    // 65536 * 65536 == 2^32 == 4294967296, which overflows u32.
    let src = r#"
    fn main(_arr: [[u8; 65536]; 65536]) {}
            ^^^^ An input parameter has 4294967296 elements which exceeds the limit of 16777216
    "#;
    check_monomorphization_error(src);
}

/// The limit applies to the whole flattened parameter, so an aggregate that individually
/// stays small but sums past the limit is still rejected.
#[test]
fn rejects_tuple_input_containing_large_array() {
    // 4294967295 elements for the array + 1 for the trailing u8.
    let src = r#"
    fn main(_input: ([u8; 4294967295], u8)) {}
            ^^^^^^ An input parameter has 4294967296 elements which exceeds the limit of 16777216
    "#;
    check_monomorphization_error(src);
}

#[test]
fn accepts_array_input_at_the_limit() {
    let src = r#"
    fn main(_arr: [u8; 16777216]) {}
    "#;
    assert!(get_monomorphized(src).is_ok(), "an input of exactly the limit should be accepted");
}

#[test]
fn rejects_array_input_just_over_the_limit() {
    let src = r#"
    fn main(_arr: [u8; 16777217]) {}
            ^^^^ An input parameter has 16777217 elements which exceeds the limit of 16777216
    "#;
    check_monomorphization_error(src);
}

/// The same limit applies to the circuit's output (return value), independent of whether the
/// program is ACIR or Brillig.
#[test]
fn rejects_large_array_return_from_main() {
    let src = r#"
    fn main() -> pub [u8; 4294967295] {
                     ^^^^^^^^^^^^^^^^ The return value has 4294967295 elements which exceeds the limit of 16777216
        [0; 4294967295]
    }
    "#;
    check_monomorphization_error(src);
}

#[test]
fn rejects_large_array_return_from_unconstrained_main() {
    let src = r#"
    unconstrained fn main() -> pub [u8; 4294967295] {
                                   ^^^^^^^^^^^^^^^^ The return value has 4294967295 elements which exceeds the limit of 16777216
        [0; 4294967295]
    }
    "#;
    check_monomorphization_error(src);
}

#[test]
fn accepts_array_return_at_the_limit() {
    let src = r#"
    unconstrained fn main() -> pub [u8; 16777216] {
        [0; 16777216]
    }
    "#;
    assert!(get_monomorphized(src).is_ok(), "a return of exactly the limit should be accepted");
}

/// A parameter over both limits reports the element limit, which is checked first.
#[test]
fn the_element_limit_is_checked_before_the_byte_limit() {
    let src = r#"
    fn main(_arr: [u16384; 16777217]) {}
            ^^^^ An input parameter has 16777217 elements which exceeds the limit of 16777216
    "#;
    check_monomorphization_error(src);
}

/// The parameters of `main` may take 2^32 - 1 bytes together, however the bytes are made up.
#[test]
fn accepts_inputs_one_byte_under_the_byte_limit() {
    for src in [
        // 2097151 * 2048 + 2047 == 2^32 - 1
        "fn main(_a: [u16384; 2097151], _b: [u8; 2047]) {}",
        // A `u16377` takes ceil(16377 / 8) == 2048 bytes, and a `bool` one byte.
        "fn main(_a: [u16377; 2097151], _b: [bool; 2047]) {}",
        // A `str<N>` takes N bytes.
        "fn main(_a: [u16384; 2097151], _b: str<2047>) {}",
        "unconstrained fn main(_a: [u16384; 2097151], _b: ([u8; 2046], u8)) {}",
    ] {
        let result = get_monomorphized(src);
        assert!(result.is_ok(), "`{src}`: {result:?}");
    }
}

/// The byte limit applies to all parameters together: the parameter that brings the total to
/// 2^32 bytes is reported, though each parameter alone stays under the limit.
#[test]
fn rejects_inputs_at_the_byte_limit() {
    let src = r#"
    fn main(_a: [u16384; 2097151], _b: [u8; 2048]) {}
                                   ^^ The input parameters take 4294967296 bytes, which reaches the limit of 4294967296
    "#;
    check_monomorphization_error(src);
}

#[test]
fn accepts_a_return_one_byte_under_the_byte_limit() {
    let src = r#"
    unconstrained fn main() -> pub ([u16384; 2097151], [u8; 2047]) {
        ([0; 2097151], [0; 2047])
    }
    "#;
    let result = get_monomorphized(src);
    assert!(result.is_ok(), "a return of 2^32 - 1 bytes should be accepted: {result:?}");
}

#[test]
fn rejects_a_return_at_the_byte_limit() {
    let src = r#"
    fn main() -> pub [u16384; 2097152] {
                     ^^^^^^^^^^^^^^^^^ The return value takes 4294967296 bytes, which reaches the limit of 4294967296
        [0; 2097152]
    }
    "#;
    check_monomorphization_error(src);
}

/// The parameters and the return are separate byte blobs, each under its own limit.
#[test]
fn inputs_and_return_are_limited_separately() {
    let src = r#"
    unconstrained fn main(_a: [u16384; 2097151], _b: [u8; 2047]) -> pub ([u16384; 2097151], [u8; 2047]) {
        ([0; 2097151], [0; 2047])
    }
    "#;
    let result = get_monomorphized(src);
    assert!(result.is_ok(), "inputs and return of 2^32 - 1 bytes each: {result:?}");
}

/// Monomorphizes the function `select` picks from `src`, as nargo compiles it into a circuit.
fn monomorphize_function(
    src: &str,
    select: impl FnOnce(&Context) -> FuncId,
) -> Result<Program, MonomorphizationError> {
    let (_, mut context, errors) = get_program(src);
    assert!(errors.is_empty(), "`{src}`: {errors:?}");
    let function = select(&context);
    monomorphize(function, &mut context.def_interner, context.file_manager.as_file_map(), false)
}

/// A contract function is an entry point, so it has the byte limits of `main`.
#[test]
fn contract_functions_have_the_byte_limits_of_main() {
    let contract_function = |context: &Context| {
        let root = *context.root_crate_id();
        let (local_id, _) = context.def_map(&root).unwrap().get_all_contracts().next().unwrap();
        context
            .module(ModuleId { krate: root, local_id })
            .value_definitions()
            .find_map(|definition| definition.as_function())
            .unwrap()
    };

    let src = "contract Wallet { pub fn go(_a: [u16384; 2097151], _b: [u8; 2047]) {} }";
    let result = monomorphize_function(src, contract_function);
    assert!(result.is_ok(), "{result:?}");

    let src = "contract Wallet { pub fn go(_a: [u16384; 2097151], _b: [u8; 2048]) {} }";
    let result = monomorphize_function(src, contract_function);
    assert!(
        matches!(
            result,
            Err(MonomorphizationError::InputSizeExceeded { num_bytes: 4294967296, .. })
        ),
        "{result:?}"
    );

    let src = "contract Wallet { pub fn go() -> pub [u16384; 2097152] { [0; 2097152] } }";
    let result = monomorphize_function(src, contract_function);
    assert!(
        matches!(
            result,
            Err(MonomorphizationError::ReturnSizeExceeded { num_bytes: 4294967296, .. })
        ),
        "{result:?}"
    );
}

/// A test function compiled into a circuit is not an entry point of a program and takes no byte
/// blob, so only the element limit applies to it.
#[test]
fn test_functions_have_no_byte_limit() {
    let src = "#[test] fn harness(_a: [u16384; 2097152]) {} fn main() {}";
    let result = monomorphize_function(src, |context| {
        let root = *context.root_crate_id();
        let def_map = context.def_map(&root).unwrap();
        def_map.get_all_test_functions(&context.def_interner).next().unwrap().id
    });
    assert!(result.is_ok(), "{result:?}");
}

/// The element check fires only for the functions monomorphization compiles as entry points of a
/// circuit, and the byte check only for `main` and contract functions. A generic `unconstrained`
/// function called from ACIR becomes a Brillig entry point per instantiation (here
/// `mk_array::<10>` and `mk_array::<4294967295>`), with `N` only known at monomorphization — yet
/// it is a plain `brillig(inline)` function, not flagged as an entry point, so its oversized
/// return is NOT rejected here. It would fail later during codegen.
///
/// `zeroed()` produces the array without materializing a literal, so the test isolates the
/// return-type boundary from the separate interior-allocation gap.
#[test]
fn generic_unconstrained_entry_point_return_is_not_checked() {
    let src = r#"
    unconstrained fn mk_array<let N: u32>() -> [u64; N] {
        zeroed()
    }
    fn main() {
        // Safety: test
        let _a = unsafe { mk_array::<10>() };
        // Safety: test
        let _b = unsafe { mk_array::<4294967295>() };
    }
    "#;
    let result = get_monomorphized_with_stdlib(src, &[stdlib_src::ZEROED]);
    assert!(result.is_ok(), "generic Brillig entry point return is not rejected: {result:?}");
}

/// A non-entry Brillig function returning a huge array is an interior allocation, not a circuit
/// boundary, so it is NOT rejected by this check (it would fail later during Brillig codegen).
#[test]
fn non_entry_brillig_return_is_not_checked() {
    let src = r#"
    unconstrained fn helper() -> [u64; 4294967295] {
        zeroed()
    }
    unconstrained fn main() {
        let _ = helper();
    }
    "#;
    let result = get_monomorphized_with_stdlib(src, &[stdlib_src::ZEROED]);
    assert!(result.is_ok(), "non-entry Brillig return is not rejected: {result:?}");
}
