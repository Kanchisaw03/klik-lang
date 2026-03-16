// KLIK stdlib - File system module

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Read entire file to string
pub fn read_to_string(path: &str) -> Result<String, io::Error> {
    fs::read_to_string(path)
}

/// Read entire file to bytes
pub fn read_bytes(path: &str) -> Result<Vec<u8>, io::Error> {
    fs::read(path)
}

/// Write string to file (creates or overwrites)
pub fn write_string(path: &str, content: &str) -> Result<(), io::Error> {
    fs::write(path, content)
}

/// Write bytes to file (creates or overwrites)
pub fn write_bytes(path: &str, content: &[u8]) -> Result<(), io::Error> {
    fs::write(path, content)
}

/// Append string to file
pub fn append_string(path: &str, content: &str) -> Result<(), io::Error> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(content.as_bytes())
}

/// Check if path exists
pub fn exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// Check if path is a file
pub fn is_file(path: &str) -> bool {
    Path::new(path).is_file()
}

/// Check if path is a directory
pub fn is_dir(path: &str) -> bool {
    Path::new(path).is_dir()
}

/// Create a directory and all parent directories
pub fn create_dir(path: &str) -> Result<(), io::Error> {
    fs::create_dir_all(path)
}

/// Remove a file
pub fn remove_file(path: &str) -> Result<(), io::Error> {
    fs::remove_file(path)
}

/// Remove a directory and all contents
pub fn remove_dir(path: &str) -> Result<(), io::Error> {
    fs::remove_dir_all(path)
}

/// Rename/move a file or directory
pub fn rename(from: &str, to: &str) -> Result<(), io::Error> {
    fs::rename(from, to)
}

/// Copy a file
pub fn copy_file(from: &str, to: &str) -> Result<u64, io::Error> {
    fs::copy(from, to)
}

/// List entries in a directory
pub fn list_dir(path: &str) -> Result<Vec<String>, io::Error> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            entries.push(name.to_string());
        }
    }
    Ok(entries)
}

/// Get file size in bytes
pub fn file_size(path: &str) -> Result<u64, io::Error> {
    let meta = fs::metadata(path)?;
    Ok(meta.len())
}

/// Get the absolute path
pub fn absolute_path(path: &str) -> Result<String, io::Error> {
    let abs = fs::canonicalize(path)?;
    Ok(abs.to_string_lossy().to_string())
}

/// Get path extension
pub fn extension(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(String::from)
}

/// Get file name from path
pub fn file_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
}

/// Get parent directory
pub fn parent(path: &str) -> Option<String> {
    Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .map(String::from)
}

/// Join path components
pub fn join(base: &str, child: &str) -> String {
    let mut path = PathBuf::from(base);
    path.push(child);
    path.to_string_lossy().to_string()
}

/// Get current working directory
pub fn cwd() -> Result<String, io::Error> {
    let path = std::env::current_dir()?;
    Ok(path.to_string_lossy().to_string())
}
