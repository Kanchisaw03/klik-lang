// KLIK stdlib - IO module

use std::io::{self, BufRead, Write};

/// Print a string to stdout
pub fn print(s: &str) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.flush();
}

/// Print a string to stdout with a newline
pub fn println(s: &str) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.write_all(b"\n");
    let _ = handle.flush();
}

/// Print to stderr
pub fn eprint(s: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.flush();
}

/// Print to stderr with newline
pub fn eprintln(s: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.write_all(b"\n");
    let _ = handle.flush();
}

/// Read a line from stdin
pub fn read_line() -> Result<String, io::Error> {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    Ok(line)
}

/// Read all of stdin
pub fn read_all() -> Result<String, io::Error> {
    let stdin = io::stdin();
    let mut result = String::new();
    for line in stdin.lock().lines() {
        result.push_str(&line?);
        result.push('\n');
    }
    Ok(result)
}
