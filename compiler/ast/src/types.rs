// KLIK Type Representation (used by AST and type checker)

use serde::{Deserialize, Serialize};
use std::fmt;

/// Resolved types used after type checking
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Type {
    // Primitive types
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float32,
    Float64,
    Bool,
    Char,
    String,
    Void,
    Never,

    // Compound types
    Array(Box<Type>, Option<usize>),
    Tuple(Vec<Type>),
    Optional(Box<Type>),
    Reference(Box<Type>, bool), // (inner, mutable)

    // User-defined types
    Struct(String, Vec<Type>), // (name, generic_args)
    Enum(String, Vec<Type>),
    Trait(String),

    // Function type
    Function(Vec<Type>, Box<Type>), // (params, return)

    // Type variables for inference
    TypeVar(u64),

    // Generic parameter
    Generic(String),

    // Error type for recovery
    Error,
}

impl Type {
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::Int
                | Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64
                | Type::Uint
                | Type::Uint8
                | Type::Uint16
                | Type::Uint32
                | Type::Uint64
                | Type::Float32
                | Type::Float64
        )
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::Int
                | Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64
                | Type::Uint
                | Type::Uint8
                | Type::Uint16
                | Type::Uint32
                | Type::Uint64
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float32 | Type::Float64)
    }

    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            Type::Int | Type::Int8 | Type::Int16 | Type::Int32 | Type::Int64
        )
    }

    pub fn size_bits(&self) -> Option<u32> {
        match self {
            Type::Int8 | Type::Uint8 | Type::Bool => Some(8),
            Type::Int16 | Type::Uint16 => Some(16),
            Type::Int32 | Type::Uint32 | Type::Float32 | Type::Char => Some(32),
            Type::Int64 | Type::Uint64 | Type::Float64 => Some(64),
            Type::Int | Type::Uint => Some(64), // Default to 64-bit
            _ => None,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Int8 => write!(f, "i8"),
            Type::Int16 => write!(f, "i16"),
            Type::Int32 => write!(f, "i32"),
            Type::Int64 => write!(f, "i64"),
            Type::Uint => write!(f, "uint"),
            Type::Uint8 => write!(f, "u8"),
            Type::Uint16 => write!(f, "u16"),
            Type::Uint32 => write!(f, "u32"),
            Type::Uint64 => write!(f, "u64"),
            Type::Float32 => write!(f, "f32"),
            Type::Float64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::Char => write!(f, "char"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Never => write!(f, "never"),
            Type::Array(inner, size) => {
                if let Some(s) = size {
                    write!(f, "[{inner}; {s}]")
                } else {
                    write!(f, "[{inner}]")
                }
            }
            Type::Tuple(elems) => {
                write!(f, "(")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{e}")?;
                }
                write!(f, ")")
            }
            Type::Optional(inner) => write!(f, "{inner}?"),
            Type::Reference(inner, mutable) => {
                if *mutable {
                    write!(f, "&mut {inner}")
                } else {
                    write!(f, "&{inner}")
                }
            }
            Type::Struct(name, args) | Type::Enum(name, args) => {
                write!(f, "{name}")?;
                if !args.is_empty() {
                    write!(f, "<")?;
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{a}")?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::Trait(name) => write!(f, "trait {name}"),
            Type::Function(params, ret) => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {ret}")
            }
            Type::TypeVar(id) => write!(f, "?T{id}"),
            Type::Generic(name) => write!(f, "{name}"),
            Type::Error => write!(f, "<error>"),
        }
    }
}
