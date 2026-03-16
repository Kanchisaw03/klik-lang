<p align="center">
  <img src="docs/assets/klik-logo.svg" alt="KLIK Logo" width="200" />
</p>

<h1 align="center">KLIK</h1>

<p align="center">
  <strong>A modern, statically-typed programming language with a complete toolchain — built from scratch in Rust.</strong>
</p>

<p align="center">
  <a href="#-quick-start">Quick Start</a> •
  <a href="#-language-tour">Language Tour</a> •
  <a href="#%EF%B8%8F-architecture">Architecture</a> •
  <a href="#-compiler-deep-dive">Compiler Deep Dive</a> •
  <a href="#-toolchain">Toolchain</a> •
  <a href="#-contributing">Contributing</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=flat-square&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/backend-Cranelift-blue?style=flat-square" alt="Cranelift" />
  <img src="https://img.shields.io/badge/targets-7%20platforms-green?style=flat-square" alt="Targets" />
  <img src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-purple?style=flat-square" alt="License" />
  <img src="https://img.shields.io/badge/version-0.1.0-red?style=flat-square" alt="Version" />
</p>

---

KLIK is not just a language — it's a **complete language ecosystem** built from the ground up. It ships with a multi-stage optimizing compiler, language server, formatter, linter, package manager, and standard library. Every component — from the hand-rolled lexer to the Cranelift-powered native code generator — is implemented in Rust across **16 modular crates** in a single workspace.

## ✨ Highlights

| Feature | Details |
|---------|---------|
| **Type System** | Hindley-Milner inference with unification, generics, traits, and algebraic data types |
| **Compilation** | Multi-stage pipeline: Source → Tokens → AST → Semantic IR → SSA IR → Cranelift → Native |
| **Targets** | Native (auto-detect), x86_64 (Linux/Windows/macOS), AArch64 (Linux/macOS), WebAssembly |
| **Optimizer** | Constant folding, dead code elimination, common subexpression elimination, CFG simplification |
| **Toolchain** | CLI, LSP server, formatter, linter (12+ rules), package manager, incremental compilation |
| **Concurrency** | `async`/`await`, `spawn`, channel-based communication |
| **Modern Syntax** | Pipe operator (`|>`), pattern matching, lambdas, ranges, expression-oriented blocks |

---

## 🚀 Quick Start

### Build from Source

```bash
# Clone the repository
git clone https://github.com/klik-lang/klik.git
cd klik

# Build the entire toolchain
cargo build --release
```

### Hello, KLIK!

```bash
# Create a new project
klik new my_project
cd my_project

# Edit src/main.klik (auto-generated)
klik run
```

```klik
fn main() {
    println("Hello, KLIK!")
}
```

### CLI Commands

```bash
klik new <name>           # Create a new project
klik build [--release]    # Compile the project
klik run [file.klik]      # Build and execute
klik check                # Type-check without building
klik fmt [--check]        # Format source files
klik lint [--fix]         # Run static analysis
klik test                 # Run test suites
klik lsp                  # Start the language server
klik add <pkg>            # Add a dependency
klik visualize <file>     # Generate AST/IR/CFG graphs
klik build --target wasm  # Compile to WebAssembly
```

---

## 📖 Language Tour

### Variables & Mutability

```klik
let x = 42                   // immutable by default, type inferred as int
let mut counter = 0          // mutable binding
let name: string = "KLIK"   // explicit type annotation
```

### Functions

```klik
fn fibonacci(n: int) -> int {
    if n <= 1 { n } else { fibonacci(n - 1) + fibonacci(n - 2) }
}

pub async fn fetch_data(url: string) -> string {
    let response = http_get(url).await
    response.body
}
```

### Structs & Methods

```klik
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Point {
        Point { x: x, y: y }
    }

    fn distance(self, other: Point) -> f64 {
        let dx = self.x - other.x
        let dy = self.y - other.y
        (dx * dx + dy * dy) |> sqrt
    }
}
```

