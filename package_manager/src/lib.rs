// KLIK Package Manager

use anyhow::{bail, Context, Result};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Package manifest (klik.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package: PackageInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
    #[serde(default)]
    pub dev_dependencies: HashMap<String, DependencySpec>,
    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default = "default_edition")]
    pub edition: String,
    #[serde(default)]
    pub entry: Option<String>,
}

fn default_edition() -> String {
    "2024".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    Simple(String),
    Detailed(DetailedDependency),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedDependency {
    pub version: Option<String>,
    pub path: Option<String>,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    #[serde(default = "default_target")]
    pub target: String,
    #[serde(default)]
    pub opt_level: Option<u8>,
    #[serde(default)]
    pub debug: bool,
}

fn default_target() -> String {
    "native".to_string()
}

/// Lock file for reproducible builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    pub version: u32,
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub source: String,
    #[serde(default)]
    pub checksum: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Resolved dependency
#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub name: String,
    pub version: Version,
    pub source: DepSource,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum DepSource {
    Registry(String),
    Path(PathBuf),
    Git { url: String, reference: String },
}

/// The package manager
pub struct PackageManager {
    root: PathBuf,
    manifest: Option<Manifest>,
}

impl PackageManager {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            manifest: None,
        }
    }

    /// Initialize a new KLIK project
    pub fn init_project(path: &Path, name: &str) -> Result<()> {
        let project_dir = path.join(name);
        fs::create_dir_all(&project_dir).context("Failed to create project directory")?;

        let manifest = Manifest {
            package: PackageInfo {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                authors: Vec::new(),
                description: Some(format!("A KLIK project: {}", name)),
                license: None,
                repository: None,
                keywords: Vec::new(),
                edition: "2024".to_string(),
                entry: Some("src/main.klik".to_string()),
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build: BuildConfig::default(),
        };

        let manifest_str =
            toml::to_string_pretty(&manifest).context("Failed to serialize manifest")?;
        fs::write(project_dir.join("klik.toml"), manifest_str)?;

        // Create source directory
        let src_dir = project_dir.join("src");
        fs::create_dir_all(&src_dir)?;

        // Create main.klik
        let main_content = r#"// Welcome to KLIK!

fn main() {
    println("Hello, KLIK!")
}
"#;
        fs::write(src_dir.join("main.klik"), main_content)?;

        // Create tests directory
        let test_dir = project_dir.join("tests");
        fs::create_dir_all(&test_dir)?;

        // Create .gitignore
        let gitignore = "target/\n*.o\n*.exe\nklik.lock\n";
        fs::write(project_dir.join(".gitignore"), gitignore)?;

        // Create README
        let readme = format!("# {}\n\nA KLIK project.\n\n## Build\n\n```\nklik build\n```\n\n## Run\n\n```\nklik run\n```\n", name);
        fs::write(project_dir.join("README.md"), readme)?;

        Ok(())
    }

    /// Load manifest from klik.toml
    pub fn load_manifest(&mut self) -> Result<&Manifest> {
        let manifest_path = self.root.join("klik.toml");
        let content = fs::read_to_string(&manifest_path)
            .context("Failed to read klik.toml. Are you in a KLIK project?")?;
        let manifest: Manifest = toml::from_str(&content).context("Failed to parse klik.toml")?;
        self.manifest = Some(manifest);
        Ok(self.manifest.as_ref().unwrap())
    }

    /// Get the loaded manifest
    pub fn manifest(&self) -> Option<&Manifest> {
        self.manifest.as_ref()
    }

    /// Resolve all dependencies
    pub fn resolve_dependencies(&self) -> Result<Vec<ResolvedDep>> {
        let manifest = self.manifest.as_ref().context("Manifest not loaded")?;

        let mut resolved = Vec::new();

        for (name, spec) in &manifest.dependencies {
            let dep = self.resolve_single_dep(name, spec)?;
            resolved.push(dep);
        }

        Ok(resolved)
    }

    fn resolve_single_dep(&self, name: &str, spec: &DependencySpec) -> Result<ResolvedDep> {
        match spec {
            DependencySpec::Simple(version_str) => {
                let _req = VersionReq::parse(version_str).context(format!(
                    "Invalid version requirement for {}: {}",
                    name, version_str
                ))?;
                Ok(ResolvedDep {
                    name: name.to_string(),
                    version: Version::parse("0.0.0").unwrap(),
                    source: DepSource::Registry(version_str.clone()),
                    dependencies: Vec::new(),
                })
            }
            DependencySpec::Detailed(detailed) => {
                if let Some(path) = &detailed.path {
                    let dep_path = self.root.join(path);
                    let version = detailed.version.as_deref().unwrap_or("0.0.0");
                    Ok(ResolvedDep {
                        name: name.to_string(),
                        version: Version::parse(version)
                            .context(format!("Invalid version for {}", name))?,
                        source: DepSource::Path(dep_path),
                        dependencies: Vec::new(),
                    })
                } else if let Some(git_url) = &detailed.git {
                    let reference = detailed
                        .tag
                        .clone()
                        .or_else(|| detailed.branch.clone())
                        .unwrap_or_else(|| "main".to_string());
                    Ok(ResolvedDep {
                        name: name.to_string(),
                        version: Version::parse("0.0.0").unwrap(),
                        source: DepSource::Git {
                            url: git_url.clone(),
                            reference,
                        },
                        dependencies: Vec::new(),
                    })
                } else if let Some(version_str) = &detailed.version {
                    let _req = VersionReq::parse(version_str)
                        .context(format!("Invalid version for {}", name))?;
                    Ok(ResolvedDep {
                        name: name.to_string(),
                        version: Version::parse("0.0.0").unwrap(),
                        source: DepSource::Registry(version_str.clone()),
                        dependencies: Vec::new(),
                    })
                } else {
                    bail!("Dependency {} has no version, path, or git source", name);
                }
            }
        }
    }

    /// Generate lock file
    pub fn generate_lockfile(&self, resolved: &[ResolvedDep]) -> Result<()> {
        let lock = LockFile {
            version: 1,
            packages: resolved
                .iter()
                .map(|dep| LockedPackage {
                    name: dep.name.clone(),
                    version: dep.version.to_string(),
                    source: match &dep.source {
                        DepSource::Registry(v) => format!("registry:{}", v),
                        DepSource::Path(p) => format!("path:{}", p.display()),
                        DepSource::Git { url, reference } => {
                            format!("git:{}#{}", url, reference)
                        }
                    },
                    checksum: None,
                    dependencies: dep.dependencies.clone(),
                })
                .collect(),
        };

        let content = toml::to_string_pretty(&lock).context("Failed to serialize lock file")?;
        fs::write(self.root.join("klik.lock"), content)?;
        Ok(())
    }

    /// Load lock file
    pub fn load_lockfile(&self) -> Result<Option<LockFile>> {
        let lock_path = self.root.join("klik.lock");
        if !lock_path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&lock_path)?;
        let lock: LockFile = toml::from_str(&content)?;
        Ok(Some(lock))
    }

    /// Add a dependency to the manifest
    pub fn add_dependency(&mut self, name: &str, version: &str) -> Result<()> {
        let manifest = self.manifest.as_mut().context("Manifest not loaded")?;

        let _req = VersionReq::parse(version)
            .context(format!("Invalid version requirement: {}", version))?;

        manifest.dependencies.insert(
            name.to_string(),
            DependencySpec::Simple(version.to_string()),
        );

        let manifest_str =
            toml::to_string_pretty(manifest).context("Failed to serialize manifest")?;
        fs::write(self.root.join("klik.toml"), manifest_str)?;

        Ok(())
    }

    /// Remove a dependency from the manifest
    pub fn remove_dependency(&mut self, name: &str) -> Result<()> {
        let manifest = self.manifest.as_mut().context("Manifest not loaded")?;

        if manifest.dependencies.remove(name).is_none() {
            bail!("Dependency '{}' not found", name);
        }

        let manifest_str =
            toml::to_string_pretty(manifest).context("Failed to serialize manifest")?;
        fs::write(self.root.join("klik.toml"), manifest_str)?;

        Ok(())
    }

    /// Get build output directory
    pub fn target_dir(&self) -> PathBuf {
        self.root.join("target")
    }

    /// Get source directory
    pub fn src_dir(&self) -> PathBuf {
        self.root.join("src")
    }

    /// Get entry file path
    pub fn entry_file(&self) -> PathBuf {
        if let Some(manifest) = &self.manifest {
            if let Some(entry) = &manifest.package.entry {
                return self.root.join(entry);
            }
        }
        self.root.join("src").join("main.klik")
    }

    /// Find all .klik source files
    pub fn find_sources(&self) -> Result<Vec<PathBuf>> {
        let src_dir = self.src_dir();
        if !src_dir.exists() {
            return Ok(Vec::new());
        }
        let mut sources = Vec::new();
        self.walk_sources(&src_dir, &mut sources)?;
        Ok(sources)
    }

    fn walk_sources(&self, dir: &Path, sources: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.walk_sources(&path, sources)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("klik") {
                sources.push(path);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        let toml_str = r#"
[package]
name = "test-project"
version = "1.0.0"
description = "A test"

[dependencies]
some_lib = "^1.0"

[dependencies.other_lib]
version = "2.0"
features = ["async"]

[build]
target = "native"
opt_level = 2
"#;
        let manifest: Manifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.package.name, "test-project");
        assert_eq!(manifest.package.version, "1.0.0");
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.build.opt_level, Some(2));
    }

    #[test]
    fn test_default_edition() {
        let toml_str = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        let manifest: Manifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.package.edition, "2024");
    }
}
