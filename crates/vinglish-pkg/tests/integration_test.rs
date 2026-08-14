use vinglish_pkg::{VinglishManifest, cmd_init, cmd_add};
use std::env;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_pkg_init_and_add() {
    let dir = tempdir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    // Test init
    assert!(cmd_init().is_ok());
    assert!(dir.path().join("ving.toml").exists());
    assert!(dir.path().join("src/main.ving").exists());

    let manifest = VinglishManifest::load("ving.toml").unwrap();
    assert_eq!(manifest.dependencies.len(), 0);

    // Test add
    assert!(cmd_add("test_lib", None).is_ok());
    
    let manifest = VinglishManifest::load("ving.toml").unwrap();
    assert_eq!(manifest.dependencies.len(), 1);
    assert!(manifest.dependencies.contains_key("test_lib"));
    
    assert!(dir.path().join(".ving_modules/test_lib/test_lib.ving").exists());
}