### Enums & Pattern Matching

```klik
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Triangle(Point, Point, Point),
}

fn area(shape: Shape) -> f64 {
    match shape {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle(w, h) => w * h,
        Shape::Triangle(a, b, c) => {
            (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)) / 2.0
        },
    }
}
```

### Generics & Traits

```klik
trait Comparable<T> {
    fn compare(self, other: T) -> int
}

fn max<T: Comparable<T>>(a: T, b: T) -> T {
    if a.compare(b) > 0 { a } else { b }
}

enum List<T> {
    Cons(T, List<T>),
    Nil,
}
```

### Pipe Operator & Lambdas

```klik
let result = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    |> map(|x| x * x)
    |> filter(|x| x % 2 == 0)
    |> sum()
```

---

## 🏗️ Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                          KLIK TOOLCHAIN                              │
├──────────┬───────────┬────────────┬───────────┬────────┬────────────┤
│  CLI     │  LSP      │ Formatter  │  Linter   │ PkgMgr │ Visualizer │
│ (clap)   │ (tower)   │            │ (12 rules)│ (toml) │ (graphviz) │
└────┬─────┴─────┬─────┴─────┬──────┴─────┬─────┴────┬───┴────────────┘
     │           │           │            │          │
     ▼           ▼           ▼            ▼          ▼
┌──────────────────────────────────────────────────────────────────────┐
│                     COMPILER PIPELINE                                │
│                                                                      │
│  ┌─────────┐  ┌────────┐  ┌──────┐  ┌──────────┐  ┌──────────────┐ │
│  │  Lexer  │→ │ Parser │→ │ AST  │→ │ Semantic │→ │ Type Checker │ │
│  │ (hand-  │  │ (Pratt │  │(rich │  │ Analyzer │  │   (HM-style  │ │
│  │ rolled) │  │  prec.) │  │ enum)│  │ (scopes) │  │  inference)  │ │
│  └─────────┘  └────────┘  └──────┘  └──────────┘  └──────┬───────┘ │
│                                                           │         │
│  ┌──────────────┐  ┌─────────────┐  ┌────────────────────┐│         │
│  │   Codegen    │← │  Optimizer  │← │     IR Builder     │◄         │
│  │ (Cranelift)  │  │ (4 passes)  │  │   (SSA / Phi)      │          │
│  └──────┬───────┘  └─────────────┘  └────────────────────┘          │
│         │                                                            │
│  ┌──────▼────────────────────────────────────────────────┐          │
│  │            Target Code Generation                      │          │
│  │  x86_64-linux │ x86_64-win │ x86_64-mac │ aarch64-*  │          │
│  │               │  wasm32    │  native    │             │          │
│  └────────────────────────────────────────────────────────┘          │
└──────────────────────────────────────────────────────────────────────┘
     │                                            │
     ▼                                            ▼
┌─────────────┐                         ┌──────────────────┐
│   Runtime   │                         │  Standard Library │
│ • allocator │                         │ • io  • math      │
│ • concurr.  │                         │ • strings • time  │
│ • errors    │                         │ • fs  • net       │
└─────────────┘                         │ • collections     │
                                        └──────────────────┘
