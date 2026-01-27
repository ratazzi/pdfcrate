//! PDF Number object
//!
//! PDF supports two numeric types: integers and real numbers.

use std::fmt;

/// A PDF number (integer or real)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PdfNumber {
    /// Integer number
    Integer(i64),
    /// Real (floating-point) number
    Real(f64),
}

impl PdfNumber {
    /// Creates a new integer number
    pub fn integer(value: i64) -> Self {
        PdfNumber::Integer(value)
    }

    /// Creates a new real number
    pub fn real(value: f64) -> Self {
        PdfNumber::Real(value)
    }

    /// Returns the value as an integer, truncating if real
    pub fn as_integer(&self) -> i64 {
        match self {
            PdfNumber::Integer(i) => *i,
            PdfNumber::Real(r) => *r as i64,
        }
    }

    /// Returns the value as a real number
    pub fn as_real(&self) -> f64 {
        match self {
            PdfNumber::Integer(i) => *i as f64,
            PdfNumber::Real(r) => *r,
        }
    }

    /// Returns true if this is an integer
    pub fn is_integer(&self) -> bool {
        matches!(self, PdfNumber::Integer(_))
    }

    /// Returns true if this is a real number
    pub fn is_real(&self) -> bool {
        matches!(self, PdfNumber::Real(_))
    }
}

impl fmt::Display for PdfNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfNumber::Integer(i) => write!(f, "{}", i),
            PdfNumber::Real(r) => {
                // Format real numbers efficiently
                if r.fract() == 0.0 && r.abs() < i64::MAX as f64 {
                    write!(f, "{:.1}", r)
                } else {
                    // Use enough precision but avoid excessive digits
                    let s = format!("{:.6}", r);
                    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
                    if trimmed.is_empty() || trimmed == "-" {
                        write!(f, "0")
                    } else {
                        write!(f, "{}", trimmed)
                    }
                }
            }
        }
    }
}

impl From<i32> for PdfNumber {
    fn from(value: i32) -> Self {
        PdfNumber::Integer(value as i64)
    }
}

impl From<i64> for PdfNumber {
    fn from(value: i64) -> Self {
        PdfNumber::Integer(value)
    }
}

impl From<f32> for PdfNumber {
    fn from(value: f32) -> Self {
        PdfNumber::Real(value as f64)
    }
}

impl From<f64> for PdfNumber {
    fn from(value: f64) -> Self {
        PdfNumber::Real(value)
    }
}

impl From<usize> for PdfNumber {
    fn from(value: usize) -> Self {
        PdfNumber::Integer(value as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer() {
        let n = PdfNumber::integer(42);
        assert!(n.is_integer());
        assert_eq!(n.as_integer(), 42);
        assert_eq!(format!("{}", n), "42");
    }

    #[test]
    fn test_real() {
        let n = PdfNumber::real(1.23456);
        assert!(n.is_real());
        assert!((n.as_real() - 1.23456).abs() < 0.00001);
        assert_eq!(format!("{}", n), "1.23456");
    }

    #[test]
    fn test_real_without_fraction() {
        let n = PdfNumber::real(42.0);
        assert_eq!(format!("{}", n), "42.0");
    }

    #[test]
    fn test_conversion() {
        let n: PdfNumber = 42i32.into();
        assert!(n.is_integer());

        let n: PdfNumber = 2.73f64.into();
        assert!(n.is_real());
    }
}
