const GIT_COMMIT: &&str = &"GIT_COMMIT";
use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only use build_data if the environment variable isn't set.
    if env::var(GIT_COMMIT).is_err() {
        let vcs_info = Path::new(".cargo_vcs_info.json");
        if vcs_info.is_file() {
            let vcs: serde_json::Value = serde_json::from_str(&fs::read_to_string(vcs_info)?)?;
            let commit = vcs["git"]["sha1"]
                .as_str()
                .ok_or("Missing Git commit in Cargo package metadata")?;
            let dirty = vcs["git"]["dirty"].as_bool().unwrap_or(false);
            println!("cargo:rustc-env=GIT_COMMIT={commit}");
            println!("cargo:rustc-env=GIT_DIRTY={dirty}");
        } else {
            build_data::set_GIT_COMMIT()?;
            build_data::set_GIT_DIRTY()?;
            build_data::no_debug_rebuilds()?;
        }
    }

    // The archive bundles the stdlib. Source checkouts can use the workspace copy.
    let bundled_stdlib = Path::new("noir_stdlib");
    let stdlib_dir = if bundled_stdlib.join("src").is_dir() {
        bundled_stdlib
    } else {
        Path::new("../../noir_stdlib")
    };
    println!("cargo:rustc-env=NOIR_STDLIB_DIR={}", stdlib_dir.canonicalize()?.display());
    rerun_if_stdlib_changes(stdlib_dir);
    Ok(())
}

/// Expects that the given directory is an existing path
fn rerun_if_stdlib_changes(directory: &Path) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();

        if path.is_dir() {
            rerun_if_stdlib_changes(&path);
        } else {
            // Tell Cargo that if the given file changes, to rerun this build script.
            println!("cargo:rerun-if-changed={}", path.to_string_lossy());
        }
    }
}
