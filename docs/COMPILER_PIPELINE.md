# KLIK Compiler Pipeline

This document describes the internal architecture of the KLIK compiler,
from source code to native binary.

## Overview

```
┌──────────┐    ┌────────┐    ┌──────────┐    ┌──────────┐    ┌────────────┐
│  Source   │───▶│ Lexer  │───▶│  Parser  │───▶│ Semantic │───▶│    Type    │
│  (.klik) │    │        │    │          │    │ Analysis │    │   Checker  │
└──────────┘    └────────┘    └──────────┘    └──────────┘    └────────────┘
                                                                     │
                    ┌──────────────────────────────────────────────────┘
                    │
                    ▼
             ┌─────────────┐    ┌───────────┐    ┌────────┐    ┌──────────┐
             │ Transpiler  │───▶│   Rust    │───▶│  rustc │───▶│  Native  │
             │ (AST→Rust)  │    │  Source   │    │        │    │  Binary  │
             └─────────────┘    └───────────┘    └────────┘    └──────────┘
```

## Pipeline Stages

### Stage 1: Lexer (`compiler/lexer`)

The lexer (tokenizer) converts raw source text into a stream of tokens.

**Input:** Raw KLIK source code (UTF-8 string)
**Output:** `Vec<Token>` — ordered sequence of tokens

**Token Categories:**
| Category | Examples |
|-------------|-------------------------------------|
| Keywords | `fn`, `let`, `mut`, `if`, `struct` |
| Identifiers | `main`, `x`, `my_func` |
| Literals | `42`, `3.14`, `"hello"`, `true` |
| Operators | `+`, `-`, `*`, `|>`, `==`, `&&` |
| Delimiters | `(`, `)`, `{`, `}`, `[`, `]` |
| Special | `->`, `=>`, `::`, `,`, `.` |

**Key Features:**

- Handles nested block comments (`/* /* nested */ */`)
- Supports hex (`0xFF`), binary (`0b1010`), octal (`0o77`) integer literals
- Tracks line/column positions for error reporting

### Stage 2: Parser (`compiler/parser`)

The parser converts the token stream into an Abstract Syntax Tree (AST).

**Input:** `Vec<Token>`
**Output:** `Program` (AST root node)

**Parser Type:** Recursive descent with Pratt parsing for expressions

**AST Node Types:**

- **Items:** `FnDef`, `StructDef`, `EnumDef`, `ImplBlock`, `TraitDef`
- **Statements:** `Let`, `Assignment`, `ExprStmt`, `Return`, `While`, `For`
- **Expressions:** `Literal`, `Identifier`, `Binary`, `Unary`, `Call`, `MethodCall`, `FieldAccess`, `Index`, `If`, `Match`, `Lambda`, `Block`, `Array`, `StructInit`

**Operator Precedence (lowest to highest):**

1. Pipe (`|>`)
2. Logical OR (`||`)
3. Logical AND (`&&`)
4. Comparison (`==`, `!=`, `<`, `>`, `<=`, `>=`)
5. Bitwise OR, XOR, AND (`|`, `^`, `&`)
6. Shift (`<<`, `>>`)
7. Addition (`+`, `-`)
8. Multiplication (`*`, `/`, `%`)
9. Unary (`!`, `-`)
10. Postfix (`.`, `()`, `[]`)

### Stage 3: Semantic Analysis (`compiler/semantic`)

The semantic analyzer performs symbol resolution and scope checking.

**Input:** `Program` (AST)
**Output:** Validated AST + diagnostics

**Checks Performed:**

- Variable resolution (undefined variables)
- Function existence (undefined functions)
- Scope management (block-level scoping)
- Struct/enum definition validation
- Import resolution
- Pipe operator RHS validation (recognizes iterator functions)
- Enum variant path resolution (`Color::Red`)

### Stage 4: Type Checker (`compiler/type_system`)

The type checker infers and validates types across the program.

**Input:** `Program` (AST)
**Output:** Type-annotated AST + type errors

**Type System Features:**

- **Bidirectional type inference** — types flow both forward and backward
- **Structural typing** for structs
- **Enum variant resolution** via `::` paths
- **Self type binding** in impl blocks
- **Pipe expression type tracking** — infers types through map/filter/sum chains
- **Variadic builtins** — `println`, `print`, `assert` accept any number of args