```

### Workspace Crate Map

The project is organized as a **Rust workspace with 16 crates**, each with a single responsibility:

```
klik/
├── compiler/
│   ├── lexer/          →  klik-lexer         Hand-rolled tokenizer (668 LOC)
│   ├── parser/         →  klik-parser        Recursive descent + Pratt (1,592 LOC)
│   ├── ast/            →  klik-ast           Rich AST with visitor pattern (665 LOC)
│   ├── semantic/       →  klik-semantic      Multi-pass semantic analysis (536 LOC)
│   ├── type_system/    →  klik-type-system   HM inference + unification (997 LOC)
│   ├── ir/             →  klik-ir            SSA IR with phi nodes (1,645 LOC)
│   ├── opt/            →  klik-opt           Optimization passes (336 LOC)
│   ├── codegen/        →  klik-codegen       Cranelift code generation (1,100 LOC)
│   ├── incremental/    →  klik-incremental   SHA-256 based incremental builds (179 LOC)
│   └── pipeline.rs                           CLI ↔ optimizer glue
├── runtime/            →  klik-runtime       Allocator, concurrency, errors
├── stdlib/             →  klik-stdlib        7 standard library modules
├── cli/                →  klik (binary)      CLI entry point (3,300+ LOC)
├── lsp/                →  klik-lsp           Language Server Protocol (443 LOC)
├── formatter/          →  klik-formatter     AST pretty-printer (845 LOC)
├── linter/             →  klik-linter        Static analysis rules (716 LOC)
├── package_manager/    →  klik-package-mgr   TOML manifest + dep resolution (425 LOC)
├── examples/                                 25 example programs
├── tests/                                    Integration test suite
└── docs/                                     Architecture, spec, guides
```

---

## 🔬 Compiler Deep Dive

### Stage 1: Lexical Analysis

The lexer is a **hand-rolled scanner** — no regex engines or parser generators. It processes source code character-by-character, producing a stream of typed tokens.

**Key capabilities:**
- Nestable block comments (`/* ... /* inner */ ... */`)
- All numeric bases: decimal, hex (`0xFF`), binary (`0b1010`), octal (`0o77`)
- Unicode escape sequences in strings and chars (`\u{1F600}`)
- 30+ keywords, 40+ operators and punctuation symbols
- Full source location tracking (`file:line:column`) for every token

```
Source: "let x = 42 + y"

  ┌─────┐   ┌─────┐   ┌───┐   ┌────┐   ┌───┐   ┌─────┐
  │ Let │ → │Id(x)│ → │ = │ → │ 42 │ → │ + │ → │Id(y)│
  └─────┘   └─────┘   └───┘   └────┘   └───┘   └─────┘
```

### Stage 2: Parsing

A **recursive descent parser** with **Pratt precedence climbing** for expressions. It transforms the token stream into a rich Abstract Syntax Tree.

**Expression precedence (low → high):**

| Level | Operators | Associativity |
|-------|-----------|---------------|
| 1 | `\|>` (pipe) | Left |
| 2 | `\|\|` | Left |
| 3 | `&&` | Left |
| 4 | `==  !=  <  <=  >  >=` | None |
| 5 | `\|` (bitwise) | Left |
| 6 | `^` | Left |
| 7 | `&` | Left |
| 8 | `<<  >>` | Left |
| 9 | `+  -` | Left |
| 10 | `*  /  %` | Left |
| 11 | `-  !  ~  &  *` (unary) | Prefix |
| 12 | `.  ()  []` (postfix) | Left |

**Error recovery**: The parser synchronizes on statement boundaries (`fn`, `let`, `struct`, `}`) to report multiple errors in a single pass.

### Stage 3: Semantic Analysis

A **two-pass semantic analyzer** resolves symbols and validates program structure:

| Pass | Purpose | Checks |
|------|---------|--------|
| **1. Declaration** | Collect all top-level definitions | Register functions, structs, enums, traits |
| **2. Resolution** | Resolve references and validate | Undefined symbols, duplicate definitions, immutable mutations, break/return placement |

**Scope management** uses a nested scope stack:

```
Global Scope
  └── Function Scope ("main")
        ├── Block Scope (if-body)
        │     └── Block Scope (nested)
        └── Loop Scope (for-body)
```

### Stage 4: Type Checking

A **Hindley-Milner-inspired type inference engine** with unification:

```
                ┌──────────────────┐
                │   Type Checker   │
                ├──────────────────┤
 Expressions →  │  1. Generate     │  → Substitution Map
                │     constraints  │
                │  2. Unify type   │  → Resolved Types
                │     variables    │
                │  3. Apply        │  → Typed AST
                │     substitution │
                └──────────────────┘
