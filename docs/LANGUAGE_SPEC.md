# KLIK Language Specification

## Lexical Grammar

### Tokens

```
IDENTIFIER  ::= [a-zA-Z_][a-zA-Z0-9_]*
INT_LIT     ::= [0-9][0-9_]* | '0x'[0-9a-fA-F_]+ | '0b'[01_]+ | '0o'[0-7_]+
FLOAT_LIT   ::= [0-9]+[.]([0-9]+)?([eE][+-]?[0-9]+)?
STRING_LIT  ::= '"' (CHAR | ESCAPE)* '"'
CHAR_LIT    ::= '\'' (CHAR | ESCAPE) '\''
ESCAPE      ::= '\\' [nrt\\'"0] | '\\u{' HEX+ '}'
COMMENT     ::= '//' .* '\n' | '/*' (COMMENT | .)* '*/'
```

### Keywords

```
fn  let  mut  if  else  while  for  in  return
struct  enum  trait  impl  import  from  pub  const
type  match  async  await  spawn  true  false  mod
break  continue  test  as
```

### Operators (by precedence, lowest to highest)

```
||                          Logical OR
&&                          Logical AND
== != < <= > >=             Comparison
|                           Bitwise OR
^                           Bitwise XOR
&                           Bitwise AND
<< >>                       Shift
+ -                         Additive
* / %                       Multiplicative
! - ~                       Unary (prefix)
. () [] |>                  Postfix / Call / Pipe
```

## Syntax Grammar (EBNF)

```ebnf
program         = item* ;

item            = function
                | struct_def
                | enum_def
                | trait_def
                | impl_def
                | import_stmt
                | const_def
                | type_alias
                | module_def
                | test_def ;

function        = ["pub"] ["async"] "fn" IDENT generic_params? "(" param_list? ")" ["->" type_expr] block ;

generic_params  = "<" generic_param ("," generic_param)* ">" ;
generic_param   = IDENT [":" type_expr ("+" type_expr)*] ;

param_list      = param ("," param)* ;
param           = IDENT [":" type_expr] ;

struct_def      = ["pub"] "struct" IDENT generic_params? "{" struct_field* "}" ;
struct_field    = ["pub"] IDENT ":" type_expr ","? ;

enum_def        = ["pub"] "enum" IDENT generic_params? "{" enum_variant* "}" ;
enum_variant    = IDENT ["(" type_expr ("," type_expr)* ")"] ","? ;

trait_def       = ["pub"] "trait" IDENT generic_params? "{" function* "}" ;

impl_def        = "impl" generic_params? [IDENT "for"] type_expr "{" function* "}" ;

import_stmt     = "import" "{" IDENT ("," IDENT)* "}" "from" STRING_LIT ;
                | "import" IDENT "from" STRING_LIT ;

const_def       = ["pub"] "const" IDENT [":" type_expr] "=" expression ;

type_alias      = ["pub"] "type" IDENT "=" type_expr ;

module_def      = ["pub"] "mod" IDENT "{" item* "}" ;

test_def        = "test" STRING_LIT block ;

(* Statements *)
block           = "{" statement* [expression] "}" ;

statement       = let_stmt
                | return_stmt
                | while_stmt
                | for_stmt
                | assign_stmt
                | break_stmt
                | continue_stmt
                | expression_stmt ;

let_stmt        = "let" ["mut"] pattern [":" type_expr] ["=" expression] ;
return_stmt     = "return" [expression] ;
while_stmt      = "while" expression block ;
for_stmt        = "for" IDENT "in" expression block ;
assign_stmt     = expression "=" expression ;
break_stmt      = "break" ;
continue_stmt   = "continue" ;
expression_stmt = expression ;

(* Expressions *)
expression      = pipe_expr ;
pipe_expr       = or_expr ("|>" or_expr)* ;
or_expr         = and_expr ("||" and_expr)* ;
and_expr        = compare_expr ("&&" compare_expr)* ;
compare_expr    = bitor_expr (("==" | "!=" | "<" | "<=" | ">" | ">=") bitor_expr)? ;
bitor_expr      = bitxor_expr ("|" bitxor_expr)* ;
bitxor_expr     = bitand_expr ("^" bitand_expr)* ;
bitand_expr     = shift_expr ("&" shift_expr)* ;
shift_expr      = add_expr (("<<" | ">>") add_expr)* ;
add_expr        = mul_expr (("+" | "-") mul_expr)* ;
mul_expr        = unary_expr (("*" | "/" | "%") unary_expr)* ;
unary_expr      = ("-" | "!" | "~") unary_expr | postfix_expr ;
postfix_expr    = primary_expr (call | index | field | method_call | ".await")* ;
call            = "(" arg_list? ")" ;
index           = "[" expression "]" ;
field           = "." IDENT ;
method_call     = "." IDENT "(" arg_list? ")" ;
arg_list        = expression ("," expression)* ;

primary_expr    = INT_LIT | FLOAT_LIT | STRING_LIT | CHAR_LIT
                | "true" | "false"
                | IDENT
                | "(" expression ")"
                | if_expr | match_expr | block | array_lit | lambda
                | struct_init | range_expr ;

if_expr         = "if" expression block ["else" (if_expr | block)] ;
match_expr      = "match" expression "{" match_arm* "}" ;
match_arm       = pattern ["if" expression] "=>" expression ","? ;
array_lit       = "[" (expression ("," expression)*)? "]" ;
lambda          = "|" param_list? "|" expression ;
struct_init     = IDENT "{" (IDENT ":" expression ("," IDENT ":" expression)*)? "}" ;
range_expr      = expression (".." | "..=") expression ;

(* Patterns *)
pattern         = IDENT
                | literal
                | "(" pattern ("," pattern)* ")"
                | IDENT "{" (IDENT [":" pattern] ("," IDENT [":" pattern])*)? "}"
                | IDENT "::" IDENT ["(" pattern ("," pattern)* ")"]
                | "_"
                | ".." ;

(* Type Expressions *)
type_expr       = IDENT
                | IDENT "<" type_expr ("," type_expr)* ">"
                | "[" type_expr "]"
                | "(" type_expr ("," type_expr)* ")"
                | "fn" "(" type_expr ("," type_expr)* ")" ["->" type_expr]
                | type_expr "?"
                | "&" ["mut"] type_expr ;
```

