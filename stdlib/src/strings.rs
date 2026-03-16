// KLIK stdlib - Strings module

/// Check if string contains a substring
pub fn contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

/// Check if string starts with prefix
pub fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// Check if string ends with suffix
pub fn ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

/// Convert to uppercase
pub fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

/// Convert to lowercase
pub fn to_lower(s: &str) -> String {
    s.to_lowercase()
}

/// Trim whitespace from both ends
pub fn trim(s: &str) -> String {
    s.trim().to_string()
}

/// Trim whitespace from start
pub fn trim_start(s: &str) -> String {
    s.trim_start().to_string()
}

/// Trim whitespace from end
pub fn trim_end(s: &str) -> String {
    s.trim_end().to_string()
}

/// Split string by delimiter
pub fn split(s: &str, delimiter: &str) -> Vec<String> {
    s.split(delimiter).map(String::from).collect()
}

/// Join strings with separator
pub fn join(parts: &[String], separator: &str) -> String {
    parts.join(separator)
}

/// Replace occurrences
pub fn replace(s: &str, from: &str, to: &str) -> String {
    s.replace(from, to)
}

/// Replace first n occurrences
pub fn replacen(s: &str, from: &str, to: &str, count: usize) -> String {
    s.replacen(from, to, count)
}

/// Get character count
pub fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// Get byte length
pub fn byte_len(s: &str) -> usize {
    s.len()
}

/// Check if string is empty
pub fn is_empty(s: &str) -> bool {
    s.is_empty()
}

/// Repeat string n times
pub fn repeat(s: &str, n: usize) -> String {
    s.repeat(n)
}

/// Reverse a string
pub fn reverse(s: &str) -> String {
    s.chars().rev().collect()
}

/// Get a substring by char indices
pub fn substring(s: &str, start: usize, end: usize) -> String {
    s.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

/// Get character at index
pub fn char_at(s: &str, index: usize) -> Option<char> {
    s.chars().nth(index)
}

/// Find index of substring
pub fn index_of(s: &str, needle: &str) -> Option<usize> {
    s.find(needle)
}

/// Find last index of substring
pub fn last_index_of(s: &str, needle: &str) -> Option<usize> {
    s.rfind(needle)
}

/// Pad string on the left to reach target length
pub fn pad_start(s: &str, target_len: usize, pad_char: char) -> String {
    let char_count = s.chars().count();
    if char_count >= target_len {
        return s.to_string();
    }
    let padding: String = std::iter::repeat_n(pad_char, target_len - char_count).collect();
    format!("{}{}", padding, s)
}

/// Pad string on the right to reach target length
pub fn pad_end(s: &str, target_len: usize, pad_char: char) -> String {
    let char_count = s.chars().count();
    if char_count >= target_len {
        return s.to_string();
    }
    let padding: String = std::iter::repeat_n(pad_char, target_len - char_count).collect();
    format!("{}{}", s, padding)
}

/// Check if string contains only digits
pub fn is_numeric(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Check if string contains only alphabetic characters
pub fn is_alphabetic(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphabetic())
}

/// Check if string contains only alphanumeric characters
pub fn is_alphanumeric(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric())
}

/// Parse string to i64
pub fn parse_int(s: &str) -> Option<i64> {
    s.trim().parse::<i64>().ok()
}

/// Parse string to f64
pub fn parse_float(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

/// Convert integer to string
pub fn from_int(n: i64) -> String {
    n.to_string()
}

/// Convert float to string
pub fn from_float(n: f64) -> String {
    n.to_string()
}

/// Convert bool to string
pub fn from_bool(b: bool) -> String {
    b.to_string()
}
