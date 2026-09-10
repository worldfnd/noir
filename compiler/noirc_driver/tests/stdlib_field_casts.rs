use std::path::Path;

use noirc_driver::{file_manager_with_stdlib, prepare_crate};
use noirc_frontend::elaborator::FrontendOptions;
use noirc_frontend::hir::Context;
use noirc_frontend::hir::def_collector::dc_crate::CompilationError;
use noirc_frontend::hir::def_map::{CrateDefMap, parse_file};
use noirc_frontend::hir::type_check::TypeCheckError;

/// The `bn254` module of `std::field` is bn254-only by construction (128-bit limbs of the modulus) but is elaborated under every field, since `Field::lt` selects it at run time rather than by a field gate. Its errors under other fields are tolerated.
// TODO: drop once field/bn254.nr is gated and Field::lt is split with #[field(not(bn254))].
const TOLERATED_FILES: [&str; 1] = ["std/field/bn254.nr"];

/// Every stdlib cast to `Field` starts from a type whose values all lie below the modulus of the field the compiler is built for; an impl that casts a wider type is gated to the fields where that type fits.
#[test]
fn stdlib_never_casts_a_wide_integer_to_field() {
    let root = Path::new("");
    let file_name = Path::new("main.nr");
    let mut file_manager = file_manager_with_stdlib(root);
    file_manager.add_file_with_source(file_name, "fn main() {}".to_owned()).unwrap();
    let parsed_files = file_manager
        .as_file_map()
        .all_file_ids()
        .map(|&file_id| (file_id, parse_file(&file_manager, file_id)))
        .collect();

    let mut context = Context::new(file_manager, parsed_files);
    let crate_id = prepare_crate(&mut context, file_name);
    let errors = CrateDefMap::collect_defs(crate_id, &mut context, FrontendOptions::test_default());

    let offenders: Vec<_> = errors
        .iter()
        .filter_map(|error| match error {
            CompilationError::TypeError(
                inner @ TypeCheckError::IntegerTypeExceedsField { location, .. },
            ) => {
                let path = context.file_manager.path(location.file).unwrap().display();
                let path = path.to_string();
                (!TOLERATED_FILES.contains(&path.as_str()))
                    .then(|| format!("{path} @ {}: {inner}", location.span.start()))
            }
            _ => None,
        })
        .collect();
    assert!(offenders.is_empty(), "{}", offenders.join("\n"));
}

/// Compiles `source` against the stdlib and returns the error messages.
fn compile_errors(source: &str) -> Vec<String> {
    let root = Path::new("");
    let file_name = Path::new("main.nr");
    let mut file_manager = file_manager_with_stdlib(root);
    file_manager.add_file_with_source(file_name, source.to_owned()).unwrap();
    let parsed_files = file_manager
        .as_file_map()
        .all_file_ids()
        .map(|&file_id| (file_id, parse_file(&file_manager, file_id)))
        .collect();
    let mut context = Context::new(file_manager, parsed_files);
    let crate_id = prepare_crate(&mut context, file_name);
    let errors = CrateDefMap::collect_defs(crate_id, &mut context, FrontendOptions::test_default());
    let root_files = context.crate_files(&crate_id);
    errors
        .iter()
        .filter(|error| root_files.contains(&error.location().file))
        .map(|error| error.to_string())
        .collect()
}

/// The stdlib's conversions from an unsigned integer into `Field` exist exactly for the types whose values all lie below the modulus.
#[test]
fn stdlib_conversions_into_field_follow_the_field_width() {
    use acvm::FieldConfig;
    for bits in [8u32, 16, 32, 64, 128] {
        let source = format!(
            "fn main() {{
                let x: u{bits} = 1;
                let from: Field = Field::from(x);
                let as_: Field = std::convert::AsPrimitive::as_(x);
                assert(from == as_);
            }}"
        );
        let errors = compile_errors(&source);
        if FieldConfig::linked().fits_unsigned(bits) {
            assert!(errors.is_empty(), "u{bits}: {errors:?}");
        } else {
            assert!(!errors.is_empty(), "u{bits} should have no conversion into Field");
        }
    }
}
