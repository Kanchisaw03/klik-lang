# KLIK Standard Library Reference

Complete reference for KLIK's built-in functions and standard library modules.

---

## Built-in Functions

These functions are available without any imports.

### I/O

#### `println(...args)`

Print values to stdout followed by a newline. Accepts any number of arguments.

```klik
println("Hello")              // Hello
println("x =", 42)            // x = 42
println("a", "b", "c")        // a b c
println()                      // (empty line)
```

#### `print(...args)`

Print values to stdout without a trailing newline.

```klik
print("Enter name: ")
```

### Type Conversion

#### `to_string(value) -> string`

Convert any value to its string representation.

```klik
let s = to_string(42)    // "42"
```

### Collections

#### `len(collection) -> int`

Return the number of elements in an array or characters in a string.

```klik
len([1, 2, 3])      // 3
len("hello")         // 5
```

### Assertions

#### `assert(condition)`

Assert that a condition is true. Panics if false.

```klik
assert(1 + 1 == 2)
```

### Optional Values

#### `Some(value) -> T?`

Wrap a value in an Optional.

#### `None -> T?`

Represents the absence of a value.

### Result Values

#### `Ok(value) -> Result<T, E>`

Wrap a success value.

#### `Err(error) -> Result<T, E>`

Wrap an error value.

---

## Iterator Operations (Pipe Operator)

These operations work with the pipe operator `|>` on arrays.

### Transformations

#### `map(fn) -> [U]`

Apply a function to each element, producing a new array.

```klik
[1, 2, 3] |> map(|x| x * 2)           // [2, 4, 6]
["a", "b"] |> map(|s| s + "!")         // ["a!", "b!"]
```

#### `flat_map(fn) -> [U]`

Map each element to an array and flatten the results.

```klik
[1, 2, 3] |> flat_map(|x| [x, x * 10])  // [1, 10, 2, 20, 3, 30]
```

#### `enumerate() -> [(int, T)]`

Pair each element with its index.

```klik
["a", "b", "c"] |> enumerate()  // [(0, "a"), (1, "b"), (2, "c")]
```

### Filtering

#### `filter(predicate) -> [T]`

Keep only elements for which the predicate returns true.

```klik
[1, 2, 3, 4, 5] |> filter(|x| x > 3)   // [4, 5]
```

#### `take(n) -> [T]`

Take the first `n` elements.

```klik
[1, 2, 3, 4, 5] |> take(3)              // [1, 2, 3]
```

#### `skip(n) -> [T]`

Skip the first `n` elements.

```klik
[1, 2, 3, 4, 5] |> skip(2)              // [3, 4, 5]
```

### Aggregation

#### `sum() -> int`

Sum all elements (numeric arrays only).

```klik
[1, 2, 3, 4, 5] |> sum()                // 15
```

#### `count() -> int`

Count the number of elements.

```klik
[1, 2, 3] |> filter(|x| x > 1) |> count()  // 2
```

#### `min() -> T`

Find the minimum element.

```klik
[5, 2, 8, 1, 9] |> min()                // 1
```

#### `max() -> T`

Find the maximum element.

```klik
[5, 2, 8, 1, 9] |> max()                // 9
```

#### `fold(initial, fn) -> U`

Reduce elements with an initial accumulator value.

```klik
[1, 2, 3] |> fold(0, |acc, x| acc + x)  // 6
[1, 2, 3] |> fold(1, |acc, x| acc * x)  // 6
```

#### `reduce(fn) -> T`

Reduce elements using the first element as initial value.

```klik
[1, 2, 3, 4] |> reduce(|a, b| a + b)    // 10
```

### Predicates

#### `any(predicate) -> bool`

Returns true if any element satisfies the predicate.

```klik
[1, 2, 3] |> any(|x| x > 2)             // true
```

#### `all(predicate) -> bool`

Returns true if all elements satisfy the predicate.

```klik
[2, 4, 6] |> all(|x| x % 2 == 0)        // true
```

#### `find(predicate) -> T?`

Find the first element matching the predicate.

```klik
[1, 2, 3, 4] |> find(|x| x > 2)         // Some(3)
```

### Collection

#### `collect() -> [T]`

Materialize an iterator chain into an array.

```klik
[1, 2, 3] |> map(|x| x * 2) |> collect()  // [2, 4, 6]
```

### Side Effects

#### `for_each(fn)`

Execute a function on each element (for side effects).

```klik
[1, 2, 3] |> for_each(|x| println(x))
```

---

## Standard Library Modules

### `math`

Mathematical functions and constants.