```

**Supported type features:**

- **Primitives**: `int`, `i8`–`i64`, `u8`–`u64`, `f32`, `f64`, `bool`, `char`, `string`, `void`
- **Compounds**: Arrays `[T]`, Tuples `(T, U)`, Optionals `T?`, References `&T`, Functions `fn(T) -> U`
- **User-defined**: Structs, Enums (algebraic data types), Traits
- **Generics**: Parametric polymorphism with trait bounds (`T: Comparable<T>`)
- **Inference**: Type variables, unification, automatic numeric coercions

### Stage 5: IR Generation

The IR Builder lowers the AST into a **Static Single Assignment (SSA)** intermediate representation with basic blocks and phi nodes:

```klik
fn abs(x: int) -> int {            // KLIK source
    if x < 0 { -x } else { x }
}
```

```
fn abs(x: i64) -> i64:            // Generated SSA IR
  entry:
    %0 = param 0                   // load parameter
    %1 = icmp slt %0, 0            // x < 0
    condbr %1, then_bb, else_bb

  then_bb:
    %2 = neg %0                    // -x
    br merge_bb

  else_bb:
    %3 = copy %0                   // x
    br merge_bb

  merge_bb:
    %4 = phi [then_bb: %2, else_bb: %3]
    ret %4
```

**IR instruction categories:**
- **Constants**: `IConst`, `FConst`, `BoolConst`, `StringConst`
- **Arithmetic**: `BinOp`, `UnaryOp` (all standard math + bitwise)
- **Memory**: `Alloca`, `Load`, `Store`, `GetElementPtr`
- **Control flow**: `Branch`, `CondBranch`, `Switch`, `Return`
- **Functions**: `Call`, `Param`
- **Type ops**: `Cast`, `Phi`

### Stage 6: Optimization

Multi-pass optimization at configurable levels:

| Flag | Level | Passes Applied |
|------|-------|----------------|
| `--opt-level O0` | None | No optimization |
| `--opt-level O1` | Basic | Constant folding |
| `--opt-level O2` | Aggressive | Constant folding → DCE → CSE → block simplification → branch simplification |

**Constant Folding** — evaluates compile-time expressions:
```
Before:  %0 = add 3, 4        After:  %0 = iconst 7
Before:  %1 = mul 2, 0        After:  %1 = iconst 0
```

**Dead Code Elimination** — removes instructions whose results are never used.

**Common Subexpression Elimination** — deduplicates identical computations within a block.

**CFG Simplification** — merges linear chains of basic blocks; simplifies branches with constant conditions.

### Stage 7: Code Generation

The codegen backend uses **Cranelift** to emit optimized machine code:

```
            SSA IR
              │
    ┌─────────▼──────────┐
    │  Cranelift Frontend │
    │  • Function sigs    │
    │  • Block mapping    │
    │  • Instruction emit │
    └─────────┬──────────┘
              │
    ┌─────────▼──────────┐
    │  Target Selection   │
    ├─────────────────────┤
    │  x86_64-linux       │
    │  x86_64-windows     │
    │  x86_64-macos       │
    │  aarch64-linux      │
    │  aarch64-macos      │
    │  wasm32             │
    │  native (auto)      │
    └─────────┬──────────┘
              │
    ┌─────────▼──────────┐
    │  Object File (.o)   │
    │  + C Runtime Link   │
    │  → Executable       │
    └────────────────────┘
