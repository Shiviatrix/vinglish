use std::{fs, sync::Mutex};

use tempfile::tempdir;
use vinglish_pkg::{cmd_init, write_lockfile, VinglishManifest};

static TEST_DIR_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn init_creates_manifest_and_lockfile() {
    let _guard = TEST_DIR_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    cmd_init().unwrap();

    assert!(fs::metadata("ving.toml").is_ok());
    assert!(fs::metadata("ving.lock").is_ok());
    let lock = fs::read_to_string("ving.lock").unwrap();
    assert!(lock.contains("\"version\": 1"));

    std::env::set_current_dir(original).unwrap();
}

#[test]
fn lockfile_serializes_manifest_dependencies() {
    let _guard = TEST_DIR_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut manifest = VinglishManifest::new("demo", "0.1.0");
    manifest.dependencies.insert(
        "core".to_string(),
        vinglish_pkg::DependencyMeta::Version("1.2.3".to_string()),
    );

    write_lockfile(&manifest).unwrap();
    let lock = fs::read_to_string("ving.lock").unwrap();
    assert!(lock.contains("\"core\""));
    assert!(lock.contains("\"version\": \"1.2.3\""));

    std::env::set_current_dir(original).unwrap();
}

#[test]
fn registry_index_resolves_known_package() {
    let index = r#"{
      "core": { "version": "0.2.1", "git": "https://github.com/Shiviatrix/vinglish.git", "path": null },
      "std": { "version": "0.2.1", "git": null, "path": "./std" }
    }"#;

    let dir = tempdir().unwrap();
    let index_path = dir.path().join("registry.json");
    fs::write(&index_path, index).unwrap();

    let old = std::env::var_os("VINGLISH_REGISTRY_INDEX");
    unsafe {
        std::env::set_var("VINGLISH_REGISTRY_INDEX", &index_path);
    }

    let info = vinglish_pkg::RegistryClient::query_package("core").unwrap();
    assert_eq!(info.version, "0.2.1");
    assert_eq!(info.git.as_deref(), Some("https://github.com/Shiviatrix/vinglish.git"));

    match old {
        Some(v) => unsafe { std::env::set_var("VINGLISH_REGISTRY_INDEX", v) },
        None => unsafe { std::env::remove_var("VINGLISH_REGISTRY_INDEX") },
    }
}
