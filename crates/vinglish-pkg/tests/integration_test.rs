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

    // Resolve an offline package through a test-local registry index.
    let package_source = dir.path().join("test_lib_source");
    fs::create_dir(&package_source).unwrap();
    fs::write(package_source.join("test_lib.ving"), "function answer() returns number\nbegin\n    return 42\nend\n").unwrap();
    let registry = dir.path().join("registry.json");
    fs::write(
        &registry,
        format!(
            r#"{{"test_lib":{{"version":"0.1.0","git":null,"path":"{}"}}}}"#,
            package_source.display()
        ),
    )
    .unwrap();
    unsafe { env::set_var("VINGLISH_REGISTRY_INDEX", &registry) };

    // Test add and fetch.
    assert!(cmd_add("test_lib", None).is_ok());
    
    let manifest = VinglishManifest::load("ving.toml").unwrap();
    assert_eq!(manifest.dependencies.len(), 1);
    assert!(manifest.dependencies.contains_key("test_lib"));
    
    assert!(dir.path().join(".ving_modules/test_lib/test_lib.ving").exists());

    unsafe { env::remove_var("VINGLISH_REGISTRY_INDEX") };
}
