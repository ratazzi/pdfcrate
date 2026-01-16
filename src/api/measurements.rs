//! Unit conversion utilities for measurements
//!
//! PDF uses points (pt) as the base unit: 72 points = 1 inch.
//!
//! # Example
//!
//! ```rust
//! use pdfcrate::prelude::*;
//!
//! let doc = Document::new();
//! let mut layout = LayoutDocument::with_margin(doc, Margin::all(Inch(0.5)));
//!
//! layout
//!     .move_down(Mm(10.0))
//!     .text("Hello")
//!     .move_down(Cm(1.5));
//! ```

/// Points per inch (PDF standard)
pub const POINTS_PER_INCH: f64 = 72.0;

/// Millimeters per inch
pub const MM_PER_INCH: f64 = 25.4;

/// A trait for types that can be converted to PDF points.
///
/// This allows functions to accept multiple unit types:
/// - `f64` - raw points (default)
/// - `Pt(f64)` - explicit points
/// - `Mm(f64)` - millimeters
/// - `Cm(f64)` - centimeters
/// - `Inch(f64)` - inches
pub trait Measurement: Copy {
    /// Convert to points
    fn to_pt(self) -> f64;
}

impl Measurement for f64 {
    #[inline]
    fn to_pt(self) -> f64 {
        self
    }
}

impl Measurement for i32 {
    #[inline]
    fn to_pt(self) -> f64 {
        self as f64
    }
}

/// Points (PDF base unit, 72 points = 1 inch)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pt(pub f64);

impl Measurement for Pt {
    #[inline]
    fn to_pt(self) -> f64 {
        self.0
    }
}

impl From<Pt> for f64 {
    fn from(pt: Pt) -> f64 {
        pt.0
    }
}

/// Inches (1 inch = 72 points)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inch(pub f64);

impl Measurement for Inch {
    #[inline]
    fn to_pt(self) -> f64 {
        self.0 * POINTS_PER_INCH
    }
}

impl From<Inch> for f64 {
    fn from(inch: Inch) -> f64 {
        inch.to_pt()
    }
}

/// Millimeters (25.4 mm = 1 inch = 72 points)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mm(pub f64);

impl Measurement for Mm {
    #[inline]
    fn to_pt(self) -> f64 {
        self.0 * POINTS_PER_INCH / MM_PER_INCH
    }
}

impl From<Mm> for f64 {
    fn from(mm: Mm) -> f64 {
        mm.to_pt()
    }
}

/// Centimeters (2.54 cm = 1 inch = 72 points)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cm(pub f64);

impl Measurement for Cm {
    #[inline]
    fn to_pt(self) -> f64 {
        self.0 * POINTS_PER_INCH / 2.54
    }
}

impl From<Cm> for f64 {
    fn from(cm: Cm) -> f64 {
        cm.to_pt()
    }
}

// Convenience functions

/// Convert millimeters to points
#[inline]
pub fn mm(millimeters: f64) -> f64 {
    Mm(millimeters).to_pt()
}

/// Convert centimeters to points
#[inline]
pub fn cm(centimeters: f64) -> f64 {
    Cm(centimeters).to_pt()
}

/// Convert inches to points
#[inline]
pub fn inch(inches: f64) -> f64 {
    Inch(inches).to_pt()
}

/// Convert points to millimeters
#[inline]
pub fn pt2mm(points: f64) -> f64 {
    points * MM_PER_INCH / POINTS_PER_INCH
}

/// Convert points to centimeters
#[inline]
pub fn pt2cm(points: f64) -> f64 {
    points * 2.54 / POINTS_PER_INCH
}

/// Convert points to inches
#[inline]
pub fn pt2inch(points: f64) -> f64 {
    points / POINTS_PER_INCH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversions() {
        assert_eq!(Inch(1.0).to_pt(), 72.0);
        assert_eq!(Inch(8.5).to_pt(), 612.0);
        assert!((Mm(25.4).to_pt() - 72.0).abs() < 0.001);
        assert!((Cm(2.54).to_pt() - 72.0).abs() < 0.001);
    }

    #[test]
    fn test_a4() {
        // A4: 210mm x 297mm
        assert!((Mm(210.0).to_pt() - 595.28).abs() < 0.01);
        assert!((Mm(297.0).to_pt() - 841.89).abs() < 0.01);
    }

    #[test]
    fn test_trait_usage() {
        fn accepts(m: impl Measurement) -> f64 {
            m.to_pt()
        }
        assert_eq!(accepts(72.0), 72.0);
        assert_eq!(accepts(Inch(1.0)), 72.0);
    }
}