```

The code generator:
- Translates IR types to Cranelift types (`i64`, `f64`, `i8` for bools, pointer for strings)
- Emits a **C print runtime** (`klik_print_runtime.c`) compiled and linked automatically
- Handles calling conventions per platform (SysV vs. Windows ABI)
- Supports string literal pooling in the data section
- Links with the system C compiler (`cl.exe`, `gcc`, `cc`) for final executable

---

## 🧰 Toolchain

### Language Server (LSP)

Full [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) support built with `tower-lsp`:

| Feature | Status | Description |
|---------|--------|-------------|
| Diagnostics | ✅ | Real-time errors from lexer, parser, semantic, and type checker |
| Completion | ✅ | Keywords, built-in types, functions, and document symbols |
| Hover | ✅ | Keyword docs and built-in function signatures |
| Go to Definition | ✅ | Jump to function/struct/enum/trait definitions |
| Formatting | ✅ | Full document formatting via the AST formatter |

### Formatter

An **AST-based pretty printer** (845 LOC) that re-emits canonical KLIK source from parsed ASTs. Guarantees consistent formatting across all constructs: functions, structs, enums, traits, impl blocks, imports, match arms, lambdas, and more.

```bash
klik fmt              # Format all .klik files in the project
klik fmt --check      # Check formatting without modifying
```

### Linter

A **rule-based static analyzer** with 12+ configurable rules:

| Rule | Severity | Description |
|------|----------|-------------|
| `naming-convention` | Warning | Enforces snake_case for variables/functions, PascalCase for types, SCREAMING_SNAKE for constants |
| `unused-variable` | Warning | Detects declared but unused variables |
| `shadowing` | Warning | Warns when a variable shadows an outer binding |
| `empty-function` | Warning | Functions with empty bodies |
| `too-many-params` | Warning | Functions with > 7 parameters |
| `long-function` | Warning | Functions with > 60 statements |
| `self-comparison` | Warning | Expressions compared with themselves |
| `division-by-zero` | Error | Literal division by zero |
| `bool-comparison` | Warning | Unnecessary `== true` or `== false` |
| `duplicate-expression` | Warning | Consecutive identical expressions |
| `single-variant-enum` | Warning | Enums with only one variant |
| `infinite-loop` | Info | `while true` loops |
| `empty-match` | Warning | Match expressions with no arms |

### Package Manager

TOML-based project management inspired by Cargo:

```toml
# klik.toml
[package]
name = "my_project"
version = "1.0.0"
edition = "2024"
entry = "src/main.klik"

[dependencies]
json = "^1.0"

[dependencies.http]
git = "https://github.com/klik-lang/http"
branch = "main"
features = ["async"]

[build]
target = "native"
opt_level = 2
```

Features: manifest parsing, dependency resolution (registry/path/git), lock file generation, semver version constraints.

### Incremental Compilation

SHA-256-based file change detection with dependency graph tracking. Only recompiles modules whose source (or transitive dependencies) have changed:

```
                ┌────────────────────┐
                │  incremental.json  │
                │  • file hashes     │
                │  • dep graph       │
                │  • cached IR       │
                └────────┬───────────┘
                         │
  Source files ──────────▼
                  Hash comparison
                         │
              ┌──────────┴──────────┐
              │ Changed    Unchanged │
              │   │            │     │
              │ Recompile  Use cache │
              └──────────────────────┘
