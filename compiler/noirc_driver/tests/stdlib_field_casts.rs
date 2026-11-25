use std::path::Path;

use acvm::{FieldConfig, FieldId};
use noirc_driver::{CompileOptions, check_crate, file_manager_with_stdlib, prepare_crate};
use noirc_frontend::elaborator::{FrontendOptions, UnstableFeature};
use noirc_frontend::hir::comptime::ComptimeIo;
use noirc_frontend::hir::def_collector::dc_crate::CompilationError;
use noirc_frontend::hir::def_map::{CrateDefMap, parse_file};
use noirc_frontend::hir::type_check::TypeCheckError;
use noirc_frontend::hir::{CheckState, Context, ParsedFiles};
use noirc_frontend::monomorphization::monomorphize;

/// The frontend options these tests elaborate under: `field`, with enums enabled.
fn frontend_options(field: FieldId, generic_builtins: bool) -> FrontendOptions<'static> {
    FrontendOptions {
        debug_comptime_in_file: None,
        enabled_unstable_features: &[UnstableFeature::Enums],
        disable_required_unstable_features: true,
        field: FieldConfig::new(field),
        generic_builtins,
    }
}

/// Collects and elaborates `crate_id`, returning the errors found.
fn collect_defs(
    crate_id: noirc_frontend::graph::CrateId,
    context: &mut Context,
    options: FrontendOptions,
) -> Vec<CompilationError> {
    let (mut comptime_io, mut state) = (ComptimeIo::stdout(), CheckState::default());
    CrateDefMap::collect_defs(crate_id, context, options, &mut comptime_io, &mut state)
}

/// The standard library writes no cast between `Field` and an integer, under any field and
/// whichever halves of its gated items are compiled: it converts as every other crate does.
#[test]
fn stdlib_never_casts_between_field_and_an_integer() {
    for field in FieldId::ALL {
        for generic_builtins in [false, true] {
            let root = Path::new("");
            let file_name = Path::new("main.nr");
            let mut file_manager = file_manager_with_stdlib(root);
            file_manager.add_file_with_source(file_name, "fn main() {}".to_owned()).unwrap();
            let parsed_files: ParsedFiles = file_manager
                .as_file_map()
                .all_file_ids()
                .map(|&file_id| (file_id, parse_file(&file_manager, file_id)))
                .collect();

            let mut context = Context::new(file_manager, parsed_files);
            let crate_id = prepare_crate(&mut context, file_name);
            let options = frontend_options(field, generic_builtins);
            let errors = collect_defs(crate_id, &mut context, options);

            let offenders: Vec<_> = errors
                .iter()
                .filter_map(|error| match error {
                    CompilationError::TypeError(
                        inner @ TypeCheckError::FieldCastDenied { location, .. },
                    ) => {
                        let path = context.file_manager.path(location.file).unwrap().display();
                        Some(format!("{path} @ {}: {inner}", location.span.start()))
                    }
                    _ => None,
                })
                .collect();
            assert!(
                offenders.is_empty(),
                "{field}, generic_builtins = {generic_builtins}:\n{}",
                offenders.join("\n")
            );
        }
    }
}

/// Compiles `source` against the stdlib and returns the error messages.
fn compile_errors(source: &str) -> Vec<String> {
    compile_errors_with(source, frontend_options(FieldId::linked(), false))
}

/// Like `compile_errors`, under the configured field of `options`.
fn compile_errors_with(source: &str, options: FrontendOptions) -> Vec<String> {
    let root = Path::new("");
    let file_name = Path::new("main.nr");
    let mut file_manager = file_manager_with_stdlib(root);
    file_manager.add_file_with_source(file_name, source.to_owned()).unwrap();
    let parsed_files: ParsedFiles = file_manager
        .as_file_map()
        .all_file_ids()
        .map(|&file_id| (file_id, parse_file(&file_manager, file_id)))
        .collect();
    let mut context = Context::new(file_manager, parsed_files);
    let crate_id = prepare_crate(&mut context, file_name);
    let errors = collect_defs(crate_id, &mut context, options);
    let root_files = context.crate_files(&crate_id);
    errors
        .iter()
        .filter(|error| root_files.contains(&error.location().file))
        .map(|error| error.to_string())
        .collect()
}

/// The stdlib's lossless conversions from an unsigned integer into `Field` exist exactly for the
/// types whose values all lie below the modulus, under every field. They are the same builtin as
/// the wrapping conversion, so this is what keeps one from reducing a value.
#[test]
fn stdlib_conversions_into_field_follow_the_field_width() {
    for field in FieldId::ALL {
        let config = FieldConfig::new(field);
        for bits in [8u32, 16, 32, 64, 128] {
            let source = format!(
                "fn main() {{
                    let x: u{bits} = 1;
                    assert(Field::from(x) == 1);
                }}"
            );
            let options = frontend_options(field, false);
            let errors = compile_errors_with(&source, options);
            if config.fits_unsigned(bits) {
                assert!(errors.is_empty(), "{field}, u{bits}: {errors:?}");
            } else {
                assert!(!errors.is_empty(), "{field}: u{bits} should not convert into Field");
            }
        }
    }
}

/// The stdlib's conversions between `Field` and an integer are the casts themselves: a program
/// that calls them holds the casts and no call, which is what a backend reading the monomorphized
/// program lowers.
#[test]
fn stdlib_conversions_between_field_and_integers_are_casts() {
    let source = "
    use std::convert::UnsafeFrom;

    fn main(x: u32, y: i8) -> pub u8 {
        let sum = Field::unsafe_from(x) + Field::unsafe_from(y) + Field::from(x);
        u8::unsafe_from(sum)
    }";
    let root = Path::new("");
    let file_name = Path::new("main.nr");
    let mut file_manager = file_manager_with_stdlib(root);
    file_manager.add_file_with_source(file_name, source.to_owned()).unwrap();
    let parsed_files: ParsedFiles = file_manager
        .as_file_map()
        .all_file_ids()
        .map(|&file_id| (file_id, parse_file(&file_manager, file_id)))
        .collect();
    let mut context = Context::new(file_manager, parsed_files);
    let crate_id = prepare_crate(&mut context, file_name);
    let (context, result) = check_crate(context, crate_id, &CompileOptions::default());
    result.expect("the program compiles");
    let main = context.get_main_function(&crate_id).expect("main is defined");
    let program = monomorphize(main, &context.def_interner, false).unwrap();

    insta::assert_snapshot!(program, @r"
    fn main$f0(x$l0: u32, y$l1: i8) -> pub u8 {
        let sum$l2 = (((x$l0 as Field) + ((y$l1 as u8) as Field)) + (x$l0 as Field));
        (sum$l2 as u8)
    }
    ");
}

/// `AsPrimitive` converts between the integer types only: across the `Field` boundary the one
/// spelling is `UnsafeFrom`.
#[test]
fn as_primitive_does_not_cross_the_field_boundary() {
    let into_field = "fn main() {
        let x: u8 = 1;
        let _: Field = std::convert::AsPrimitive::as_(x);
    }";
    assert!(!compile_errors(into_field).is_empty(), "u8 should not convert into Field");

    let out_of_field = "fn main() {
        let x: Field = 1;
        let _: u8 = std::convert::AsPrimitive::as_(x);
    }";
    assert!(!compile_errors(out_of_field).is_empty(), "Field should not convert into u8");

    let between_integers = "fn main() {
        let x: u32 = 300;
        let y: u8 = std::convert::AsPrimitive::as_(x);
        assert(y == 44);
    }";
    let errors = compile_errors(between_integers);
    assert!(errors.is_empty(), "{errors:?}");
}
