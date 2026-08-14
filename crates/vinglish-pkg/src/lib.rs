use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

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
        manifest.dependencies.insert(
            package.to_string(), 
            DependencyMeta::Version("*".to_string())
        );
    }
    
    manifest.save("ving.toml")?;
    
    // Create local stub for now (will be replaced by actual fetcher)
    let target_dir = Path::new(".ving_modules").join(package);
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    
    let dummy_path = target_dir.join(format!("{}.ving", package));
    fs::write(&dummy_path, format!("package {}
module {}

public function hello() returns number
begin
    return 0
end
", package, package)).map_err(|e| e.to_string())?;
    
    println!("Successfully added `{}` to ving.toml", package);
    Ok(())
}
