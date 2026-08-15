use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackageMeta {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DependencyMeta {
    Version(String),
    Detailed {
        version: Option<String>,
        path: Option<String>,
        git: Option<String>,
        branch: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VinglishManifest {
    pub package: PackageMeta,
    #[serde(default)]
    pub dependencies: HashMap<String, DependencyMeta>,
}

impl VinglishManifest {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            package: PackageMeta {
                name: name.to_string(),
                version: version.to_string(),
                description: None,
                authors: None,
            },
            dependencies: HashMap::new(),
        }
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = toml::to_string(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())
    }
}

pub fn cmd_init() -> Result<(), String> {
    println!("Initializing new Vinglish package...");
    let name = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("my_pkg"))
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let name = if name.is_empty() { "my_pkg".to_string() } else { name };
    
    let manifest = VinglishManifest::new(&name, "0.1.0");
    manifest.save("ving.toml")?;
    
    fs::create_dir_all("src").map_err(|e| e.to_string())?;
    fs::write(
        "src/main.ving",
        "function main() returns number
begin
    return 0
end
",
    )
    .map_err(|e| e.to_string())?;
    
    println!("Created package `{}`", name);
    Ok(())
}

pub fn cmd_add(package: &str, url: Option<&str>) -> Result<(), String> {
    println!("Adding package '{}'...", package);
    
    let mut manifest = VinglishManifest::load("ving.toml").map_err(|_| "Failed to read ving.toml. Are you in a Vinglish package?".to_string())?;
    
    if let Some(git_url) = url {
        manifest.dependencies.insert(
            package.to_string(), 
            DependencyMeta::Detailed {
                version: None,
                path: None,
                git: Some(git_url.to_string()),
                branch: None,
            }
        );
    } else {
        // Query the mock registry
        match RegistryClient::query_package(package) {
            Ok(info) => {
                println!("Found '{}' version {} in registry", package, info.version);
                manifest.dependencies.insert(
                    package.to_string(), 
                    DependencyMeta::Detailed {
                        version: Some(info.version),
                        path: info.path,
                        git: info.git,
                        branch: None,
                    }
                );
            }
            Err(e) => {
                return Err(format!("Failed to resolve '{}' in registry: {}", package, e));
            }
        }
    }
    
    manifest.save("ving.toml")?;
    
    // We now just call fetch_dependencies to actually download the package
    fetch_dependencies()?;
    
    println!("Successfully added `{}` to ving.toml", package);
    Ok(())
}

pub struct RegistryClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryResponse {
    pub version: String,
    pub git: Option<String>,
    pub path: Option<String>,
}

impl RegistryClient {
    pub fn query_package(name: &str) -> Result<RegistryResponse, String> {
        // Mock registry implementation for the sandbox environment.
        // In a real environment, this would use reqwest to fetch from registry.vinglish.org
        let mock_registry_file = PathBuf::from("/tmp/mock_vinglish_registry/index.json");
        if !mock_registry_file.exists() {
            return Err("Registry index not found. Are you connected to the internet?".to_string());
        }
        
        let content = fs::read_to_string(mock_registry_file).map_err(|e| e.to_string())?;
        let index: HashMap<String, RegistryResponse> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        
        if let Some(pkg) = index.get(name) {
            Ok(RegistryResponse {
                version: pkg.version.clone(),
                git: pkg.git.clone(),
                path: pkg.path.clone(),
            })
        } else {
            Err(format!("Package '{}' not found in registry", name))
        }
    }
}


use std::process::Command;

pub fn fetch_dependencies() -> Result<(), String> {
    let manifest = match VinglishManifest::load("ving.toml") {
        Ok(m) => m,
        Err(_) => return Ok(()), // Not a package, nothing to fetch
    };

    let modules_dir = Path::new(".ving_modules");
    if !modules_dir.exists() {
        fs::create_dir_all(modules_dir).map_err(|e| e.to_string())?;
    }

    for (name, dep) in manifest.dependencies {
        let target_dir = modules_dir.join(&name);
        if target_dir.exists() {
            // Already fetched
            continue;
        }

        match dep {
            DependencyMeta::Detailed { git: Some(git_url), branch, .. } => {
                println!("Fetching {} from {}...", name, git_url);
                let mut cmd = Command::new("git");
                cmd.arg("clone");
                if let Some(b) = branch {
                    cmd.arg("--branch").arg(b);
                }
                cmd.arg(&git_url).arg(&target_dir);
                
                let status = cmd.status().map_err(|e| format!("Failed to execute git clone: {}", e))?;
                if !status.success() {
                    return Err(format!("Failed to clone repository for {}", name));
                }
            }
            DependencyMeta::Detailed { path: Some(local_path), .. } => {
                println!("Copying local dependency {} from {}...", name, local_path);
                
                // Recursively copy directory
                let mut cmd = Command::new("cp");
                cmd.arg("-r");
                cmd.arg(&local_path);
                cmd.arg(&target_dir);
                
                let status = cmd.status().map_err(|e| format!("Failed to copy local path: {}", e))?;
                if !status.success() {
                    return Err(format!("Failed to copy local path {} for {}", local_path, name));
                }
            }
            _ => {
                // If it's just a version string, we'd normally query a registry. 
                // For now, if we don't have a git url, we'll just skip or error.
                println!("Warning: Registry fetching not yet implemented for {}", name);
            }
        }
    }
    
    Ok(())
}
