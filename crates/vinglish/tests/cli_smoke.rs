use std::path::Path;

use assert_cmd::Command;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn vng_help_is_available() {
    Command::cargo_bin("vng")
        .unwrap()
        .current_dir(repo_root())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn hello_example_passes_public_check() {
    Command::cargo_bin("vng")
        .unwrap()
        .current_dir(repo_root())
        .args(["check", "examples/basics/hello.ving"])
        .assert()
        .success();
}
