use std::path::Path;

use acvm::FieldId;
use fm::FileManager;
use noirc_driver::{CompileOptions, check_crate, compile_no_check, prepare_crate};
use noirc_errors::CustomDiagnostic;
use noirc_frontend::graph::CrateId;
use noirc_frontend::hir::Context;
use noirc_frontend::hir::def_map::parse_file;

fn context_for(source: &str) -> (Context<'static, 'static>, CrateId) {
    let root = Path::new("");
    let file_name = Path::new("main.nr");
    let mut file_manager = FileManager::new(root);
    file_manager.add_file_with_source(file_name, source.to_owned()).unwrap();
    let parsed_files = file_manager
        .as_file_map()
        .all_file_ids()
        .map(|&file_id| (file_id, parse_file(&file_manager, file_id)))
        .collect();
    let mut context = Context::new(file_manager, parsed_files);
    let crate_id = prepare_crate(&mut context, file_name);
    (context, crate_id)
}

#[test]
fn the_default_field_is_the_one_the_compiler_is_built_with() {
    assert_eq!(CompileOptions::default().field, FieldId::linked());
    let (mut context, crate_id) = context_for("fn main() {}");
    check_crate(&mut context, crate_id, &CompileOptions::default())
        .expect("the built field compiles");
}

#[test]
fn frontend_accepts_a_non_linked_field_and_backend_rejects_it() {
    let requested = FieldId::ALL.into_iter().find(|id| *id != FieldId::linked()).unwrap();
    let options = CompileOptions { field: requested, ..Default::default() };

    let source = "fn main() -> pub Field { comptime { 0 - 1 } }";
    let (mut context, crate_id) = context_for(source);
    check_crate(&mut context, crate_id, &options).expect("the front half runs under any field");

    let main = context.get_main_function(&crate_id).expect("main is defined");
    let error = compile_no_check(&mut context, &options, main, None, false)
        .expect_err("the back half needs the linked field");
    let message = CustomDiagnostic::from(error).message;
    assert!(
        message.contains(requested.name()) && message.contains(FieldId::linked().name()),
        "{message}"
    );
}

#[test]
fn cached_artifacts_cannot_bypass_field_selection() {
    let requested = FieldId::ALL.into_iter().find(|id| *id != FieldId::linked()).unwrap();
    let source = "fn main() -> pub Field { 1 }";

    let (mut context, crate_id) = context_for(source);
    check_crate(&mut context, crate_id, &CompileOptions::default()).expect("the built field");
    let main = context.get_main_function(&crate_id).expect("main is defined");
    let cached = compile_no_check(&mut context, &CompileOptions::default(), main, None, false)
        .expect("the built field compiles");

    let options = CompileOptions { field: requested, ..Default::default() };
    let (mut context, crate_id) = context_for(source);
    check_crate(&mut context, crate_id, &options).expect("the front half runs under any field");
    let main = context.get_main_function(&crate_id).expect("main is defined");
    compile_no_check(&mut context, &options, main, Some(cached), false)
        .expect_err("a cached artifact is not a compilation for another field");
}
