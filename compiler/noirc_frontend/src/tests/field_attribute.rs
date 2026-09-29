//! `native_field()` is the field the compiler is built with and `FOREIGN_FIELD` matches no configuration, so each test holds under bn254 and Goldilocks alike.

use acvm::{FieldConfig, FieldId};

use crate::elaborator::FrontendOptions;
use crate::hir::def_collector::dc_crate::CompilationError;
use crate::test_utils::{GetProgramOptions, get_program_with_options};
use crate::tests::{assert_no_errors, get_program_errors, get_program_errors_for_field};

/// A modulus that is not any supported field, so it matches no configuration.
const FOREIGN_FIELD: &str = "23";

fn native_field() -> &'static str {
    FieldConfig::linked().name()
}

#[test]
fn gated_out_associated_method_is_not_collected() {
    let src = format!(
        "struct Foo {{}}

        impl Foo {{
            #[field({FOREIGN_FIELD})]
            fn gone() -> u32 {{
                1
            }}
        }}

        fn main() {{
            let _ = Foo::gone();
        }}"
    );
    assert!(!get_program_errors(&src).is_empty(), "expected the gated-out method to be missing");
}

#[test]
fn same_named_methods_gated_to_different_fields_do_not_collide() {
    let native = native_field();
    let src = format!(
        "struct Foo {{}}

        impl Foo {{
            #[field({native})]
            fn value() -> u32 {{
                1
            }}

            #[field({FOREIGN_FIELD})]
            fn value() -> u32 {{
                2
            }}
        }}

        fn main() {{
            assert(Foo::value() == 1);
        }}"
    );
    assert_no_errors(&src);
}

#[test]
fn gated_out_impl_block_is_not_collected() {
    let src = format!(
        "struct Foo {{}}

        #[field({FOREIGN_FIELD})]
        impl Foo {{
            fn gone() -> u32 {{
                1
            }}
        }}

        fn main() {{
            let _ = Foo::gone();
        }}"
    );
    assert!(!get_program_errors(&src).is_empty(), "expected the gated-out impl to be missing");
}

#[test]
fn impl_blocks_gated_to_different_fields_do_not_collide() {
    let native = native_field();
    let src = format!(
        "struct Foo {{}}

        #[field({native})]
        impl Foo {{
            fn value() -> u32 {{
                1
            }}
        }}

        #[field({FOREIGN_FIELD})]
        impl Foo {{
            fn value() -> u32 {{
                2
            }}
        }}

        fn main() {{
            assert(Foo::value() == 1);
        }}"
    );
    assert_no_errors(&src);
}

#[test]
fn gated_out_generated_impl_is_not_collected() {
    let src = format!(
        "#[make]
        struct Foo {{}}

        comptime fn make(_: TypeDefinition) -> Quoted {{
            quote {{ #[field({FOREIGN_FIELD})] impl Foo {{ fn gone() -> u32 {{ 1 }} }} }}
        }}

        fn main() {{
            let _ = Foo::gone();
        }}"
    );
    assert!(
        !get_program_errors(&src).is_empty(),
        "expected the gated-out generated impl to be missing"
    );
}

#[test]
fn trait_impls_gated_to_different_fields_do_not_overlap() {
    let native = native_field();
    let src = format!(
        "trait Value {{
            fn value(self) -> u32;
        }}

        struct Foo {{}}

        #[field({native})]
        impl Value for Foo {{
            fn value(self) -> u32 {{
                1
            }}
        }}

        #[field({FOREIGN_FIELD})]
        impl Value for Foo {{
            fn value(self) -> u32 {{
                2
            }}
        }}

        fn main() {{
            assert(Foo {{}}.value() == 1);
        }}"
    );
    assert_no_errors(&src);
}

#[test]
fn gated_out_trait_impl_is_not_collected() {
    let src = format!(
        "trait Value {{
            fn value(self) -> u32;
        }}

        struct Foo {{}}

        #[field({FOREIGN_FIELD})]
        impl Value for Foo {{
            fn value(self) -> u32 {{
                1
            }}
        }}

        fn main() {{
            let _ = Foo {{}}.value();
        }}"
    );
    assert!(
        !get_program_errors(&src).is_empty(),
        "expected the gated-out trait impl to be missing"
    );
}

