# Data bus

`call_data(<id>)` and `return_data` mark an entry point's parameters and return as a data bus, which a backend can read as read-only arrays instead of witnesses. Both modifiers still parse as visibilities ([`function`](../compiler/noirc_frontend/src/parser/parser/function.rs)), `abi_gen` still maps them to `AbiVisibility::DataBus` ([`abi_gen`](../compiler/noirc_driver/src/abi_gen.rs)), and the backend half of the feature, the data-bus arrays SSA builds for them ([`data_bus`](../compiler/noirc_evaluator/src/ssa/function_builder/data_bus.rs)) and their ACIR lowering, stays in place with its own tests.

# Refused by the frontend

`call_data` or `return_data` on any function, whether on a parameter or the return and whether or not the function is an entry point, is an elaboration error, `DataBusUnsupported` ([`errors`](../compiler/noirc_frontend/src/hir/resolution/errors.rs)), raised by the `databus_visibility` lint ([`lints`](../compiler/noirc_frontend/src/elaborator/lints.rs)). Because the modifiers stay known, an author gets this specific diagnostic rather than a parse error, and the fix it names is the plain or `pub` visibility. [`visibility`](../compiler/noirc_frontend/src/tests/visibility.rs) pins the rule, and [`databus_unsupported`](../test_programs/compile_failure/databus_unsupported/src/main.nr) pins nargo's output.

# Why

A program's inputs and return cross the entry point as bytes ([`field-genericity`](field-genericity.md)), and Mavros reads them from that blob and writes each value into its own witness, so the data bus has nothing to group. `return_data` would also satisfy the frontend's check that a returned value is `pub` while Mavros treats it as private, so a verifier would never see the return. Refusing the modifiers during elaboration reports the problem at its source, with a location, to every tool that elaborates the program, before any backend sees it.
