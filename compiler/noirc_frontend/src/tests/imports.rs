use crate::hir::def_collector::dc_crate::CompilationError;
use crate::hir::def_collector::errors::DefCollectorErrorKind;
use crate::tests::{assert_no_errors, check_errors, get_program_errors};

#[test]
fn use_super() {
    let src = r#"
    fn some_func() {}

    mod foo {
        use super::some_func;

        pub fn bar() {
            some_func();
        }
    }

    fn main() { }
    "#;
    assert_no_errors(src);
}

#[test]
fn no_super() {
    let src = "
    use super::some_func;
        ^^^^^ There is no super module
    ";
    check_errors(src);
}

#[test]
fn use_super_in_path() {
    let src = r#"
    fn some_func() {}

    mod foo {
        pub fn func() {
            super::some_func();
        }
    }

    fn main() { }
    "#;
    assert_no_errors(src);
}

#[test]
fn use_super_super() {
    let src = r#"
    fn some_func() {}

    mod foo {
        mod bar {
            use super::super::some_func;

            pub fn baz() {
                some_func();
            }
        }
    }

    fn main() { }
    "#;
    assert_no_errors(src);
}

#[test]
fn use_super_super_in_path() {
    let src = r#"
    fn some_func() {}

    mod foo {
        mod bar {
            pub fn func() {
                super::super::some_func();
            }
        }
    }

    fn main() { }
    "#;
    assert_no_errors(src);
}

#[test]
fn no_super_super() {
    // `foo` is only one level deep, so `super::super` walks past the crate root.
    let src = "
    mod foo {
        use super::super::some_func;
            ^^^^^ There is no super module
    }
    ";
    check_errors(src);
}

#[test]
fn can_use_pub_use_item() {
    let src = r#"
    mod foo {
        mod bar {
            pub fn baz() {}
        }

        pub use bar::baz;
    }

    fn main() {
        foo::baz();
    }
    "#;
    assert_no_errors(src);
}

#[test]
fn warns_on_re_export_of_item_with_less_visibility() {
    let src = r#"
    mod foo {
        mod bar {
            pub(crate) fn baz() {}
        }

        pub use bar::baz;
                     ^^^ cannot re-export baz because it has less visibility than this use statement
                     ~~~ consider marking baz as pub
    }

    fn main() {
        foo::baz();
    }
    "#;
    check_errors(src);
}

#[test]
fn errors_if_using_alias_in_import() {
    let src = r#"
    mod foo {
        pub type bar = i32;
    }

    use foo::bar::baz;
             ^^^ bar is a type alias, not a module

    fn main() {
    }
    "#;
    check_errors(src);
}

#[test]
fn errors_if_importing_trait_method() {
    // Like Rust, a trait's methods can't be imported through the trait. The method is still
    // callable via a qualified path (`Trait::method(..)`).
    let src = r#"
    mod foo {
        pub trait Trait {
            fn method(self);
        }
    }

    use foo::Trait::method;
             ^^^^^ Trait is a trait, not a module

    fn main() {
    }
    "#;
    check_errors(src);
}

#[test]
fn private_use_reexports_that_comes_later() {
    let src = r#"
    mod history {
        use crate::note::Note;

        pub fn foo() {
            let _ = Note {};
        }
    }
    mod note {
        mod retrieved_history {
            pub struct Note {}
        }
        pub use retrieved_history::Note;
    }

    fn main() {}
    "#;
    assert_no_errors(src);
}

#[test]
fn pub_use_reexports_that_comes_later() {
    let src = r#"
    mod history {
        pub use crate::note::Note;
    }
    mod note {
        mod retrieved_history {
            pub struct Note {}
        }
        pub use retrieved_history::Note;
    }

    fn main() {}
    "#;
    assert_no_errors(src);
}

#[test]
fn invalid_mod_crate_path() {
    let src = r#"
mod crate::mod;
    ^^^^^ Expected an identifier but found 'crate'
         ^^ Expected an item but found '::'
              ^ Expected an identifier but found ';'

fn main() {
}
"#;
    check_errors(src);
}

/// Import errors are reported in the order the imports are written, whatever their locations
/// hash to.
#[test]
fn import_errors_are_reported_in_source_order() {
    let names = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"];
    let functions: String = names.iter().map(|name| format!("pub fn {name}() {{}}\n")).collect();
    let imports = names.join(", ");
    let src = format!(
        "mod first {{ {functions} }}
        mod second {{ {functions} }}
        use first::{{{imports}}};
        use second::{{{imports}}};
        fn main() {{}}"
    );

    let errors = get_program_errors(&src);
    let duplicates: Vec<String> = errors
        .iter()
        .filter_map(|error| match error {
            CompilationError::DefinitionError(DefCollectorErrorKind::Duplicate {
                first_def,
                ..
            }) => Some(first_def.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(duplicates, names);
}