#[test]
fn gated_out_generated_trait_impl_is_not_collected() {
    let src = format!(
        "trait Value {{
            fn value(self) -> u32;
        }}

        #[make]
        struct Foo {{}}

        comptime fn make(_: TypeDefinition) -> Quoted {{
            quote {{ #[field({FOREIGN_FIELD})] impl Value for Foo {{ fn value(self) -> u32 {{ 1 }} }} }}
        }}

        fn main() {{
            let _ = Foo {{}}.value();
        }}"
    );
    assert!(
        !get_program_errors(&src).is_empty(),
        "expected the gated-out generated trait impl to be missing"
    );
}

#[test]
fn gated_out_submodule_is_not_collected() {
    let src = format!(
        "#[field({FOREIGN_FIELD})]
        mod gone {{
            pub fn value() -> u32 {{
                1
            }}
        }}

        fn main() {{
            let _ = gone::value();
        }}"
    );
    assert!(!get_program_errors(&src).is_empty(), "expected the gated-out module to be missing");
}

#[test]
fn same_named_submodules_gated_to_different_fields_do_not_collide() {
    let native = native_field();
    let src = format!(
        "#[field({native})]
        mod m {{
            pub fn value() -> u32 {{
                1
            }}
        }}

        #[field({FOREIGN_FIELD})]
        mod m {{
            pub fn value() -> u32 {{
                2
            }}
        }}

        fn main() {{
            assert(m::value() == 1);
        }}"
    );
    assert_no_errors(&src);
}

#[test]
fn gated_out_free_function_is_not_collected() {
    let src = format!(
        "#[field({FOREIGN_FIELD})]
        fn gone() -> u32 {{
            1
        }}

        fn main() {{
            let _ = gone();
        }}"
    );
    assert!(!get_program_errors(&src).is_empty(), "expected the gated-out function to be missing");
}

#[test]
fn same_named_free_functions_gated_to_different_fields_do_not_collide() {
    let native = native_field();
    let src = format!(
        "#[field({native})]
        fn value() -> u32 {{
            1
        }}

        #[field({FOREIGN_FIELD})]
        fn value() -> u32 {{
            2
        }}

        fn main() {{
            assert(value() == 1);
        }}"
    );
    assert_no_errors(&src);
}

#[test]
fn gated_out_global_is_not_collected() {
    let src = format!(
        "#[field({FOREIGN_FIELD})]
        global GONE: u32 = 1;

        fn main() {{
            let _ = GONE;
        }}"
    );
    assert!(!get_program_errors(&src).is_empty(), "expected the gated-out global to be missing");
}

#[test]
fn same_named_globals_gated_to_different_fields_do_not_collide() {
    let native = native_field();
    let src = format!(
        "#[field({native})]
        global VALUE: u32 = 1;

        #[field({FOREIGN_FIELD})]
        global VALUE: u32 = 2;

        fn main() {{
            assert(VALUE == 1);
        }}"
    );
    assert_no_errors(&src);
}

#[test]
fn gated_out_module_declaration_needs_no_file() {
    let src = format!(
        "#[field({FOREIGN_FIELD})]
        mod gone;

        fn main() {{}}"
    );
    assert_no_errors(&src);
}

/// The gate compares against the configured field, so one binary gates the same item in and out.
#[test]
fn one_build_gates_by_the_configured_field() {
    let src = "#[field(goldilocks)]
        fn value() -> u32 {
            1
        }

        fn main() {
            let _ = value();
        }";
    let under_goldilocks = get_program_errors_for_field(src, FieldId::Goldilocks);
    assert!(under_goldilocks.is_empty(), "gated in under Goldilocks: {under_goldilocks:?}");
    assert!(
        !get_program_errors_for_field(src, FieldId::Bn254).is_empty(),
        "expected the item to be gated out under bn254"
    );
}

#[test]
fn field_gate_and_negation_select_one_definition() {
    let src = "#[field(bn254)]
        fn value() -> u32 {
            1
        }

        #[field(not(bn254))]
        fn value() -> u32 {
            2
        }

        fn main() {
            comptime { assert(value() == VALUE); }
        }";
    let under_bn254 = src.replace("VALUE", "1");
    let errors = get_program_errors_for_field(&under_bn254, FieldId::Bn254);
    assert!(errors.is_empty(), "bn254: {errors:?}");

    let under_goldilocks = src.replace("VALUE", "2");
    let errors = get_program_errors_for_field(&under_goldilocks, FieldId::Goldilocks);
    assert!(errors.is_empty(), "goldilocks: {errors:?}");
}

