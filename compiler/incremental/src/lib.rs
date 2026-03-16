// KLIK Incremental Compilation System
// Tracks dependency graphs and file hashes for minimal recompilation

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Incremental compilation state
#[derive(Debug, Serialize, Deserialize)]
pub struct IncrementalState {
    /// Map from file path to its content hash
    pub file_hashes: HashMap<PathBuf, String>,
    /// Module dependency graph: module -> dependencies
    pub dependencies: HashMap<String, Vec<String>>,
    /// Cached IR for each module
    pub cached_modules: HashMap<String, CachedModule>,
    /// Build timestamp
    pub last_build: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedModule {
    pub name: String,
    pub hash: String,
    pub ir_hash: String,
}

impl IncrementalState {
    pub fn new() -> Self {
        Self {
            file_hashes: HashMap::new(),
            dependencies: HashMap::new(),
            cached_modules: HashMap::new(),
            last_build: 0,
        }
    }

    /// Load cached state from disk
    pub fn load(cache_dir: &Path) -> Option<Self> {
        let state_file = cache_dir.join("incremental.json");
        if state_file.exists() {
            let data = fs::read_to_string(&state_file).ok()?;
            serde_json::from_str(&data).ok()
        } else {
            None
        }
    }

    /// Save state to disk
    pub fn save(&self, cache_dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(cache_dir)?;
        let state_file = cache_dir.join("incremental.json");
        let data = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(state_file, data)
    }

    /// Check which files have changed since last build
    pub fn changed_files(&self, source_dir: &Path) -> Vec<PathBuf> {
        let mut changed = Vec::new();

        let walker = walkdir::WalkDir::new(source_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "klik")
                    .unwrap_or(false)
            });

        for entry in walker {
            let path = entry.path().to_path_buf();
            let current_hash = hash_file(&path);

            match self.file_hashes.get(&path) {
                Some(cached_hash) if cached_hash == &current_hash => {
                    // File unchanged
                }
                _ => {
                    changed.push(path);
                }
            }
        }

        changed
    }

    /// Update hash for a file
    pub fn update_file_hash(&mut self, path: PathBuf, hash: String) {
        self.file_hashes.insert(path, hash);
    }

    /// Get set of modules that need recompilation
    pub fn modules_to_recompile(&self, changed_files: &[PathBuf]) -> Vec<String> {
        let mut to_recompile: Vec<String> = Vec::new();

        for path in changed_files {
            let module_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            to_recompile.push(module_name.clone());

            // Add reverse dependencies
            self.add_reverse_deps(&module_name, &mut to_recompile);
        }

        to_recompile.sort();
        to_recompile.dedup();
        to_recompile
    }

    fn add_reverse_deps(&self, module: &str, result: &mut Vec<String>) {
        for (name, deps) in &self.dependencies {
            if deps.iter().any(|d| d == module) && !result.contains(name) {
                result.push(name.clone());
                self.add_reverse_deps(name, result);
            }
        }
    }

    /// Register a module dependency
    pub fn add_dependency(&mut self, module: String, depends_on: String) {
        self.dependencies
            .entry(module)
            .or_default()
            .push(depends_on);
    }

    /// Mark a module as cached
    pub fn cache_module(&mut self, name: String, hash: String, ir_hash: String) {
        self.cached_modules.insert(
            name.clone(),
            CachedModule {
                name,
                hash,
                ir_hash,
            },
        );
    }

    /// Check if a module is still valid in cache
    pub fn is_cached(&self, module_name: &str, current_hash: &str) -> bool {
        self.cached_modules
            .get(module_name)
            .map(|c| c.hash == current_hash)
            .unwrap_or(false)
    }
}

impl Default for IncrementalState {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash the contents of a file
pub fn hash_file(path: &Path) -> String {
    match fs::read(path) {
        Ok(contents) => hash_bytes(&contents),
        Err(_) => String::new(),
    }
}

/// Hash arbitrary bytes
pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Hash source code string
pub fn hash_source(source: &str) -> String {
    hash_bytes(source.as_bytes())
}
