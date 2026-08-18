use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PackageLock {
    pub version: u32,
    pub dependencies: HashMap<String, LockEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LockEntry {
    pub version: String,
    pub source: Option<String>,
    pub checksum: Option<String>,
    #[serde(default)]
    pub requirement: Option<String>,
    #[serde(default)]
    pub integrity: Option<String>,
}

impl PackageLock {
    pub fn validate_against_manifest(&self, manifest: &VinglishManifest) -> Result<(), String> {
        for (name, dep) in &manifest.dependencies {
            let lock_entry = self
                .dependencies
                .get(name)
                .ok_or_else(|| format!("lockfile is missing the entry for `{name}`"))?;

            let requirement = dep.version_requirement();
            if requirement.trim().is_empty() || requirement == "*" {
                continue;
            }

            let version_req = VersionReq::parse(&requirement)
                .map_err(|error| format!("invalid semver requirement for `{name}`: {error}"))?;
            let version = Version::parse(&lock_entry.version)
                .map_err(|error| format!("invalid locked version for `{name}`: {error}"))?;

            if !version_req.matches(&version) {
                return Err(format!(
                    "lockfile version {} for `{name}` does not satisfy requirement {}",
                    lock_entry.version, requirement
                ));
            }
        }

        Ok(())
    }
}

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
        #[serde(default)]
        checksum: Option<String>,
        #[serde(default)]
        integrity: Option<String>,
    },
}

impl DependencyMeta {
    pub fn version_requirement(&self) -> String {
        match self {
            Self::Version(version) => version.clone(),
            Self::Detailed { version, .. } => version.clone().unwrap_or_else(|| "*".to_string()),
        }
    }
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

pub fn write_lockfile(manifest: &VinglishManifest) -> Result<(), String> {
    let lock = PackageLock {
        version: 1,
        dependencies: manifest
            .dependencies
            .iter()
            .filter_map(|(name, dep)| {
                let requirement = dep.version_requirement();
                let (version, source, checksum, integrity) = match dep {
                    DependencyMeta::Version(version) => (
                        version.clone(),
                        None,
                        None,
                        None,
                    ),
                    DependencyMeta::Detailed {
                        version,
                        path,
                        git,
                        branch: _,
                        checksum,
                        integrity,
                    } => (
                        version.clone().unwrap_or_else(|| "0.0.0".to_string()),
                        path.clone().or_else(|| git.clone()),
                        checksum.clone(),
                        integrity.clone(),
                    ),
                };

                Some((
                    name.clone(),
                    LockEntry {
                        version,
                        source,
                        checksum,
                        requirement: Some(requirement),
                        integrity,
                    },
                ))
            })
            .collect(),
    };

    let content = serde_json::to_string_pretty(&lock).map_err(|e| e.to_string())?;
    fs::write("ving.lock", content).map_err(|e| e.to_string())
}

pub fn cmd_init() -> Result<(), String> {
    println!("Initializing new Vinglish package...");
    let name = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("my_pkg"))
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let name = if name.is_empty() {
        "my_pkg".to_string()
    } else {
        name
    };

    let manifest = VinglishManifest::new(&name, "0.1.0");
    manifest.save("ving.toml")?;
    write_lockfile(&manifest)?;

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
                version: Some("*".to_string()),
                path: None,
                git: Some(git_url.to_string()),
                branch: None,
                checksum: None,
                integrity: None,
            },
        );
    } else {
        // Query the mock registry
        match RegistryClient::query_package(package) {
            Ok(info) => {
                println!("Found '{}' version {} in registry", package, info.version);
                manifest.dependencies.insert(
                    package.to_string(),
                    DependencyMeta::Detailed {
                        version: Some(info.version.clone()),
                        path: info.path,
                        git: info.git,
                        branch: None,
                        checksum: info.checksum,
                        integrity: info.integrity,
                    },
                );
            }
            Err(e) => {
                return Err(format!("Failed to resolve '{}' in registry: {}", package, e));
            }
        }
    }

    manifest.save("ving.toml")?;
    write_lockfile(&manifest)?;
    let lock_content = fs::read_to_string("ving.lock").map_err(|e| e.to_string())?;
    let lock: PackageLock = serde_json::from_str(&lock_content).map_err(|e| e.to_string())?;
    lock.validate_against_manifest(&manifest)?;

    // We now just call fetch_dependencies to actually download the package
    fetch_dependencies()?;

    println!("Successfully added `{}` to ving.toml", package);
    Ok(())
}