#[test]
fn gated_out_struct_is_not_collected() {
    let src = format!(
        "#[field({FOREIGN_FIELD})]
        struct Gone {{}}

        fn main() {{
            let _ = Gone {{}};
        }}"
    );
    assert!(!get_program_errors(&src).is_empty(), "expected the gated-out struct to be missing");
}

#[test]
fn gated_out_struct_skips_local_impls() {
    let src = format!(
        "#[field({FOREIGN_FIELD})]
        struct Gone {{}}

        impl Gone {{
            fn value() -> u32 {{
                1
            }}
        }}

        fn main() {{}}"
    );
    let errors = get_program_errors(&src);
    assert!(errors.is_empty(), "expected the impl block to leave with its type: {errors:?}");
}

#[test]
fn same_named_structs_gated_to_different_fields_do_not_collide() {
    let native = native_field();
    let src = format!(
        "#[field({native})]
        struct Foo {{
            value: u32,
        }}

        #[field({FOREIGN_FIELD})]
        struct Foo {{
            other: Field,
        }}

        impl Foo {{
            fn get(self) -> u32 {{ self.value }}
        }}

        fn main() {{
            assert(Foo {{ value: 1 }}.get() == 1);
        }}"
    );
    assert_no_errors(&src);
}

#[test]
fn gated_out_type_does_not_hide_impls_for_another_modules_type() {
    for object_type in ["other::Foo", "Foo"] {
        let import = if object_type == "Foo" { "use other::Foo;" } else { "" };
        let src = format!(
            "#[field({FOREIGN_FIELD})]
            struct Foo {{}}
            mod other {{ pub struct Foo {{}} }}
            {import}
            trait Value {{ fn value(self) -> u32; }}
            impl {object_type} {{ fn new() -> Self {{ Self {{}} }} }}
            impl Value for {object_type} {{ fn value(self) -> u32 {{ 1 }} }}
            fn main() {{ assert(other::Foo::new().value() == 1); }}"
        );
        assert_no_errors(&src);
    }
}

#[test]
fn gated_out_generated_types_are_not_collected() {
    for definition in ["struct Gone { value: Missing }", "enum Gone { Value(Missing) }"] {
        let src = format!(
            "#[make]
            struct Foo {{}}
            comptime fn make(_: TypeDefinition) -> Quoted {{
                quote {{ #[field({FOREIGN_FIELD})] {definition} }}
            }}
            fn main() {{
                let _ = Foo {{}};
            }}"
        );
        assert_no_errors(&src);
    }
}

#[test]
fn gated_out_type_alias_is_not_collected() {
    let native = native_field();
    let src = format!(
        "#[field({FOREIGN_FIELD})]
        type Gone = Missing;

        #[field({native})]
        type Kept = u32;

        fn main() {{
            let _: Kept = 1;
        }}"
    );
    assert_no_errors(&src);
}

#[test]
fn same_named_type_aliases_gated_to_different_fields_do_not_collide() {
    let native = native_field();
    let src = format!(
        "#[field({native})]
        type Alias = u32;

        #[field({FOREIGN_FIELD})]
        type Alias = Field;

        fn main() {{
            let value: Alias = 1;
            assert(value == 1);
        }}"
    );
    assert_no_errors(&src);
}

#[test]
fn gated_out_trait_is_not_collected() {
    let src = format!(
        "#[field({FOREIGN_FIELD})]
        trait Gone {{
            fn value(self) -> u32;
        }}

        fn main() {{
            let _: u32 = 1;
        }}"
    );
    let errors = get_program_errors(&src);
    assert!(errors.is_empty(), "a gated-out trait with no users is quiet: {errors:?}");

    let src = format!(
        "#[field({FOREIGN_FIELD})]
        trait Gone {{
            fn value(self) -> u32;
        }}

        fn use_it<T>(x: T) -> u32
        where
            T: Gone,
        {{
            x.value()
        }}

        fn main() {{}}"
    );
    assert!(!get_program_errors(&src).is_empty(), "expected the gated-out trait to be missing");
}

#[test]
fn unknown_field_name_warns() {
    let src = "#[field(bn245)]
        fn gone() -> u32 {
            1
        }

        fn main() {}";
    let errors = get_program_errors(src);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].to_string().contains("bn245"), "{errors:?}");
}

