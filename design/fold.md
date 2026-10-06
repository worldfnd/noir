# Fold

`#[fold]` asks for a constrained function to be compiled as a circuit of its own, called from the main circuit and verified recursively, instead of being inlined into it. The attribute still parses as a known function attribute ([`attributes`](../compiler/noirc_frontend/src/parser/parser/attributes.rs)), and the backend half of the feature, `InlineType::Fold` in the monomorphized AST ([`ast`](../compiler/noirc_frontend/src/monomorphization/ast.rs)) and the separate ACIR functions that SSA and ACIR generation produce for it, stays in place with its own tests.

# Refused by the frontend

Every function carrying `#[fold]` is an elaboration error, `FoldAttributeUnsupported` ([`errors`](../compiler/noirc_frontend/src/hir/resolution/errors.rs)), raised by the `inlining_attributes` lint ([`lints`](../compiler/noirc_frontend/src/elaborator/lints.rs)) whether the function is constrained or unconstrained and whether or not it is called. Because the attribute stays known, an author gets this specific diagnostic rather than an unknown-attribute error. [`runtime`](../compiler/noirc_frontend/src/tests/runtime.rs) pins the rule (`fold_attribute_is_refused_on_constrained_functions`, `fold_attribute_is_refused_on_unconstrained_functions`), and [`fold_attribute_unsupported`](../test_programs/compile_failure/fold_attribute_unsupported/src/main.nr) pins nargo's output.

# Why

The compiler proves one whole circuit at a time. A folded function is a second circuit whose proof the first one verifies, which needs a proving system that composes circuits; Mavros lowers a single circuit and refuses `InlineType::Fold` itself. A program that needs recursion verifies a proof explicitly instead. Refusing the attribute during elaboration reports the problem at its source, with a location, to every tool that elaborates the program, before any backend sees it.