## Type System

### Primitive Types

| Type                      | Description                             |
| ------------------------- | --------------------------------------- |
| `int`                     | 64-bit signed integer (alias for `i64`) |
| `i8`, `i16`, `i32`, `i64` | Signed integers                         |
| `u8`, `u16`, `u32`, `u64` | Unsigned integers                       |
| `f32`, `f64`              | IEEE 754 floating point                 |
| `bool`                    | Boolean (`true` / `false`)              |
| `char`                    | Unicode scalar value                    |
| `string`                  | UTF-8 string                            |
| `void`                    | Unit / no value                         |

### Compound Types

| Type      | Syntax         | Example           |
| --------- | -------------- | ----------------- |
| Array     | `[T]`          | `[int]`           |
| Tuple     | `(T, U)`       | `(int, string)`   |
| Optional  | `T?`           | `int?`            |
| Reference | `&T`, `&mut T` | `&Point`          |
| Function  | `fn(T) -> U`   | `fn(int) -> bool` |
| Generic   | `Name<T>`      | `List<int>`       |

### Type Inference

KLIK uses bidirectional type inference based on Hindley-Milner unification:

```klik
let x = 42          // inferred as int
let y = 3.14        // inferred as f64
let z = "hello"     // inferred as string
let list = [1, 2, 3] // inferred as [int]
```

## Semantics

### Ownership & Mutability

- Variables are immutable by default
- Use `mut` for mutable bindings
- Parameters are passed by value (copy for primitives, move for compound types)

### Scoping

- Lexical scoping with block-level scope
- Functions, structs, enums visible at module level
- `pub` makes items visible outside their module

### Control Flow

- `if` is an expression (returns a value)
- `match` must be exhaustive
- `for` iterates over ranges and collections
- `while` for condition-based loops

### Error Handling

- Use `Result<T, E>` enum for fallible operations
- Pattern match on `Ok`/`Err` for handling
- No exceptions

### Concurrency

- `async fn` for asynchronous functions
- `.await` to await async results
- `spawn` to launch concurrent tasks
- Channel-based communication between tasks