| Function    | Signature                        | Description             |
| ----------- | -------------------------------- | ----------------------- |
| `abs_i64`   | `(int) -> int`                   | Absolute value          |
| `abs_f64`   | `(float) -> float`               | Absolute value          |
| `pow_i64`   | `(int, int) -> int`              | Integer power           |
| `pow_f64`   | `(float, float) -> float`        | Float power             |
| `sqrt`      | `(float) -> float`               | Square root             |
| `cbrt`      | `(float) -> float`               | Cube root               |
| `floor`     | `(float) -> float`               | Floor                   |
| `ceil`      | `(float) -> float`               | Ceiling                 |
| `round`     | `(float) -> float`               | Round                   |
| `min_i64`   | `(int, int) -> int`              | Minimum                 |
| `max_i64`   | `(int, int) -> int`              | Maximum                 |
| `min_f64`   | `(float, float) -> float`        | Minimum                 |
| `max_f64`   | `(float, float) -> float`        | Maximum                 |
| `clamp_i64` | `(int, int, int) -> int`         | Clamp to range          |
| `clamp_f64` | `(float, float, float) -> float` | Clamp                   |
| `gcd`       | `(int, int) -> int`              | Greatest common divisor |
| `lcm`       | `(int, int) -> int`              | Least common multiple   |

### `strings`

String manipulation functions.

| Function      | Signature                            | Description          |
| ------------- | ------------------------------------ | -------------------- |
| `contains`    | `(string, string) -> bool`           | Substring search     |
| `starts_with` | `(string, string) -> bool`           | Prefix check         |
| `ends_with`   | `(string, string) -> bool`           | Suffix check         |
| `to_upper`    | `(string) -> string`                 | Uppercase            |
| `to_lower`    | `(string) -> string`                 | Lowercase            |
| `trim`        | `(string) -> string`                 | Trim whitespace      |
| `trim_start`  | `(string) -> string`                 | Trim leading spaces  |
| `trim_end`    | `(string) -> string`                 | Trim trailing spaces |
| `split`       | `(string, string) -> [string]`       | Split by delimiter   |
| `join`        | `([string], string) -> string`       | Join with separator  |
| `replace`     | `(string, string, string) -> string` | Replace substring    |
| `substring`   | `(string, int, int) -> string`       | Extract substring    |
| `char_at`     | `(string, int) -> char`              | Character at index   |
| `repeat`      | `(string, int) -> string`            | Repeat string        |
| `reverse`     | `(string) -> string`                 | Reverse string       |
| `is_empty`    | `(string) -> bool`                   | Check if empty       |

### `collections`

Generic collection types.

#### `List<T>`

A growable, generic list.

| Method     | Signature       | Description            |
| ---------- | --------------- | ---------------------- |
| `new`      | `() -> List<T>` | Create empty list      |
| `push`     | `(T)`           | Append element         |
| `pop`      | `() -> T?`      | Remove and return last |
| `len`      | `() -> int`     | Number of elements     |
| `is_empty` | `() -> bool`    | Check if empty         |
| `get`      | `(int) -> T?`   | Get by index           |
| `insert`   | `(int, T)`      | Insert at index        |
| `remove`   | `(int) -> T`    | Remove at index        |

### `io`

Input/output operations.

| Function    | Signature      | Description               |
| ----------- | -------------- | ------------------------- |
| `print`     | `(...args)`    | Print to stdout           |
| `println`   | `(...args)`    | Print with newline        |
| `eprint`    | `(...args)`    | Print to stderr           |
| `eprintln`  | `(...args)`    | Print to stderr + newline |
| `read_line` | `() -> string` | Read line from stdin      |

### `fs`

File system operations.

| Function      | Signature                        | Description        |
| ------------- | -------------------------------- | ------------------ |
| `read_file`   | `(string) -> Result<string>`     | Read file contents |
| `write_file`  | `(string, string) -> Result<()>` | Write to file      |
| `append_file` | `(string, string) -> Result<()>` | Append to file     |
| `file_exists` | `(string) -> bool`               | Check existence    |
| `remove_file` | `(string) -> Result<()>`         | Delete file        |
| `create_dir`  | `(string) -> Result<()>`         | Create directory   |
| `list_dir`    | `(string) -> Result<[string]>`   | List directory     |

### `time`

Time and duration utilities.

| Function       | Signature   | Description            |
| -------------- | ----------- | ---------------------- |
| `now_millis`   | `() -> int` | Current time in ms     |
| `now_micros`   | `() -> int` | Current time in µs     |
| `now_nanos`    | `() -> int` | Current time in ns     |
| `sleep_millis` | `(int)`     | Sleep for milliseconds |
| `sleep_secs`   | `(int)`     | Sleep for seconds      |

### `net`

Networking primitives.

| Function      | Signature                             | Description       |
| ------------- | ------------------------------------- | ----------------- |
| `tcp_connect` | `(string, int) -> Result<Connection>` | TCP connect       |
| `tcp_listen`  | `(string, int) -> Result<Listener>`   | TCP listen        |
| `http_get`    | `(string) -> Result<string>`          | HTTP GET request  |
| `http_post`   | `(string, string) -> Result<string>`  | HTTP POST request |
