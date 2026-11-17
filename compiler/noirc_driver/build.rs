const GIT_COMMIT: &&str = &"GIT_COMMIT";
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), String> {
    // Only query git if the environment variable isn't set.
    if std::env::var(GIT_COMMIT).is_err() {
        let commit = git(&["rev-parse", "HEAD"])?;
        let dirty = !git(&["status", "-s"])?.is_empty();
        println!("cargo:rustc-env=GIT_COMMIT={commit}");
        println!("cargo:rustc-env=GIT_DIRTY={dirty}");
        // Debug builds only rerun this script when PROFILE changes, not on every source edit.
        if std::env::var("PROFILE").is_ok_and(|profile| profile == "debug") {
            println!("cargo:rerun-if-env-changed=PROFILE");
        }
    }

    let stdlib_src_dir = Path::new("../../noir_stdlib/");
    rerun_if_stdlib_changes(stdlib_src_dir);
    Ok(())
}

fn git(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("error executing 'git {}': {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "command 'git {}' failed: {}: {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
