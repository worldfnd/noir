# Accepted types restriction

- Oracles should be able to accept & return function types even though they are lowered into
field values during SSA. This is undocumented but shouldn't break anything. `println` currently
would break if it weren't allowed to accept functions.

# Modifier restrictions

- An `#[oracle(...)]` function must be `unconstrained` (an oracle is resolved by the external
oracle handler at runtime, which only happens in an unconstrained context).
- An `#[oracle(...)]` function cannot be `comptime`. An oracle is resolved at runtime, whereas a
`comptime` function is evaluated at compile time, so the two are fundamentally incompatible.
Without this restriction a `comptime` oracle could still be called at runtime from Brillig (its
empty body bypasses the usual "comptime functions are only callable at compile time" check),
which contradicts the meaning of `comptime`.

# Only `print` is supported

A program may call two kinds of oracle: the standard library's `print` oracle, which `std::print` and `std::println` call, and the oracles of the `__debug` crate the debugger instruments a program with. Monomorphization refuses any other oracle the first time it reaches one, whether the program calls it or uses it as a value, with `UnsupportedOracle` ([`errors`](../compiler/noirc_frontend/src/monomorphization/errors.rs)). The comptime interpreter refuses calls to the same oracles with the same diagnostic; an oracle function value used only during comptime evaluation remains allowed if it is never called and does not appear in runtime code. The rule applies to reaching an oracle, not to declaring one: a declaration still elaborates and the restrictions above still apply to it, so a library that declares an oracle only for its own tests, as `noir-lang/poseidon` and `noir-lang/sha256` do, still compiles as a dependency. Both passes recognise the debugger's oracles by the crate that defines them, not by their `__debug` name prefix, so a user oracle cannot borrow the exemption. Every backend therefore receives a program whose only external calls are prints and debugger events, and a backend with no oracle handler, such as Mavros, never meets another. The standard library offers nothing built on other oracles, such as a mocking API for tests, for the same reason.

[`oracles`](../compiler/noirc_frontend/src/tests/oracles.rs) pins the rule (`errors_if_non_print_oracle_is_called`, `errors_if_non_print_oracle_is_used_as_value`, `errors_if_oracle_is_reached_through_a_generic_wrapper`, `errors_if_user_oracle_uses_the_debugger_prefix`, `errors_if_user_oracle_with_the_debugger_prefix_is_evaluated_at_comptime`, `errors_if_non_print_oracle_is_evaluated_at_comptime`, `does_not_error_if_unused_oracle_is_declared`, `println_still_compiles`), and [`oracle_unsupported`](../test_programs/compile_failure/oracle_unsupported/src/main.nr) pins nargo's output.
