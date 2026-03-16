# KLIK Social Launch Kit

This file is a practical launch kit for showing KLIK in the best possible way on X, LinkedIn, Reddit, Discord, GitHub, and short-form video.

## Positioning

Use this framing:

- KLIK is not just a parser or toy compiler.
- It is a full language toolchain built in Rust.
- It has a real compiler pipeline, a Cranelift backend, a Rust transpiler backend, IR optimization passes, testing tools, visualization, tracing, and an ecosystem around it.

Short hook:

> I built a programming language in Rust with two backends, SSA/IR, Cranelift codegen, optimizer passes, CFG/AST visualization, tracing, formatter, linter, package manager, and LSP support.

## Best Demo Order

Use this order for the strongest 30-60 second demo:

1. Show `klik test-backend`
2. Show `klik run examples/awesome_klik.klik`
3. Show `klik run examples/advanced/web_server.klik`
4. Show `klik run examples/advanced/pipeline_showcase.klik`
5. Show `klik build examples/benchmark.klik --opt-level O2 --emit-ir --emit-cfg`
6. Show `klik run examples/test_pipe.klik --trace`
7. Show `klik visualize examples/test_pipe.klik`

This sequence communicates:

- language breadth
- compiler engineering depth
- backend correctness
- tooling maturity
- developer experience

## Highest-Impact Demos

### Demo 1: Backend Parity

Command:

```powershell
cargo run -p klik-cli -- test-backend
```

Why it works:

- proves the Rust backend and Cranelift backend agree on core examples
- shows benchmarking data
- makes the project feel engineered, not improvised

Lines worth highlighting:

- `PASS pipeline_validation`
- `PASS benchmark`
- `PASS stress`
- backend compile time comparison
- benchmark execution time comparison

### Demo 2: Full Language Showcase

Command:

```powershell
cargo run -p klik-cli -- run examples/awesome_klik.klik
```

Why it works:

- one file demonstrates enums, pattern matching, structs, impl methods, pipelines, lambdas, recursion, loops, and analytics-like computation

Real output highlights from the current project:

- `Weighted Total: 3016`
- `High values (>=85): [95, 88, 91, 87, 93]`
- `Sum of cubes: 3025`
- `Features demonstrated: Enums, Structs, Methods, Pipelines, Recursion, Control Flow, Lambdas`

### Demo 3: Web Server Simulation

Command:

```powershell
cargo run -p klik-cli -- run examples/advanced/web_server.klik
```

Why it works:

- people instantly understand request/response flows
- shows structs, branching, route handling, aggregation, and pipelines in a concrete app-shaped demo

Real output highlights:

- `[ 200 OK ] GET / -> Welcome to KLIK Web Server!`
- `[ 201 Created ] POST /users -> User created: Diana`
- `Success responses (2xx): 7`
- `Error responses (4xx+): 2`

### Demo 4: Pipe Operator / Functional Style

Command:

```powershell
cargo run -p klik-cli -- run examples/advanced/pipeline_showcase.klik
```

Why it works:

- concise and visual
- great for code screenshot + terminal screenshot pair

Real output highlights:

- `Sum of doubled numbers > 5: 104`
- `Sum of even Fibonacci numbers (0-10): 44`
- `Product 1..5: 120`

### Demo 5: Compiler Internals / Engineering Credibility

Commands:

```powershell
cargo run -p klik-cli -- build examples/benchmark.klik --opt-level O2 --emit-ir --emit-ast --emit-cfg
cargo run -p klik-cli -- run examples/test_pipe.klik --trace
```

Why it works:

- shows this is a real compiler, not only a syntax demo
- optimization levels, IR dumping, CFG output, and trace logs are strong trust signals

Trace lines worth showing:

- `[PARSE] AST built successfully`
- `[IR] IR module generated`
- `[OPT] constant folding pass applied (...)`
- `[CODEGEN] native transpiler backend selected`
- `[LINK] rustc produced executable`
- `[RUN] program executed successfully`

## Screenshot Plan

Capture these in order.

### Screenshot 1: Project credibility shot

Show:

- terminal running `klik test-backend`
- visible `CORE TEST RESULTS`
- visible benchmark timing section

Goal:

- instantly communicate this is a serious language project

### Screenshot 2: Language ergonomics shot

Show code from:

- `examples/advanced/pipeline_showcase.klik`

Focus on:

- `|> map(...) |> filter(...) |> sum()`

Pair with terminal output:

- `Sum of doubled numbers > 5: 104`
- `Product 1..5: 120`

### Screenshot 3: “This is a real language” shot

Show code from:

- `examples/awesome_klik.klik`

Focus on:

- enum definitions
- struct definitions
- `impl` methods
- pipeline section

Pair with output section showing:

- weighted analytics summary
- recursion results
- pipeline outputs

### Screenshot 4: App-shaped demo shot

Show code from:

- `examples/advanced/web_server.klik`

Pair with terminal lines:

