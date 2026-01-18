//! PDF object types
//!
//! This module implements the core PDF object types as defined in the PDF specification.
//! PDF documents are built from these basic object types.

mod array;
mod dict;
mod name;
mod number;
mod reference;
mod stream;
mod string;

pub use array::PdfArray;
pub use dict::PdfDict;
pub use name::PdfName;
pub use number::PdfNumber;
pub use reference::PdfRef;
pub use stream::PdfStream;
pub use string::{PdfHexString, PdfString};

/// Formats a floating-point number for PDF output using ryu for optimal precision.
///
/// This produces the shortest decimal representation that round-trips correctly,
/// following PDF spec requirements for real numbers (ISO 32000-1:2008 Section 7.3.3).
///
/// For values that are exactly integers, returns integer format without decimal point.
/// For extreme values (< 1e-6 or >= 1e12), falls back to standard formatting.
pub fn format_real(value: f64) -> String {
    // Handle special cases
    if value.is_nan() {
        return "0".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            "999999999".to_string()
        } else {
            "-999999999".to_string()
        };
    }

    // For exact integers, use integer format
    if value.fract() == 0.0 && value.abs() < i64::MAX as f64 {
        return format!("{}", value as i64);
    }

    // Use ryu for optimal float formatting
    // Only use ryu for "normal" range values; extreme values use fallback
    if value == 0.0 || (value.abs() > 1e-6 && value.abs() < 1e12) {
        let mut buffer = ryu::Buffer::new();
        let formatted = buffer.format(value);
        // Remove unnecessary trailing zeros after decimal point
        if formatted.contains('.') && !formatted.contains('e') && !formatted.contains('E') {
            formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            formatted.to_string()
        }
    } else {
        // Fallback for extreme values
        let s = format!("{:.6}", value);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

use std::fmt;

/// A PDF object that can be any of the basic PDF types
#[derive(Debug, Clone, PartialEq)]
pub enum PdfObject {
    /// Null object
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer number
    Integer(i64),
    /// Real number
    Real(f64),
    /// Name object (e.g., /Type)
    Name(PdfName),
    /// Literal string
    String(PdfString),
    /// Hexadecimal string
    HexString(PdfHexString),
    /// Array of objects
    Array(PdfArray),
    /// Dictionary
    Dict(PdfDict),
    /// Stream (dictionary + data)
    Stream(PdfStream),
    /// Indirect reference (e.g., 1 0 R)
    Reference(PdfRef),
}

impl PdfObject {
    /// Returns true if this is a null object
    pub fn is_null(&self) -> bool {
        matches!(self, PdfObject::Null)
    }

    /// Returns the boolean value if this is a Bool, None otherwise
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfObject::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the integer value if this is an Integer, None otherwise
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            PdfObject::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the real value if this is a Real or Integer, None otherwise
    pub fn as_real(&self) -> Option<f64> {
        match self {
            PdfObject::Real(r) => Some(*r),
            PdfObject::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Returns a reference to the Name if this is a Name, None otherwise
    pub fn as_name(&self) -> Option<&PdfName> {
        match self {
            PdfObject::Name(n) => Some(n),
            _ => None,
        }
    }

    /// Returns a reference to the String if this is a String, None otherwise
    pub fn as_string(&self) -> Option<&PdfString> {
        match self {
            PdfObject::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns a reference to the Array if this is an Array, None otherwise
    pub fn as_array(&self) -> Option<&PdfArray> {
        match self {
            PdfObject::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Returns a mutable reference to the Array if this is an Array, None otherwise
    pub fn as_array_mut(&mut self) -> Option<&mut PdfArray> {
        match self {
            PdfObject::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Returns a reference to the Dict if this is a Dict, None otherwise
    pub fn as_dict(&self) -> Option<&PdfDict> {
        match self {
            PdfObject::Dict(d) => Some(d),
            _ => None,
        }
    }

    /// Returns a mutable reference to the Dict if this is a Dict, None otherwise
    pub fn as_dict_mut(&mut self) -> Option<&mut PdfDict> {
        match self {
            PdfObject::Dict(d) => Some(d),
            _ => None,
        }
    }

    /// Returns a reference to the Stream if this is a Stream, None otherwise
    pub fn as_stream(&self) -> Option<&PdfStream> {
        match self {
            PdfObject::Stream(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the Reference if this is a Reference, None otherwise
    pub fn as_reference(&self) -> Option<PdfRef> {
        match self {
            PdfObject::Reference(r) => Some(*r),
            _ => None,
        }
    }

    /// Returns the type name of this object
    pub fn type_name(&self) -> &'static str {
        match self {
            PdfObject::Null => "null",
            PdfObject::Bool(_) => "boolean",
            PdfObject::Integer(_) => "integer",
            PdfObject::Real(_) => "real",
            PdfObject::Name(_) => "name",
            PdfObject::String(_) => "string",
            PdfObject::HexString(_) => "hexstring",
            PdfObject::Array(_) => "array",
            PdfObject::Dict(_) => "dictionary",
            PdfObject::Stream(_) => "stream",
            PdfObject::Reference(_) => "reference",
        }
    }
}

impl fmt::Display for PdfObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfObject::Null => write!(f, "null"),
            PdfObject::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            PdfObject::Integer(i) => write!(f, "{}", i),
            PdfObject::Real(r) => {
                // Format real numbers without unnecessary trailing zeros
                if r.fract() == 0.0 {
                    write!(f, "{:.1}", r)
                } else {
                    write!(f, "{}", r)
                }
            }
            PdfObject::Name(n) => write!(f, "{}", n),
            PdfObject::String(s) => write!(f, "{}", s),
            PdfObject::HexString(s) => write!(f, "{}", s),
            PdfObject::Array(a) => write!(f, "{}", a),
            PdfObject::Dict(d) => write!(f, "{}", d),
            PdfObject::Stream(s) => write!(f, "{}", s),
            PdfObject::Reference(r) => write!(f, "{}", r),
        }
    }
}

impl From<bool> for PdfObject {
    fn from(b: bool) -> Self {
        PdfObject::Bool(b)
    }
}

impl From<i32> for PdfObject {
    fn from(i: i32) -> Self {
        PdfObject::Integer(i as i64)
    }
}

impl From<i64> for PdfObject {
    fn from(i: i64) -> Self {
        PdfObject::Integer(i)
    }
}

impl From<f64> for PdfObject {
    fn from(r: f64) -> Self {
        PdfObject::Real(r)
    }
}

impl From<f32> for PdfObject {
    fn from(r: f32) -> Self {
        PdfObject::Real(r as f64)
    }
}

impl From<PdfName> for PdfObject {
    fn from(n: PdfName) -> Self {
        PdfObject::Name(n)
    }
}

impl From<PdfString> for PdfObject {
    fn from(s: PdfString) -> Self {
        PdfObject::String(s)
    }
}

impl From<PdfHexString> for PdfObject {
    fn from(s: PdfHexString) -> Self {
        PdfObject::HexString(s)
    }
}

impl From<PdfArray> for PdfObject {
    fn from(a: PdfArray) -> Self {
        PdfObject::Array(a)
    }
}

impl From<PdfDict> for PdfObject {
    fn from(d: PdfDict) -> Self {
        PdfObject::Dict(d)
    }
}

impl From<PdfStream> for PdfObject {
    fn from(s: PdfStream) -> Self {
        PdfObject::Stream(s)
    }
}

impl From<PdfRef> for PdfObject {
    fn from(r: PdfRef) -> Self {
        PdfObject::Reference(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_object_types() {
        assert_eq!(PdfObject::Null.type_name(), "null");
        assert_eq!(PdfObject::Bool(true).type_name(), "boolean");
        assert_eq!(PdfObject::Integer(42).type_name(), "integer");
        assert_eq!(PdfObject::Real(3.14).type_name(), "real");
    }

    #[test]
    fn test_pdf_object_display() {
        assert_eq!(format!("{}", PdfObject::Null), "null");
        assert_eq!(format!("{}", PdfObject::Bool(true)), "true");
        assert_eq!(format!("{}", PdfObject::Bool(false)), "false");
        assert_eq!(format!("{}", PdfObject::Integer(42)), "42");
    }

    #[test]
    fn test_format_real_integers() {
        // Exact integers should not have decimal point
        assert_eq!(format_real(0.0), "0");
        assert_eq!(format_real(1.0), "1");
        assert_eq!(format_real(-1.0), "-1");
        assert_eq!(format_real(42.0), "42");
        assert_eq!(format_real(1000000.0), "1000000");
    }

    #[test]
    fn test_format_real_decimals() {
        // Decimals should be formatted optimally
        assert_eq!(format_real(0.5), "0.5");
        assert_eq!(format_real(3.14), "3.14");
        assert_eq!(format_real(-2.5), "-2.5");
        // Should not have trailing zeros
        let result = format_real(1.50);
        assert!(!result.ends_with('0') || result == "0", "Got: {}", result);
    }

    #[test]
    fn test_format_real_precision() {
        // High precision values should round-trip correctly
        let value = 0.123456789;
        let formatted = format_real(value);
        let parsed: f64 = formatted.parse().unwrap();
        assert_eq!(
            value, parsed,
            "Value should round-trip: {} -> {}",
            value, formatted
        );
    }

    #[test]
    fn test_format_real_special_values() {
        // NaN should become 0
        assert_eq!(format_real(f64::NAN), "0");
        // Infinity should become large number
        assert_eq!(format_real(f64::INFINITY), "999999999");
        assert_eq!(format_real(f64::NEG_INFINITY), "-999999999");
    }

    #[test]
    fn test_format_real_small_values() {
        // Very small values
        let result = format_real(0.001);
        assert!(
            result.contains("0.001") || result == "0.001",
            "Got: {}",
            result
        );
    }
}