// --- `--generic-builtins`: the benchmark mode prefers an item's field-generic twin. ---

/// The errors of compiling `src` under the native field with `--generic-builtins`.
fn generic_builtins_errors(src: &str) -> Vec<CompilationError> {
    let options = GetProgramOptions {
        frontend_options: FrontendOptions {
            generic_builtins: true,
            ..FrontendOptions::test_default()
        },
        ..Default::default()
    };
    get_program_with_options(src, options).2
}

/// Each kind of item pairs with a twin of the same kind and name in its module: with the option
/// the `not(native)` half is the one compiled, without it the native half. A method pairs by
/// its type and name, a trait impl by its trait and object type as written, and a twin's own
/// `not(native)` helpers come along.
#[test]
fn generic_builtins_prefer_the_not_native_twin_of_every_item_kind() {
    let native = native_field();
    // The twins, and a `main` body that compiles and passes only with the generic twin.
    let cases = [
        (
            format!(
                "#[field({native})] fn value() -> u32 {{ 1 }}
                 #[field(not({native}))] fn value() -> u32 {{ helper() }}
                 #[field(not({native}))] fn helper() -> u32 {{ 2 }}"
            ),
            "comptime { assert(value() == 2); }",
        ),
        (
            format!(
                "struct Foo {{}}
                 struct Bar {{}}
                 impl Foo {{
                     #[field({native})] fn value() -> u32 {{ 1 }}
                     #[field(not({native}))] fn value() -> u32 {{ 2 }}
                 }}
                 impl Bar {{ #[field({native})] fn value() -> u32 {{ 1 }} }}"
            ),
            "comptime { assert(Foo::value() == 2); assert(Bar::value() == 1); }",
        ),
        (
            format!(
                "struct Foo<T> {{ x: T }}
                 trait Value {{ fn value(self) -> u32; }}
                 #[field({native})] impl<T> Value for Foo<T> {{ fn value(self) -> u32 {{ 1 }} }}
                 #[field(not({native}))] impl<T> Value for Foo<T> {{ fn value(self) -> u32 {{ 2 }} }}"
            ),
            "comptime { assert(Foo { x: 1 }.value() == 2); }",
        ),
        (
            format!(
                "#[field({native})] global VALUE: u32 = 1;
                 #[field(not({native}))] global VALUE: u32 = 2;"
            ),
            "comptime { assert(VALUE == 2); }",
        ),
        (
            format!(
                "#[field({native})] type Word = u32;
                 #[field(not({native}))] type Word = u64;"
            ),
            "let word: Word = 1u64; assert(word == 1);",
        ),
        (
            format!(
                "#[field({native})] enum Kind {{ Narrow(u32) }}
                 #[field(not({native}))] enum Kind {{ Wide(u64) }}"
            ),
            "let _ = Kind::Wide(1);",
        ),
        (
            format!(
                "#[field({native})] trait Value {{ fn value(self) -> u32; }}
                 #[field(not({native}))] trait Value {{ fn value(self) -> u64; }}
                 impl Value for Field {{ fn value(self) -> u64 {{ 2 }} }}"
            ),
            "let one: Field = 1; assert(one.value() == 2);",
        ),
        (
            format!(
                "#[field({native})] mod m {{ pub fn value() -> u32 {{ 1 }} }}
                 #[field(not({native}))] mod m {{ pub fn value() -> u32 {{ 2 }} }}"
            ),
            "comptime { assert(m::value() == 2); }",
        ),
    ];
    for (twins, body) in cases {
        let src = format!("{twins}\nfn main() {{ {body} }}");
        let errors = generic_builtins_errors(&src);
        assert!(errors.is_empty(), "with the option, the generic twin: `{src}`: {errors:?}");
        assert!(
            !get_program_errors(&src).is_empty(),
            "without the option, the native half: `{src}`"
        );
    }
}

#[test]
fn generic_builtins_prefer_the_not_native_twin_of_a_whole_impl_and_of_a_struct() {
    let native = native_field();
    let src = format!(
        "#[field({native})]
        struct Foo {{ x: u32 }}

        #[field(not({native}))]
        struct Foo {{ x: u64 }}

        #[field({native})]
        impl Foo {{ fn value(self) -> u64 {{ self.x as u64 }} }}

        #[field(not({native}))]
        impl Foo {{ fn value(self) -> u64 {{ self.x + 1 }} }}

        fn main() {{
            let foo = Foo {{ x: 1 }};
            let wide: u64 = foo.x;
            assert(wide == 1);
            comptime {{ assert(Foo {{ x: 1 }}.value() == 2); }}
        }}"
    );
    assert!(generic_builtins_errors(&src).is_empty(), "the generic twins are compiled");
    assert!(!get_program_errors(&src).is_empty(), "without the option, `x` is a `u32`");
}

/// An item gated to the native field with no `not(native)` twin in its module stays: the
/// option changes nothing the standard library has only one definition of.
#[test]
fn generic_builtins_keep_an_untwinned_native_item() {
    let native = native_field();
    let src = format!(
        "#[field({native})]
        fn only_native() -> u32 {{ 1 }}

        #[field({native})]
        struct OnlyNative {{ x: u32 }}

        fn main() {{
            let _ = OnlyNative {{ x: only_native() }};
            comptime {{ assert(only_native() == 1); }}
        }}"
    );
    assert!(generic_builtins_errors(&src).is_empty(), "untwinned native items are kept");
}

/// Twins are paired within a module: a `not(native)` item elsewhere does not drop a native one.
#[test]
fn generic_builtins_pair_twins_within_a_module() {
    let native = native_field();
    let src = format!(
        "mod other {{
            #[field(not({native}))]
            pub fn value() -> u32 {{ 2 }}
        }}

        #[field({native})]
        fn value() -> u32 {{ 1 }}

        fn main() {{
            comptime {{
                assert(value() == 1);
                assert(other::value() == 2);
            }}
        }}"
    );
    assert!(generic_builtins_errors(&src).is_empty(), "each module pairs its own twins");
}