- `GET / -> Welcome to KLIK Web Server!`
- `POST /users -> User created: Diana`

### Screenshot 5: Compiler-internals shot

Show terminal output from:

- `klik build examples/benchmark.klik --opt-level O2 --emit-ir --emit-ast --emit-cfg`

And show generated files:

- `benchmark.ir`
- `benchmark.dot`
- `benchmark.ast.dot`
- `benchmark.cfg.dot`

### Screenshot 6: Trace mode shot

Show terminal output from:

- `klik run examples/test_pipe.klik --trace`

Goal:

- developers love seeing compiler stage logs

## Short Video Script

### 20-second version

1. Open with terminal on `klik test-backend`
2. Scroll through `PASS` lines and timing numbers
3. Cut to `awesome_klik.klik`
4. Run it and show the output header and feature summary
5. Cut to `test_pipe.klik --trace`
6. End on emitted IR/CFG files from `benchmark`

On-screen caption:

> Built a programming language in Rust. Two backends. SSA + IR. Cranelift. Optimizer passes. Trace mode. Visualization. Real examples.

### 45-second version

1. Show the repo tree briefly
2. Run `klik test-backend`
3. Show `awesome_klik.klik` code
4. Run `awesome_klik.klik`
5. Show `web_server.klik` output
6. Run `test_pipe.klik --trace`
7. Run `benchmark --opt-level O2 --emit-ir --emit-cfg`
8. Close on generated compiler artifacts

## Ready-to-Post Copy

### X / Twitter Post

```text
I built a programming language in Rust.

KLIK has:
- parser -> AST -> IR -> SSA -> Cranelift -> object -> linker -> exe
- a Rust transpiler backend + a Cranelift backend
- optimizer passes
- CFG / AST / IR visualization
- trace mode
- formatter, linter, package manager, and LSP

Core backend parity tests are passing, including benchmark + stress cases.

This is one of the coolest things I’ve built.
```

Shorter version:

```text
Built a programming language in Rust.

Not just syntax:
- IR + SSA
- Cranelift backend
- optimizer passes
- backend parity tests
- trace mode
- AST/IR/CFG visualization
- formatter/linter/LSP

KLIK is getting real.
```

### LinkedIn Post

```text
I’ve been building a programming language called KLIK in Rust.

What makes it exciting for me is that it has moved past “toy compiler” territory.

Current pipeline:
KLIK source -> parser -> AST -> IR -> SSA -> Cranelift codegen -> object -> linker -> executable

It now includes:
- a Rust backend and a Cranelift backend
- IR optimization passes
- backend comparison testing
- compile-time and execution-time benchmarking
- AST / IR / CFG visualization
- developer trace mode
- formatter, linter, package manager, and LSP support

The most satisfying part is seeing real examples run consistently across both backends.

I’m sharing a few demos below: analytics-style pipelines, pattern matching, structs + impl methods, and a web-server simulator.
```

### Reddit / Hacker News Style Intro

```text
Show HN: KLIK, a statically typed programming language I’m building in Rust

It currently has:
- parser / AST / semantic analysis / type checking
- IR + SSA-based lowering
- Cranelift codegen backend
- Rust transpiler backend for comparison
- optimization passes (constant folding, DCE, CFG simplification, branch simplification)
- backend parity tests and benchmarking
- AST / IR / CFG visualization
- trace mode for compiler stages

I’ve been focusing on making the compiler pipeline real and inspectable rather than only adding syntax features.

A few demos include:
- functional pipelines with `|>`
- enums + pattern matching
- structs + impl methods
- analytics-style showcase app
- web-server simulation
```

## Suggested Hashtags

Use sparingly. Good combinations:

- `#rustlang #compilers #programminglanguages #opensource`
- `#buildinpublic #systemsprogramming #devtools`

## Thumbnail / Cover Text Ideas

- `I built a programming language in Rust`
- `From source code to executable`
- `Parser -> AST -> IR -> SSA -> Cranelift`
- `Not a toy compiler anymore`
- `Two backends. One language.`

## Practical Capture Commands

Run these from repo root.

```powershell
cargo run -p klik-cli -- test-backend
cargo run -p klik-cli -- run examples/awesome_klik.klik
cargo run -p klik-cli -- run examples/advanced/pipeline_showcase.klik
cargo run -p klik-cli -- run examples/advanced/web_server.klik
cargo run -p klik-cli -- build examples/benchmark.klik --opt-level O2 --emit-ir --emit-ast --emit-cfg
cargo run -p klik-cli -- run examples/test_pipe.klik --trace
```

For visualization images:

```powershell
cargo run -p klik-cli -- visualize examples/test_pipe.klik
```

Note:

- PNG rendering requires Graphviz `dot` installed and available on PATH.

## Best Single-Line Pitch

Use this if you only get one sentence:

> KLIK is a Rust-built programming language with a real compiler pipeline, dual backends, IR optimization passes, tracing, visualization, and passing backend parity tests.
