use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[test]
fn run_ui_tests() {
    let ui_dir = Path::new("tests/ui");
    if !ui_dir.exists() {
        return;
    }

    let mut tests = vec![];
    for entry in WalkDir::new(ui_dir) {
        let entry = entry.unwrap();
        if entry.path().extension().map_or(false, |ext| ext == "ving") {
            tests.push(entry.path().to_path_buf());
        }
    }

    tests.sort();

    for test_path in tests {
        let test_name = test_path.file_stem().unwrap().to_str().unwrap();
        
        let mut cmd = Command::cargo_bin("vng").unwrap();
        cmd.arg("run").arg(&test_path);

        let output = cmd.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n");
        
        // Remove absolute paths from stderr to make it deterministic
        let stderr = stderr.replace(ui_dir.canonicalize().unwrap().to_str().unwrap(), "$DIR");
        // Also replace relative paths if they appear
        let stderr = stderr.replace("tests/ui", "$DIR");

        let combined = if !stderr.is_empty() {
            format!("{}\n{}", stdout, stderr)
        } else {
            stdout.to_string()
        };

        insta::with_settings!({
            snapshot_path => PathBuf::from("snapshots"),
            prepend_module_to_snapshot => false,
        }, {
            insta::assert_snapshot!(test_name, combined);
        });
    }
}