pub struct RegistryClient;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistryResponse {
    pub version: String,
    pub git: Option<String>,
    pub path: Option<String>,
    #[serde(default)]
    pub checksum: Option<String>,
    #[serde(default)]
    pub integrity: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistryVersionInfo {
    pub version: String,
    pub git: Option<String>,
    pub path: Option<String>,
    #[serde(default)]
    pub checksum: Option<String>,
    #[serde(default)]
    pub integrity: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum RegistryIndexEntry {
    Single(RegistryResponse),
    Versioned {
        latest: String,
        versions: HashMap<String, RegistryVersionInfo>,
    },
}

impl RegistryClient {
    pub fn query_package(name: &str) -> Result<RegistryResponse, String> {
        Self::query_package_with_requirement(name, "*")
    }

    pub fn query_package_with_requirement(name: &str, requirement: &str) -> Result<RegistryResponse, String> {
        if !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(format!("invalid package name `{name}`"));
        }

        if let Some(index_path) = std::env::var_os("VINGLISH_REGISTRY_INDEX") {
            let content = fs::read_to_string(&index_path)
                .map_err(|error| format!("cannot read registry index {:?}: {error}", index_path))?;
            let index: HashMap<String, RegistryIndexEntry> = serde_json::from_str(&content)
                .map_err(|error| format!("invalid registry index: {error}"))?;
            let entry = index
                .get(name)
                .cloned()
                .ok_or_else(|| format!("package `{name}` was not found in the local registry index"))?;
            return match entry {
                RegistryIndexEntry::Single(response) => {
                    let req = parse_requirement(requirement)?;
                    let version = Version::parse(&response.version)
                        .map_err(|error| format!("registry version for `{name}` is invalid: {error}"))?;
                    if req.matches(&version) {
                        Ok(response)
                    } else {
                        Err(format!(
                            "package `{name}` version {} does not satisfy requirement {}",
                            response.version, requirement
                        ))
                    }
                }
                RegistryIndexEntry::Versioned { versions, .. } => {
                    let req = parse_requirement(requirement)?;
                    let mut best_match: Option<RegistryVersionInfo> = None;
                    for info in versions.values() {
                        let version = Version::parse(&info.version)
                            .map_err(|error| format!("registry version for `{name}` is invalid: {error}"))?;
                        if req.matches(&version) {
                            match &best_match {
                                None => best_match = Some(info.clone()),
                                Some(current) => {
                                    let current_version = Version::parse(&current.version)
                                        .map_err(|error| format!("current registry version for `{name}` is invalid: {error}"))?;
                                    if version > current_version {
                                        best_match = Some(info.clone());
                                    }
                                }
                            }
                        }
                    }
                    best_match.ok_or_else(|| format!(
                        "package `{name}` has no registry version that satisfies requirement {}",
                        requirement
                    )).map(|info| RegistryResponse {
                        version: info.version,
                        git: info.git,
                        path: info.path,
                        checksum: info.checksum,
                        integrity: info.integrity,
                    })
                }
            };
        }

        let endpoint = registry_endpoint(name)?;
        let response = reqwest::blocking::Client::new()
            .get(&endpoint)
            .send()
            .map_err(|error| format!("cannot contact Vinglish registry at {endpoint}: {error}"))?
            .error_for_status()
            .map_err(|error| format!("registry request for `{name}` failed: {error}"))?
            .json::<RegistryResponse>()
            .map_err(|error| format!("registry returned an invalid response for `{name}`: {error}"))?;

        let req = parse_requirement(requirement)?;
        let version = Version::parse(&response.version)
            .map_err(|error| format!("registry version for `{name}` is invalid: {error}"))?;
        if req.matches(&version) {
            Ok(response)
        } else {
            Err(format!(
                "package `{name}` version {} does not satisfy requirement {}",
                response.version, requirement
            ))
        }
    }
}

fn parse_requirement(requirement: &str) -> Result<VersionReq, String> {
    let req = requirement.trim();
    if req.is_empty() || req == "*" {
        return Ok(VersionReq::STAR);
    }
    VersionReq::parse(req).map_err(|error| format!("invalid semver requirement `{requirement}`: {error}"))
}

fn registry_endpoint(name: &str) -> Result<String, String> {
    let base = std::env::var("VINGLISH_REGISTRY_URL")
        .unwrap_or_else(|_| "https://registry.vinglish.org/v1".to_string());
    let base = base.trim_end_matches('/');
    if !(base.starts_with("https://") || base.starts_with("http://")) {
        return Err("VINGLISH_REGISTRY_URL must be an http(s) URL".to_string());
    }
    Ok(format!("{base}/packages/{name}"))
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

    for (name, dep) in &manifest.dependencies {
        let target_dir = modules_dir.join(name);
        if target_dir.exists() {
            continue;
        }

        match dep {
            DependencyMeta::Detailed {
                git: Some(git_url),
                branch,
                ..
            } => {
                println!("Fetching {} from {}...", name, git_url);
                let mut cmd = Command::new("git");
                cmd.arg("clone");
                if let Some(b) = branch {
                    cmd.arg("--branch").arg(b);
                }
                cmd.arg(git_url).arg(&target_dir);

                let status = cmd.status().map_err(|e| format!("Failed to execute git clone: {}", e))?;
                if !status.success() {
                    return Err(format!("Failed to clone repository for {}", name));
                }
            }
            DependencyMeta::Detailed {
                path: Some(local_path),
                ..
            } => {
                println!("Copying local dependency {} from {}...", name, local_path);

                let mut cmd = Command::new("cp");
                cmd.arg("-r");
                cmd.arg(local_path);
                cmd.arg(&target_dir);

                let status = cmd.status().map_err(|e| format!("Failed to copy local path: {}", e))?;
                if !status.success() {
                    return Err(format!("Failed to copy local path {} for {}", local_path, name));
                }
            }
            _ => {
                println!("Warning: Registry fetching not yet implemented for {}", name);
            }
        }
    }

    write_lockfile(&manifest)?;
    Ok(())
}
