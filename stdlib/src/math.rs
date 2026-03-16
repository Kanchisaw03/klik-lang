// KLIK stdlib - Math module

/// Absolute value for i64
pub fn abs_i64(x: i64) -> i64 {
    x.abs()
}

/// Absolute value for f64
pub fn abs_f64(x: f64) -> f64 {
    x.abs()
}

/// Power: base^exp for integers
pub fn pow_i64(base: i64, exp: u32) -> i64 {
    base.pow(exp)
}

/// Power: base^exp for floats
pub fn pow_f64(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

/// Square root
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// Cube root
pub fn cbrt(x: f64) -> f64 {
    x.cbrt()
}

/// Floor
pub fn floor(x: f64) -> f64 {
    x.floor()
}

/// Ceil
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

/// Round
pub fn round(x: f64) -> f64 {
    x.round()
}

/// Minimum of two i64
pub fn min_i64(a: i64, b: i64) -> i64 {
    a.min(b)
}

/// Maximum of two i64
pub fn max_i64(a: i64, b: i64) -> i64 {
    a.max(b)
}

/// Minimum of two f64
pub fn min_f64(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Maximum of two f64
pub fn max_f64(a: f64, b: f64) -> f64 {
    a.max(b)
}

/// Clamp i64
pub fn clamp_i64(val: i64, min: i64, max: i64) -> i64 {
    val.clamp(min, max)
}

/// Clamp f64
pub fn clamp_f64(val: f64, min: f64, max: f64) -> f64 {
    val.clamp(min, max)
}

/// Natural logarithm
pub fn ln(x: f64) -> f64 {
    x.ln()
}

/// Base-10 logarithm
pub fn log10(x: f64) -> f64 {
    x.log10()
}

/// Base-2 logarithm
pub fn log2(x: f64) -> f64 {
    x.log2()
}

/// Logarithm with arbitrary base
pub fn log(x: f64, base: f64) -> f64 {
    x.log(base)
}

/// Sine
pub fn sin(x: f64) -> f64 {
    x.sin()
}

/// Cosine  
pub fn cos(x: f64) -> f64 {
    x.cos()
}

/// Tangent
pub fn tan(x: f64) -> f64 {
    x.tan()
}

/// Arcsine
pub fn asin(x: f64) -> f64 {
    x.asin()
}

/// Arccosine
pub fn acos(x: f64) -> f64 {
    x.acos()
}

/// Arctangent
pub fn atan(x: f64) -> f64 {
    x.atan()
}

/// Two-argument arctangent
pub fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

/// Euler's number e
pub const E: f64 = std::f64::consts::E;

/// Pi
pub const PI: f64 = std::f64::consts::PI;

/// Tau (2*Pi)
pub const TAU: f64 = std::f64::consts::TAU;

/// Infinity
pub const INFINITY: f64 = f64::INFINITY;

/// Negative infinity
pub const NEG_INFINITY: f64 = f64::NEG_INFINITY;

/// NaN
pub const NAN: f64 = f64::NAN;

/// Check if NaN
pub fn is_nan(x: f64) -> bool {
    x.is_nan()
}

/// Check if finite
pub fn is_finite(x: f64) -> bool {
    x.is_finite()
}

/// Check if infinite
pub fn is_infinite(x: f64) -> bool {
    x.is_infinite()
}

/// GCD using Euclidean algorithm
pub fn gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// LCM
pub fn lcm(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 {
        0
    } else {
        (a / gcd(a, b)) * b
    }
}
