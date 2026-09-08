use std::path::Path;

use acvm::FieldId;
use fm::FileManager;
use noirc_driver::{CompileOptions, check_crate, prepare_crate};
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

// TODO: Remove this rejection test once comptime evaluation supports the configured field.
#[test]
fn a_field_the_compiler_is_not_built_with_is_refused() {
    let requested = FieldId::ALL.into_iter().find(|id| *id != FieldId::linked()).unwrap();
    let (mut context, crate_id) = context_for("fn main() {}");
    let options = CompileOptions { field: requested, ..Default::default() };
    let errors = check_crate(&mut context, crate_id, &options)
        .expect_err("a field the compiler is not built with is refused");
    assert_eq!(errors.len(), 1, "{errors:?}");
    let message = &errors[0].message;
    assert!(
        message.contains(requested.name()) && message.contains(FieldId::linked().name()),
        "{message}"
    );
}
