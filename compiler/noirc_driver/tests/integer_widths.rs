//! The front half admits every integer width the language has, inside a program and at an entry
//! point alike; the circuit backend lowers a few. The boundary between them fires on the circuit
//! path only.

use std::path::Path;

use acvm::FieldConfig;
use fm::FileManager;
use noirc_driver::{CompileOptions, check_crate, compile_no_check, prepare_crate};
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
/// The width is that of a helper `main` calls; `main` itself is covered by
/// `an_entry_point_takes_every_width_and_the_circuit_path_refuses_it`.
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

        // The backend carries an integer in one field element, so a lowerable width is lowered
        // only when every value of its type lies below the linked field's modulus.
        let linked = FieldConfig::linked();
        let fits = linked.fits_unsigned(bits);
        let (mut context, crate_id) = context_for(&source);
        check_crate(&mut context, crate_id, &CompileOptions::default()).unwrap();
        let main = context.get_main_function(&crate_id).expect("main is defined");
        let result = compile_no_check(&mut context, &CompileOptions::default(), main, None, false);
        match result {
            Ok(_) => assert!(lowerable && fits, "{typ} reached the backend"),
            Err(error) => {
                let message = CustomDiagnostic::from(error).message;
                assert!(!(lowerable && fits), "{typ}: {message}");
                assert!(message.contains(&format!("uses `{typ}`")), "{message}");
                if lowerable {
                    assert!(message.contains(linked.name()), "{typ}: {message}");
                }
            }
        }
    }
}

/// The byte codec encodes every width the front half admits, and no wider one, and caps an entry
/// point's inputs and return where the monomorphizer does.
#[test]
fn the_byte_codec_and_the_front_half_share_their_limits() {
    assert_eq!(noirc_abi::bytes::MAX_INTEGER_WIDTH, noirc_frontend::shared::MAX_INTEGER_WIDTH);
    assert_eq!(
        noirc_abi::bytes::MAX_INPUT_BYTES,
        noirc_frontend::monomorphization::ast::MAX_INPUT_BYTES
    );
}

/// An entry point takes and returns every width, since its values cross as bytes, so `nargo check`
/// accepts a `main` the circuit backend has no lowering for; `nargo compile` refuses it at the
/// driver's width gate, naming the type.
#[test]
fn an_entry_point_takes_every_width_and_the_circuit_path_refuses_it() {
    let source = "fn main(x: u34) -> pub u34 { x + 1 }";
    let (mut context, crate_id) = context_for(source);
    check_crate(&mut context, crate_id, &CompileOptions::default())
        .expect("the front half admits every width at an entry point");
    let main = context.get_main_function(&crate_id).expect("main is defined");
    let error = compile_no_check(&mut context, &CompileOptions::default(), main, None, false)
        .expect_err("the backend has no lowering for u34");
    let message = CustomDiagnostic::from(error).message;
    assert!(message.contains("uses `u34`"), "{message}");
}

/// A width the backend cannot lower is refused wherever it appears: nested in an array, or in a
/// local that never reaches the signature.
#[test]
fn unlowerable_widths_are_refused_inside_types_and_bodies() {
    let sources = [
        "fn helper(x: [u34; 2]) -> u34 { x[0] } fn main(x: u8) -> pub u8 { helper([x as u34; 2]) as u8 }",
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

/// An alias of unit still means no return, including while building the entry ABI before
/// monomorphization has normalized aliases.
#[test]
fn an_aliased_unit_return_has_no_abi_value() {
    for field in acvm::FieldId::ALL {
        for source in [
            "fn main() {}",
            "fn main() -> pub () {}",
            "type Nothing = (); fn main() -> pub Nothing {}",
            "type Nothing = (); type Other = Nothing; fn main() -> pub Other {}",
        ] {
            let (mut context, crate_id) = context_for(source);
            let options = CompileOptions { field, ..Default::default() };
            check_crate(&mut context, crate_id, &options).unwrap();
            let (_, return_type) = noirc_driver::compute_function_abi(&context, &crate_id).unwrap();
            assert_eq!(return_type, None, "{field}: {source}");
        }
    }
}

/// The checked source signature reaches the byte codec unchanged under either field, preserving
/// declaration order, signed padding, nested values and the widest integer in separate blobs.
#[test]
fn nested_entry_abis_have_identical_bytes_under_every_field() {
    use noirc_abi::{Abi, AbiReturnType, AbiVisibility, bytes, input_parser::Format};

    let source = "
        struct Packet { z: i17, a: bool, text: str<3>, pair: ([u2; 2], (u8, bool)) }
        type Output = (Packet, u16384);
        fn main(packet: pub Packet, wide: u16384) -> pub Output { (packet, wide) }
    ";
    let widest = format!("0x{}", "f".repeat(4096));
    let packet = r#"{"pair": [[0, 3], [255, false]], "text": "abc", "a": true, "z": -1}"#;
    let document = format!(
        "{{\"wide\": \"{widest}\", \"packet\": {packet}, \"return\": [{packet}, \"{widest}\"]}}"
    );
    let mut expected = vec![0xff, 0xff, 0x01, 1, b'a', b'b', b'c', 0, 3, 255, 0];
    expected.extend(vec![0xff; 2048]);

    for field in acvm::FieldId::ALL {
        let (mut context, crate_id) = context_for(source);
        let options = CompileOptions { field, ..Default::default() };
        check_crate(&mut context, crate_id, &options).unwrap();
        let main = context.get_main_function(&crate_id).unwrap();
        monomorphize(main, &mut context.def_interner, context.file_manager.as_file_map(), false)
            .unwrap();
        let (parameters, return_type) =
            noirc_driver::compute_function_abi(&context, &crate_id).unwrap();
        let abi = Abi {
            parameters,
            return_type: return_type
                .map(|abi_type| AbiReturnType { abi_type, visibility: AbiVisibility::Public }),
            error_types: Default::default(),
        };
        assert_eq!(abi.parameters[0].visibility, AbiVisibility::Public);
        assert_eq!(abi.parameters[1].visibility, AbiVisibility::Private);
        let inputs = bytes::parse(&Format::Json, &document, &abi).unwrap();
        for format in [Format::Toml, Format::Json] {
            let serialized = bytes::serialize(&format, &abi, &inputs).unwrap();
            assert_eq!(
                bytes::parse(&format, &serialized, &abi).unwrap(),
                inputs,
                "{field}: {format:?}"
            );
        }
        assert_eq!(bytes::encode_params(&abi, &inputs.params).unwrap(), expected, "{field}");
        assert_eq!(bytes::decode_params(&abi, &expected).unwrap(), inputs.params, "{field}");
        let ret = inputs.ret.unwrap();
        assert_eq!(bytes::encode_return(&abi, &ret).unwrap(), expected, "{field}");
        assert_eq!(bytes::decode_return(&abi, &expected).unwrap(), ret, "{field}");
    }
}
