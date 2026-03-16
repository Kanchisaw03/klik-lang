# KLIK Language Guide

A comprehensive guide to the KLIK programming language.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Basic Syntax](#basic-syntax)
3. [Types](#types)
4. [Variables](#variables)
5. [Functions](#functions)
6. [Control Flow](#control-flow)
7. [Structs](#structs)
8. [Enums & Pattern Matching](#enums--pattern-matching)
9. [Arrays](#arrays)
10. [Pipe Operator](#pipe-operator)
11. [Lambdas](#lambdas)
12. [Error Handling](#error-handling)
13. [Concurrency](#concurrency)

---

## Getting Started

### Hello World

```klik
fn main() {
    println("Hello, KLIK!")
}
```

### Compiling and Running

```bash
# Run a file directly
klik run hello.klik

# Build to executable
klik build hello.klik

# Create a new project
klik new my_project
```

---

## Basic Syntax

KLIK uses a clean, expression-based syntax inspired by Rust and Kotlin.

- Statements are separated by newlines (no semicolons needed)
- Blocks use `{ }` braces
- Comments start with `//` (single line) or `/* */` (block, nestable)
- No trailing commas required

```klik
// This is a comment

/* This is a
   block comment */

fn main() {
    println("KLIK is clean and minimal")
}
```

---

## Types

### Primitive Types

| Type     | Description           | Example         |
| -------- | --------------------- | --------------- |
| `int`    | 64-bit signed integer | `42`            |
| `float`  | 64-bit float          | `3.14`          |
| `bool`   | Boolean               | `true`, `false` |
| `string` | UTF-8 string          | `"hello"`       |
| `char`   | Unicode character     | `'a'`           |
| `void`   | No value              | —               |

### Compound Types

| Type       | Description   | Example            |
| ---------- | ------------- | ------------------ |
| `[T]`      | Array of T    | `[1, 2, 3]`        |
| `T?`       | Optional T    | `Some(42)`, `None` |
| `fn(T)->U` | Function type | —                  |

---

## Variables

Variables are **immutable by default**. Use `mut` for mutable bindings.

```klik
let x = 42           // immutable
let mut y = 10       // mutable
y = 20               // OK: y is mutable

let name = "KLIK"    // type inferred as string
let pi: float = 3.14 // explicit type annotation
```

---

## Functions

Functions are declared with `fn`. Return types are specified with `->`.

```klik
fn add(a: int, b: int) -> int {
    a + b
}

fn greet(name: string) {
    println("Hello,", name)
}

fn main() {
    let sum = add(3, 5)
    println("Sum:", sum)
    greet("World")
}
```

### Early Return

```klik
fn abs(x: int) -> int {
    if x < 0 {
        return -x
    }
    x
}
```

---

## Control Flow

### If/Else Expressions

`if` is an expression and returns a value:

```klik
let max = if a > b { a } else { b }

if temperature > 100 {
    println("Hot!")
} else if temperature > 50 {
    println("Warm")
} else {
    println("Cold")
}
```

### While Loops

```klik
let mut i = 0
while i < 10 {
    println(i)
    i = i + 1
}
```

### For Loops

```klik
let items = [1, 2, 3, 4, 5]
for item in items {
    println(item)
}
```

---

## Structs

Define data structures with `struct` and add methods with `impl`:

```klik
struct Point {
    x: int,
    y: int,
}

impl Point {
    fn new(x: int, y: int) -> Point {
        Point { x: x, y: y }
    }

    fn distance_sq(self) -> int {
        self.x * self.x + self.y * self.y
    }

    fn translate(self, dx: int, dy: int) -> Point {
        Point { x: self.x + dx, y: self.y + dy }
    }
}

fn main() {
    let p = Point::new(3, 4)
    println("Distance squared:", p.distance_sq())
}
```

---

## Enums & Pattern Matching

### Defining Enums

```klik
enum Color {
    Red,
    Green,
    Blue,
}

enum Direction {
    North,
    South,
    East,
    West,
}
```

### Pattern Matching

Use `match` for exhaustive pattern matching:

```klik
fn color_name(c: Color) -> string {
    match c {
        Color::Red => "red",
        Color::Green => "green",
        Color::Blue => "blue",
    }
}

fn describe(n: int) -> string {
    match n {
        0 => "zero",
        1 => "one",
        _ => "other",
    }
}
```

---

## Arrays

Arrays are ordered collections of elements:

```klik
let numbers = [1, 2, 3, 4, 5]
let names = ["Alice", "Bob", "Charlie"]

// Access by index
println(numbers[0])  // 1

// Iterate
for n in numbers {
    println(n)
}

// Length
println("Count:", len(numbers))
```

---

## Pipe Operator

The pipe operator `|>` chains operations in a readable left-to-right flow:

```klik
// Without pipes
let result = sum(filter(map([1,2,3,4,5], double), is_even))

// With pipes - much clearer!
let result = [1, 2, 3, 4, 5]
    |> map(|x| x * 2)
    |> filter(|x| x > 5)
    |> sum()
```

### Available Iterator Operations

| Operation       | Description                  | Returns  |
| --------------- | ---------------------------- | -------- |
| `map(f)`        | Transform each element       | Array    |
| `filter(f)`     | Keep elements matching pred  | Array    |
| `sum()`         | Sum all elements             | int      |
| `count()`       | Count elements               | int      |
| `collect()`     | Collect into array           | Array    |
| `fold(init, f)` | Reduce with initial value    | Value    |
| `reduce(f)`     | Reduce without initial value | Value    |
| `any(f)`        | Any element matches?         | bool     |
| `all(f)`        | All elements match?          | bool     |
| `find(f)`       | First matching element       | Optional |
| `min()`         | Minimum element              | Value    |
| `max()`         | Maximum element              | Value    |
| `take(n)`       | First n elements             | Array    |
| `skip(n)`       | Skip first n elements        | Array    |
| `enumerate()`   | Pair with indices            | Array    |
| `flat_map(f)`   | Map and flatten              | Array    |
| `for_each(f)`   | Execute side effect          | void     |

### Pipeline Examples

```klik
// Sum of squares of even numbers
let result = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    |> filter(|x| x % 2 == 0)
    |> map(|x| x * x)
    |> sum()

// Check if any number > 100
let has_big = [10, 50, 200, 5] |> any(|x| x > 100)

// Count items matching a condition
let num_even = [1, 2, 3, 4, 5] |> filter(|x| x % 2 == 0) |> count()
```

---

## Lambdas

Anonymous functions (closures) are defined with `|params| body`:

```klik
let double = |x| x * 2
let add = |a, b| a + b

// Used inline with pipe operations
[1, 2, 3] |> map(|x| x * x) |> collect()

// Multi-line lambdas use blocks
[1, 2, 3] |> for_each(|x| {
    println("Value:", x)
})
```

---

## Error Handling

KLIK uses `Result` types for error handling:

```klik
fn divide(a: int, b: int) -> Result<int, string> {
    if b == 0 {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}
```

---

## Concurrency

KLIK supports async/await and spawn for concurrent programming:

```klik
async fn fetch_data() -> string {
    // async operations here
    "data"
}

fn main() {
    // Spawn a concurrent task
    let handle = spawn(|| {
        println("Running in another thread!")
    })
}
```

---

## Best Practices

1. **Prefer immutability** — Use `let` by default, `mut` only when needed
2. **Use pipes for data transformation** — Chains of map/filter/reduce are clearer than nested calls
3. **Leverage pattern matching** — `match` is more readable than long if/else chains
4. **Struct methods with `impl`** — Keep data and behavior together
5. **Keep functions small** — Each function should do one thing well