**Type Categories:**
| Category | Types |
|----------|-------|
| Numeric | `Int`, `Float` |
| Text | `String`, `Char` |
| Boolean | `Bool` |
| Compound | `Array(T)`, `Optional(T)`, `Struct(name, fields)` |
| Special | `Void`, `Function(params, return)`, `Self` |

### Stage 5: Transpiler (in `cli/src/commands.rs`)

The transpiler converts the KLIK AST directly to Rust source code.

**Input:** Type-checked `Program` (AST)
**Output:** Valid Rust source file (`.rs`)

**Transpilation Rules:**

| KLIK Feature      | Rust Output                                 |
| ----------------- | ------------------------------------------- |
| `int`             | `i64`                                       |
| `float`           | `f64`                                       |
| `string`          | `String`                                    |
| `[T]`             | `Vec<T>`                                    |
| `struct`          | `#[derive(Clone, Debug)] struct`            |
| `enum`            | `#[derive(Clone, Debug)] enum`              |
| `impl`            | `impl`                                      |
| `println(a, b)`   | `println!("{} {}", display(a), display(b))` |
| `x \|> map(f)`    | `x.into_iter().map(f)`                      |
| `x \|> filter(f)` | `x.into_iter().filter(\|&x\| f(x))`         |
| `x \|> sum()`     | `x.into_iter().sum::<i64>()`                |
| `x \|> collect()` | `x.into_iter().collect::<Vec<_>>()`         |

**Special Handling:**

- String concatenation with `+` uses `format!()` when literals are involved
- Array/identifier arguments are auto-cloned to preserve KLIK's value semantics
- Pipe sources from identifiers are cloned before `.into_iter()` to allow reuse
- Filter-family closures auto-dereference parameters (Rust iterators pass references)
- A `klik_display` helper is emitted for proper formatting of mixed types

### Stage 6: Native Compilation (via `rustc`)

The generated Rust source is compiled to a native binary using `rustc`.

**Process:**

1. Write generated `.rs` file to a temp directory
2. Invoke `rustc` with appropriate flags
3. Produce native executable for the host platform
4. Report any compilation errors with source mapping

**Build Modes:**
| Mode | Flags | Description |
|---------|--------------------|-----------------------|
| Debug | (default) | Fast compilation |
| Release | `-C opt-level=3` | Optimized binary |

## Crate Structure

```
klik/
├── cli/                 # Command-line interface (clap 4)
├── compiler/
│   ├── ast/            # AST type definitions
│   ├── lexer/          # Tokenization
│   ├── parser/         # Parsing (tokens → AST)
│   ├── semantic/       # Symbol resolution, scope checking
│   ├── type_system/    # Type inference and checking
│   ├── ir/             # Intermediate representation (SSA-based)
│   ├── optimizer/      # IR optimization passes
│   ├── codegen/        # Cranelift code generation (planned)
│   └── incremental/    # Incremental compilation support
├── runtime/            # Runtime library (allocator, concurrency)
├── stdlib/             # Standard library modules
├── formatter/          # Code formatter
├── linter/             # Linter
├── lsp/                # Language Server Protocol implementation
└── package_manager/    # Package management
```

## Data Flow

```
Source Code
    │
    ▼
Lexer::tokenize(source) → Vec<Token>
    │
    ▼
Parser::parse(tokens) → Program (AST)
    │
    ▼
SemanticAnalyzer::analyze(program) → Diagnostics
    │
    ▼
TypeChecker::check(program) → Type Errors
    │
    ▼
transpile_program_to_rust(program) → String (Rust code)
    │
    ▼
rustc <temp.rs> -o <output> → Native Binary
    │
    ▼
Execute binary → Program Output
```

## Error Reporting

The compiler uses colored diagnostics with source location:

```
error: type error: type mismatch: expected int, found string at file.klik:10:5
```

Errors are reported at each stage:

- **Lexer:** Invalid tokens, unterminated strings
- **Parser:** Unexpected tokens, missing delimiters
- **Semantic:** Undefined variables/functions
- **Type Checker:** Type mismatches, invalid operations
- **Transpiler:** Unsupported language features
- **rustc:** Borrow checker issues, type errors in generated code