/// Gates naming another field are evaluated as always: the option only reads the native field's.
#[test]
fn generic_builtins_leave_gates_on_other_fields_alone() {
    let kept = format!(
        "#[field(not({FOREIGN_FIELD}))]
        fn kept() -> u32 {{ 2 }}

        fn main() {{
            comptime {{ assert(kept() == 2); }}
        }}"
    );
    assert!(generic_builtins_errors(&kept).is_empty(), "a `not(foreign)` item is kept");

    let gone = format!(
        "#[field({FOREIGN_FIELD})]
        fn gone() -> u32 {{ 1 }}

        fn main() {{
            let _ = gone();
        }}"
    );
    assert!(!generic_builtins_errors(&gone).is_empty(), "a `foreign` item is still gated out");

    let twinned = format!(
        "#[field({native})]
        #[field(not({FOREIGN_FIELD}))]
        fn value() -> u32 {{ 1 }}

        #[field(not({native}))]
        fn value() -> u32 {{ 2 }}

        fn main() {{
            comptime {{ assert(value() == 2); }}
        }}",
        native = native_field()
    );
    let errors = generic_builtins_errors(&twinned);
    assert!(
        errors.is_empty(),
        "a native item's other gates still admit it; its twin is compiled: {errors:?}"
    );
}

/// A `not(native)` item that another gate excludes is no twin: the native half stays, whether
/// the item carries the other gate itself or sits in an impl that does.
#[test]
fn generic_builtins_ignore_a_twin_that_another_gate_excludes() {
    let native = native_field();
    let excluded_function = format!(
        "#[field({native})]
        fn value() -> u32 {{ 1 }}

        #[field(not({native}))]
        #[field({FOREIGN_FIELD})]
        fn value() -> u32 {{ 2 }}

        fn main() {{
            comptime {{ assert(value() == 1); }}
        }}"
    );
    let method_in_excluded_impl = format!(
        "struct Foo {{}}

        impl Foo {{
            #[field({native})]
            fn value() -> u32 {{ 1 }}
        }}

        #[field({FOREIGN_FIELD})]
        impl Foo {{
            #[field(not({native}))]
            fn value() -> u32 {{ 2 }}
        }}

        fn main() {{
            comptime {{ assert(Foo::value() == 1); }}
        }}"
    );
    for src in [excluded_function, method_in_excluded_impl] {
        let errors = generic_builtins_errors(&src);
        assert!(errors.is_empty(), "the excluded twin leaves the native half in place: {errors:?}");
    }
}
