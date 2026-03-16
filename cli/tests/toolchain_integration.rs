use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn unique_temp_dir() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let dir = std::env::temp_dir().join(format!("klik_cli_it_{}_{}", std::process::id(), stamp));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write_program(dir: &Path, name: &str, src: &str) -> PathBuf {
    let path = dir.join(format!("{}.klik", name));
    fs::write(&path, src).expect("write source");
    path
}

fn run_klik(dir: &Path, args: &[&str]) {
    let output = Command::new(env!("CARGO_BIN_EXE_klik"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run klik");

    if !output.status.success() {
        panic!(
            "klik command failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn run_binary(path: &Path) -> String {
    let output = Command::new(path).output().expect("run binary");
    assert!(
        output.status.success(),
        "binary failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn builds_and_runs_core_program_shapes() {
    let dir = unique_temp_dir();

    let programs = [
        (
            "hello",
            r#"
fn main() {
    println("Hello KLIK")
}
"#,
            "Hello KLIK",
        ),
        (
            "math",
            r#"
fn main() {
    let x = 40 + 2
    println(to_string(x))
}
"#,
            "42",
        ),
        (
            "functions",
            r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let value = add(20, 22)
    println(to_string(value))
}
"#,
            "42",
        ),
        (
            "variables",
            r#"
fn main() {
    let name = "KLIK"
    println(name)
}
"#,
            "KLIK",
        ),
        (
            "strings",
            r#"
fn main() {
    let message = "toolchain"
    println(message)
}
"#,
            "toolchain",
        ),
    ];

    for (name, src, expected) in programs {
        let source_path = write_program(&dir, name, src);
        run_klik(&dir, &["build", source_path.to_string_lossy().as_ref()]);

        let exe = if cfg!(windows) {
            dir.join(format!("{}.exe", name))
        } else {
            dir.join(name)
        };
        assert!(exe.exists(), "expected output binary at {}", exe.display());

        let stdout = run_binary(&exe);
        assert!(
            stdout.contains(expected),
            "expected stdout to contain `{expected}`, got `{stdout}`"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}
