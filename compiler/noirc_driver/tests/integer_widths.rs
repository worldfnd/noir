//! The front half admits every integer width the language has; the circuit backend lowers a
//! few. The boundary between them fires on the circuit path only.

use std::path::Path;

use acvm::FieldConfig;
use fm::FileManager;
use noirc_abi::{AbiType, Sign};
use noirc_driver::{
    CompileOptions, check_crate, compile_no_check, compute_function_abi, prepare_crate,
};
use noirc_errors::CustomDiagnostic;
use noirc_frontend::graph::CrateId;
use noirc_frontend::hir::Context;
use noirc_frontend::hir::def_map::parse_file;
use noirc_frontend::monomorphization::ast::Type;
use noirc_frontend::monomorphization::monomorphize;
use noirc_frontend::shared::Signedness;

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

/// The widths the circuit backend lowers keep compiling; every other width passes the front
/// half and the monomorphizer, and stops at circuit generation with a message naming the type.
/// The width is that of a helper `main` calls, since `main` itself takes and returns only
/// integers narrower than the field.
#[test]
fn the_front_half_takes_every_width_and_the_circuit_path_stops_at_the_backend() {
    let cases = [
        ("u8", Signedness::Unsigned, 8, true),
        ("u128", Signedness::Unsigned, 128, true),
        ("i64", Signedness::Signed, 64, true),
        ("u3", Signedness::Unsigned, 3, false),
        ("u34", Signedness::Unsigned, 34, false),
        ("i128", Signedness::Signed, 128, false),
        ("u16384", Signedness::Unsigned, 16384, false),
    ];
    for (typ, signedness, bits, lowerable) in cases {
        let source = format!(
            "fn increment(x: {typ}) -> {typ} {{ x + 1 }}
             fn main(x: u8) -> pub u8 {{ increment((x % 2) as {typ}) as u8 }}"
        );

        let (mut context, crate_id) = context_for(&source);
        check_crate(&mut context, crate_id, &CompileOptions::default())
            .expect("the front half admits every width");
        let main = context.get_main_function(&crate_id).expect("main is defined");
        let program = monomorphize(
            main,
            &mut context.def_interner,
            context.file_manager.as_file_map(),
            false,
        )
        .expect("every width monomorphizes");
        let increment =
            program.functions.iter().find(|function| function.name == "increment").unwrap();
        assert_eq!(increment.return_type, Type::Integer(signedness, bits), "{typ}");

        // Arithmetic on a lowerable width wider than the linked field meets the backend's own
        // limits, which are not the width boundary this test is about.
        if lowerable && !FieldConfig::linked().fits_unsigned(bits) {
            continue;
        }
        let (mut context, crate_id) = context_for(&source);
        check_crate(&mut context, crate_id, &CompileOptions::default()).unwrap();
        let main = context.get_main_function(&crate_id).expect("main is defined");
        let result = compile_no_check(&mut context, &CompileOptions::default(), main, None, false);
        match result {
            Ok(_) => assert!(lowerable, "{typ} reached the backend"),
            Err(error) => {
                let message = CustomDiagnostic::from(error).message;
                assert!(!lowerable, "{typ}: {message}");
                assert!(message.contains(&format!("uses `{typ}`")), "{message}");
            }
        }
    }
}

/// A width the backend cannot lower is refused wherever it appears: nested in an array, or in a
/// local that never reaches the signature.
#[test]
fn unlowerable_widths_are_refused_inside_types_and_bodies() {
    let sources = [
        "fn main(x: [u34; 2]) -> pub u34 { x[0] }",
        "fn main(x: u8) -> pub u8 { let wide: u34 = x as u34; (wide + 1) as u8 }",
        "fn helper(x: u34) -> u34 { x } fn main(x: u8) -> pub u8 { helper(x as u34) as u8 }",
    ];
    for source in sources {
        let (mut context, crate_id) = context_for(source);
        check_crate(&mut context, crate_id, &CompileOptions::default()).unwrap();
        let main = context.get_main_function(&crate_id).expect("main is defined");
        let error = compile_no_check(&mut context, &CompileOptions::default(), main, None, false)
            .expect_err("the backend has no lowering for u34");
        let message = CustomDiagnostic::from(error).message;
        assert!(message.contains("`u34`"), "{source}: {message}");
    }
}

/// The widest integer `main` takes is one bit short of the field's modulus.
#[test]
fn the_abi_carries_the_width_as_written() {
    let widest = FieldConfig::linked().num_bits() - 1;
    let source = format!("fn main(x: u34, y: i{widest}) -> pub u34 {{ assert(y == y); x }}");
    let (mut context, crate_id) = context_for(&source);
    check_crate(&mut context, crate_id, &CompileOptions::default()).unwrap();
    let (parameters, return_type) =
        compute_function_abi(&context, &crate_id).expect("main has an abi");
    assert_eq!(parameters[0].typ, AbiType::Integer { sign: Sign::Unsigned, width: 34 });
    assert_eq!(parameters[1].typ, AbiType::Integer { sign: Sign::Signed, width: widest });
    assert_eq!(return_type, Some(AbiType::Integer { sign: Sign::Unsigned, width: 34 }));
}
