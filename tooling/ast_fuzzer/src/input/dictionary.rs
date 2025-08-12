use std::collections::BTreeSet;

use noirc_evaluator::ssa::ssa_gen::Ssa;
use num_bigint::BigInt;

/// Collect all `Field` values in the SSA which could be interesting for fuzzing.
pub(crate) fn build_dictionary_from_ssa(ssa: &Ssa) -> BTreeSet<BigInt> {
    let mut constants = BTreeSet::new();
    for func in ssa.functions.values() {
        for (constant, _) in func.constants() {
            constants.insert(constant.clone());
        }
    }
    constants
}
