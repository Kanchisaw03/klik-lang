# KLIK Architecture

## Workspace Layout

KLIK is a Rust workspace split by responsibilities.

- `cli/`: `klik` executable and command orchestration.
- `compiler/ast`: AST, spans, and language model types.
- `compiler/lexer`: tokenization.
- `compiler/parser`: AST construction from tokens.
- `compiler/semantic`: symbol/scope validation and semantic checks.
- `compiler/type_system`: type inference and type checking.
- `compiler/ir`: SSA-like intermediate representation and AST->IR lowering.
- `compiler/optimizer`: IR optimization passes.
- `compiler/codegen`: Cranelift-based IR->object backend.
- `compiler/incremental`: cache state and change tracking.
- `runtime/`: allocator, concurrency, and runtime error utilities.
- `stdlib/`: standard library modules (`collections`, `fs`, `io`, `math`, `net`, `strings`, `time`).
- `formatter/`: AST->formatted source rendering.
- `linter/`: language lint diagnostics.
- `lsp/`: language server (diagnostics, hover, definition, completion, formatting).
- `package_manager/`: `klik.toml` parsing and dependency metadata handling.
- `examples/`: sample KLIK source programs.
- `tests/`: integration KLIK programs used for toolchain checks.

## Compiler Pipeline

The verified pipeline is:

`Source -> Lexer -> Parser -> AST -> Semantic -> Type System -> IR -> Optimizer -> Codegen -> Binary`

### Stage Responsibilities

1. Lexer (`compiler/lexer`)

- Produces `Token` stream with span information.
- Supports identifiers, numeric literals, string literals, keywords, operators, punctuation.

2. Parser (`compiler/parser`)

- Converts tokens into `klik_ast::Program`.
- Supports function declarations, blocks, let bindings, expressions, control-flow constructs.

3. Semantic (`compiler/semantic`)

- Scope and symbol checks.
- Catches undefined symbols, duplicate definitions, invalid break/continue/return contexts.

4. Type system (`compiler/type_system`)

- Type checking and unification.
- Supports core scalar and function typing; validates expression compatibility.

5. IR lowering (`compiler/ir`)

- Lowers AST to SSA-style IR module (`IrModule`).
- Includes instructions like `Const`, `BinOp`, `Call`, `Load`, `Store`, `Cast`, and block terminators.

6. Optimizer (`compiler/optimizer`)

- Performs constant folding, dead code elimination, and additional CFG/CSE simplifications by optimization level.

7. Codegen (`compiler/codegen`)

- Translates IR to Cranelift and emits object bytes.
- Supports native and multiple target triples.

8. Binary emission (`cli`)

- For current native CLI path, executable emission is completed after frontend+IR+optimizer+Cranelift object generation by generating a minimal Rust backend output and invoking `rustc`.
- This keeps end-to-end `klik build <file.klik>` functional while preserving existing Cranelift object generation.

## IR Design

IR is block-based and value-oriented.

- Module: `IrModule` with functions, globals, and string literal table.
- Function: `IrFunction` with params, locals, basic blocks, return type.
- Value model: SSA-like `Value(u32)` identifiers.
- Control flow: block terminators (`Return`, `Branch`, `CondBranch`, `Switch`, `Unreachable`).
- Operations:
  - Arithmetic: integer and float binops.
  - Calls: direct function calls.
  - Memory ops: `Load`, `Store`, `Alloca`, `GetElementPtr`.
  - Compare/cast operations.

## Runtime Design

Runtime is lightweight and modular.

- `allocator.rs`: arena allocator and reference-counted allocation helpers.
- `concurrency.rs`: async task handles and channel/mutex wrappers.
- `error.rs`: runtime error model and panic utility.
- `lib.rs`: runtime lifecycle hooks (`init_runtime`, `shutdown_runtime`) currently minimal.

## Tooling Surfaces

### CLI

Supported commands include:

- `klik build [file.klik]`
- `klik run [file.klik]`
- `klik fmt [files...]`
- `klik lint [files...]`
- `klik init [name]`
- `klik add`, `klik remove`, `klik lsp`, and project management commands

### Formatter and Linter

- Formatter: AST-driven canonical source output.
- Linter: naming/style/safety checks plus rule set including unused-variable, shadowing, and long-function diagnostics.

### LSP

Provides:

- diagnostics
- hover
- go-to definition
- completion
- formatting

## Verified End-to-End Path

`klik build examples/hello.klik` produces a native executable (`hello.exe` on Windows) and running it prints:

`Hello KLIK`
