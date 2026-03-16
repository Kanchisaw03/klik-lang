# KLIK Compiler Architecture

## Overview

The KLIK compiler is a multi-stage pipeline that transforms source code into native binaries or WebAssembly. It is implemented as a Rust workspace with modular crates.

## Crate Layout

```
klik/
├── Cargo.toml              # Workspace root
├── compiler/
│   ├── ast/                 # Abstract Syntax Tree definitions
│   ├── lexer/               # Tokenizer
│   ├── parser/              # Recursive descent parser
│   ├── semantic/            # Name resolution & validation
│   ├── type_system/         # Type inference & checking
│   ├── ir/                  # SSA Intermediate Representation
│   ├── optimizer/           # Optimization passes
│   ├── codegen/             # Cranelift code generation
│   └── incremental/         # Incremental compilation cache
├── runtime/                 # Runtime library (allocator, concurrency)
├── stdlib/                  # Standard library modules
├── package_manager/         # Dependency resolution & project management
├── cli/                     # `klik` command-line tool
├── lsp/                     # Language Server Protocol server
├── formatter/               # Code formatter
├── linter/                  # Static analysis / linter
├── examples/                # Example KLIK programs
└── docs/                    # Documentation
```

## Compilation Pipeline

```
  Source (.klik)
       │
       ▼
  ┌─────────────────────────────────────────────────┐
  │  LEXER (compiler/lexer)                         │
  │  • Hand-written scanner                         │
  │  • Unicode support                              │
  │  • Nestable block comments                      │
  │  • Source location tracking (line:column:offset) │
  │  Output: Vec<Token>                              │
  └─────────────────────────────────────────────────┘
       │
       ▼
  ┌─────────────────────────────────────────────────┐
  │  PARSER (compiler/parser)                       │
  │  • Recursive descent for statements/items       │
  │  • Pratt parsing for expression precedence      │
  │  • Error recovery with synchronization          │
  │  Output: AST (Program)                          │
  └─────────────────────────────────────────────────┘
       │
       ▼
  ┌─────────────────────────────────────────────────┐
  │  SEMANTIC ANALYSIS (compiler/semantic)          │
  │  • Name resolution                              │
  │  • Scope tracking (global/function/block/loop)  │
  │  • Duplicate definition detection               │
  │  • Mutability enforcement                       │
  │  • Control flow validation (break/continue)     │
  │  Output: Validated AST                          │
  └─────────────────────────────────────────────────┘
       │
       ▼
  ┌─────────────────────────────────────────────────┐
  │  TYPE CHECKER (compiler/type_system)            │
  │  • Hindley-Milner type inference                │
  │  • Unification engine                           │
  │  • Generic instantiation                        │
  │  • Type variable generation                     │
  │  • Constraint solving                           │
  │  Output: Fully-typed AST                        │
  └─────────────────────────────────────────────────┘
       │
       ▼
  ┌─────────────────────────────────────────────────┐
  │  IR BUILDER (compiler/ir)                       │
  │  • Generates SSA form IR                        │
  │  • Basic blocks with terminators                │
  │  • Control flow graph construction              │
  │  • Phi nodes for value merging                  │
  │  Output: IrModule                               │
  └─────────────────────────────────────────────────┘
       │
       ▼
  ┌─────────────────────────────────────────────────┐
  │  OPTIMIZER (compiler/optimizer)                 │
  │  Passes:                                        │
  │  • Constant folding                             │
  │  • Dead code elimination                        │
  │  • Common subexpression elimination             │
  │  • CFG simplification                           │
  │  Levels: None | Basic | Standard | Aggressive   │
  │  Output: Optimized IrModule                     │
  └─────────────────────────────────────────────────┘
       │
       ▼
  ┌─────────────────────────────────────────────────┐
  │  CODE GENERATOR (compiler/codegen)              │
  │  • Cranelift IR translation                     │
  │  • Target support:                              │
  │    - x86_64 (Linux, macOS, Windows)             │
  │    - AArch64 (Linux, macOS)                     │
  │    - WebAssembly (wasm32)                       │
  │  • Object file emission                         │
  │  Output: Native binary or .wasm                 │
  └─────────────────────────────────────────────────┘
```

## Incremental Compilation

The incremental compilation system (`compiler/incremental`) tracks:

1. **File hashes** — SHA-256 of each source file
2. **Dependency graph** — Which modules depend on which
3. **Cached artifacts** — Previously compiled modules

On rebuild, only changed files and their dependents are recompiled.

## Runtime System

The runtime (`runtime/`) provides:

- **Arena Allocator** — Fast bump allocation with 64KB chunks
- **Reference Counting** — `RcAlloc<T>` for heap objects
- **Task System** — Lightweight async tasks via tokio
- **Channels** — Bounded MPSC channels for inter-task communication
- **Mutex** — `parking_lot`-based mutexes
- **Error Handling** — Runtime error types and panic handler

## Standard Library

| Module        | Contents                            |
| ------------- | ----------------------------------- |
| `io`          | print, println, read_line           |
| `math`        | Numeric operations, trig, constants |
| `strings`     | String manipulation                 |
| `collections` | List, Map, Set, Deque               |
| `fs`          | File system operations              |
| `time`        | Timestamps, timers, sleep           |
| `net`         | TCP/UDP networking                  |

## Tooling

### CLI (`cli/`)

The `klik` binary provides all development commands: new, build, run, check, test, fmt, lint, watch, lsp.

### LSP Server (`lsp/`)

Tower-LSP based server providing:

- Real-time diagnostics
- Auto-completion (keywords, symbols, types)
- Hover information
- Go-to-definition
- Document formatting

### Formatter (`formatter/`)

AST-based code formatter that produces canonical formatting.

### Linter (`linter/`)

Static analysis rules:

- Naming conventions (snake_case, PascalCase, SCREAMING_SNAKE_CASE)
- Empty functions/structs
- Division by zero
- Self-comparison
- Boolean comparison simplification
- Too many parameters
- Duplicate expressions
- Infinite loop detection

## Future Roadmap

- [ ] Self-hosting (KLIK compiler written in KLIK)
- [ ] LLVM backend (alternative to Cranelift)
- [ ] Borrow checker / lifetime analysis
- [ ] Trait object dispatch
- [ ] Macro system
- [ ] Package registry
- [ ] Debugger integration (DWARF/DAP)
- [ ] Playground (web-based)
