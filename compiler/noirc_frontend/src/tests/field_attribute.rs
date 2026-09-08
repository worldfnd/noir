//! `CHOSEN_FIELD` is the field the compiler was built for and `FOREIGN_FIELD` is gated out under every build, so each test holds under bn254 and Goldilocks alike.

use crate::hir::def_collector::dc_mod::CHOSEN_FIELD;
use crate::tests::{assert_no_errors, get_program_errors};

/// A modulus that is not any supported field, so `is_native_field` is false everywhere.
const FOREIGN_FIELD: &str = "23";

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
    let src = format!(
        "struct Foo {{}}

        impl Foo {{
            #[field({CHOSEN_FIELD})]
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
    let src = format!(
        "struct Foo {{}}

        #[field({CHOSEN_FIELD})]
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
    let src = format!(
        "trait Value {{
            fn value(self) -> u32;
        }}

        struct Foo {{}}

        #[field({CHOSEN_FIELD})]
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
    let src = format!(
        "#[field({CHOSEN_FIELD})]
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
    let src = format!(
        "#[field({CHOSEN_FIELD})]
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
    let src = format!(
        "#[field({CHOSEN_FIELD})]
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
