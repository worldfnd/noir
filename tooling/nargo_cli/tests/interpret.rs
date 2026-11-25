//! Integration tests for `nargo interpret` and `nargo execute --force-comptime`.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

use assert_fs::prelude::{FileWriteStr, PathChild, PathCreateDir};
use nargo::constants::PKG_FILE;

/// A `Prover.toml` whose array length disagrees with the declared parameter type
/// should surface a clean error, the same way `nargo execute` does, instead of
/// panicking inside the SSA input conversion.
#[test]
fn interpret_with_array_length_mismatch_reports_clean_error() {
    let project_dir = assert_fs::TempDir::new().unwrap();
    project_dir.child("src").create_dir_all().unwrap();
    project_dir
        .child(PKG_FILE)
        .write_str(
            r#"[package]
name = "interpret_length_mismatch"
type = "bin"
authors = []
compiler_version = ">=0.0.0"

[dependencies]
"#,
        )
        .unwrap();
    project_dir
        .child("src/main.nr")
        .write_str("fn main(a: pub [u32; 3]) -> pub u32 { a[0] }\n")
        .unwrap();
    project_dir.child("Prover.toml").write_str("a = [\"1\"]\n").unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("nargo").unwrap();
    cmd.arg("--program-dir").arg(project_dir.path()).arg("interpret");
    cmd.assert().failure().stderr(
        predicate::str::contains("does not match the specified type")
            .and(predicate::str::contains("array length != input length").not())
            .and(predicate::str::contains("application panicked").not()),
    );
}

/// An integer width the SSA has no type for is refused with the diagnostic `nargo compile`
/// gives, before SSA generation is asked to build a value of that type.
#[test]
fn interpret_refuses_an_integer_width_the_backend_cannot_lower() {
    let project_dir = assert_fs::TempDir::new().unwrap();
    project_dir.child("src").create_dir_all().unwrap();
    project_dir
        .child(PKG_FILE)
        .write_str(
            r#"[package]
name = "interpret_unlowerable_width"
type = "bin"
authors = []
compiler_version = ">=0.0.0"

[dependencies]
"#,
        )
        .unwrap();
    project_dir
        .child("src/main.nr")
        .write_str(
            "fn main(x: u8) -> pub bool {
    let wide: u200 = x as u200;
    wide * 3 == wide + wide + wide
}
",
        )
        .unwrap();
    project_dir.child("Prover.toml").write_str("x = 7\n").unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("nargo").unwrap();
    cmd.arg("--program-dir").arg(project_dir.path()).arg("interpret");
    cmd.assert().failure().stderr(
        predicate::str::contains("uses `u200`, which the circuit backend cannot lower")
            .and(predicate::str::contains("application panicked").not()),
    );
}

/// The comptime interpreter evaluates every integer width, so `execute --force-comptime` reads
/// an input of whatever width `main` declares, a signed one from its two's complement bit pattern.
#[test]
fn force_comptime_reads_integer_inputs_of_every_width() {
    let project_dir = assert_fs::TempDir::new().unwrap();
    project_dir.child("src").create_dir_all().unwrap();
    project_dir
        .child(PKG_FILE)
        .write_str(
            r#"[package]
name = "force_comptime_wide_inputs"
type = "bin"
authors = []
compiler_version = ">=0.0.0"

[dependencies]
"#,
        )
        .unwrap();
    project_dir
        .child("src/main.nr")
        .write_str(
            "fn main(wide: u200, narrow: i24, byte: u8) -> pub i24 {
    assert(wide == 1267650600228229401496703205376);
    assert(byte == 255);
    narrow - 1
}
",
        )
        .unwrap();
    project_dir
        .child("Prover.toml")
        .write_str("wide = \"1267650600228229401496703205376\"\nnarrow = \"-3\"\nbyte = \"255\"\n")
        .unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("nargo").unwrap();
    cmd.arg("--program-dir").arg(project_dir.path()).arg("execute").arg("--force-comptime");
    cmd.assert().success().stdout(predicate::str::contains("Circuit output: -4"));
}
