# Reference counts

Brillig arrays and vectors carry a reference count so that a write can modify the storage in place when nothing else can observe it. The [`ownership`](../compiler/noirc_frontend/src/ownership/mod.rs) pass clones a value in unconstrained code wherever it is shared, roughly where a Rust `Copy` value would be copied, SSA generation lowers each clone to `inc_rc`, and the Brillig [`ArrayCopy`](../compiler/noirc_evaluator/src/brillig/brillig_ir/procedures/array_copy.rs) procedure writes in place only when the count is 1 and copies otherwise. The count serves only this copy-on-write scheme. Constrained code has no counts, and other consumers of the monomorphized AST, such as Mavros, elide reference-count operations wherever they can, so one program yields different counts, or none, depending on the backend and the optimization level.

# Not observable from Noir

No builtin exposes a count: `std::mem` has no `array_refcount` or `vector_refcount`, so a program that names either fails to resolve it at compile time ([`mem_refcount_builtins_unavailable`](../test_programs/compile_failure/mem_refcount_builtins_unavailable/src/main.nr)). A program's meaning therefore does not depend on how a backend manages memory, and every backend accepts the same programs.

The upstream declarations of the two builtins, their comptime evaluation, the `ArrayRefCount` and `VectorRefCount` SSA intrinsics, and the tests that only exercised them are kept commented out under a note that points here, so a rebase onto upstream Noir surfaces changes to them as conflicts rather than restoring them. The upstream `reference_counts_*` programs, which assert exact counts, are kept commented out in `test_programs/fork_excluded`, a directory no test harness scans ([`test_programs/README.md`](../test_programs/README.md)).

# Observing copies

Copies are observed without reading counts. `nargo execute --count-array-copies` reports how many arrays an unconstrained execution copied ([`execute_cmd`](../tooling/nargo_cli/src/cli/execute_cmd.rs)). The [ownership tests](../compiler/noirc_frontend/src/ownership/tests.rs) pin where clones are placed, and the [`rc_invariant`](../compiler/noirc_evaluator/src/ssa/validation/rc_invariant/mod.rs) validator checks that an array modified in place at count 1 is either protected by an `inc_rc` or no longer used afterwards. Neither checks exact counts, so an `inc_rc` that only adds a copy is a performance question, visible through `--count-array-copies`, rather than a correctness one.