```

### Visualization

The CLI can export **Graphviz DOT** files for the AST, IR, and CFG:

```bash
klik build file.klik --emit-ast    # AST tree graph
klik build file.klik --emit-ir     # IR basic blocks + textual dump
klik build file.klik --emit-cfg    # Control flow graph
klik visualize file.klik --open    # Open interactive HTML pipeline view
```

---

## 📁 Examples

The `examples/` directory contains **25 programs** demonstrating the language:

| Example | Demonstrates |
|---------|-------------|
| `hello.klik` | Minimal program |
| `fibonacci.klik` | Recursion & iteration |
| `types.klik` | Structs, enums, pattern matching, impl blocks |
| `generics.klik` | Generic types, traits, Option/Result patterns |
| `awesome_klik.klik` | Full showcase — enums, structs, pipes, lambdas, recursion |
| `advanced/web_server.klik` | HTTP server |
| `advanced/json_parser.klik` | JSON parsing from scratch |
| `advanced/game_of_life.klik` | Conway's Game of Life |
| `advanced/mini_database.klik` | In-memory database |
| `advanced/task_scheduler.klik` | Async task scheduling |

---

## 🧪 Type System Reference

### Primitive Types

| Type | Size | Range |
|------|------|-------|
| `int` (alias: `i64`) | 64-bit | −2⁶³ to 2⁶³−1 |
| `i8`, `i16`, `i32` | 8/16/32-bit | Signed |
| `u8`, `u16`, `u32`, `u64` | 8/16/32/64-bit | Unsigned |
| `f32` | 32-bit | IEEE 754 single |
| `f64` | 64-bit | IEEE 754 double |
| `bool` | 8-bit | `true` / `false` |
| `char` | 32-bit | Unicode scalar |
| `string` | Pointer | UTF-8 |
| `void` | 0-bit | Unit |

### Compound Types

```klik
[int]              // array of int
(int, string)      // tuple
int?               // optional (nullable)
&Point             // immutable reference
&mut Point         // mutable reference
fn(int) -> bool    // function type
List<int>          // generic type
```

---

## 🛠️ Build & Development

### Prerequisites

- **Rust 1.75+** (2021 edition)
- **C compiler** (MSVC `cl.exe` on Windows, `gcc`/`cc` on Unix) — for linking the print runtime
- Optional: **Graphviz** — for `--emit-ast/ir/cfg` visualization

### Building

```bash
cargo build --release          # Build everything
cargo test                     # Run all tests
cargo test -p klik-linter      # Test a specific crate
```

### Project Structure

```
compiler/          Compiler pipeline (10 crates)
runtime/           Memory allocator, concurrency, error handling
stdlib/            Standard library (io, math, strings, collections, fs, time, net)
cli/               CLI binary with 15+ subcommands
lsp/               Language Server Protocol implementation
formatter/         AST-based code formatter
linter/            Static analysis with 12+ rules
package_manager/   TOML manifest, dependency resolution, lockfiles
examples/          25 example programs
tests/             Integration test suite
docs/              Architecture docs, language spec, stdlib reference
```

---

## 🔑 Key Dependencies

| Crate | Purpose |
|-------|---------|
| `cranelift-*` v0.116 | Machine code generation for all targets |
| `tower-lsp` v0.20 | Language Server Protocol framework |
| `tokio` v1 | Async runtime for LSP and concurrent features |
| `clap` v4 | CLI argument parsing |
| `serde` / `toml` | Manifest serialization |
| `sha2` v0.10 | Incremental compilation hashing |
| `semver` v1 | Version constraint resolution |
| `ariadne` v0.4 | Beautiful error reporting |
| `petgraph` v0.7 | Graph data structures (dependency resolution) |
| `dashmap` v6 | Concurrent document store for LSP |

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [`ARCHITECTURE.md`](docs/ARCHITECTURE.md) | High-level system architecture |
| [`LANGUAGE_SPEC.md`](docs/LANGUAGE_SPEC.md) | Formal grammar (EBNF), type system, semantics |
| [`LANGUAGE_GUIDE.md`](docs/LANGUAGE_GUIDE.md) | Tutorial and usage guide |
| [`COMPILER_PIPELINE.md`](docs/COMPILER_PIPELINE.md) | Stage-by-stage compilation walkthrough |
| [`STDLIB_REFERENCE.md`](docs/STDLIB_REFERENCE.md) | Standard library API reference |

---

## 🤝 Contributing

Contributions are welcome! KLIK is organized as independent crates, making it easy to work on isolated components:

1. **Fork** the repository
2. **Pick a crate** to work on (e.g., `compiler/lexer`, `linter`, `stdlib`)
3. **Write tests** — each crate has its own test suite
4. **Submit a PR** with a clear description

### Good First Issues

- Add new lint rules to the linter
- Expand standard library modules (`math`, `collections`, `strings`)
- Improve error messages with source snippets
- Add more example programs
- Enhance LSP features (rename, find references)

---

## 📜 License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

---

