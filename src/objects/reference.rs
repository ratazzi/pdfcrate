//! PDF Reference object
//!
//! References are used to refer to indirect objects in a PDF document.
//! Format: object_number generation_number R (e.g., "1 0 R")

use std::fmt;

/// A PDF indirect object reference
///
/// References point to objects stored elsewhere in the PDF file.
/// They consist of an object number and a generation number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PdfRef {
    /// Object number (unique identifier)
    pub object_number: u32,
    /// Generation number (for versioning, usually 0)
    pub generation: u16,
}

impl PdfRef {
    /// Creates a new reference with generation 0
    pub fn new(object_number: u32) -> Self {
        PdfRef {
            object_number,
            generation: 0,
        }
    }

    /// Creates a new reference with specified generation
    pub fn with_generation(object_number: u32, generation: u16) -> Self {
        PdfRef {
            object_number,
            generation,
        }
    }

    /// Returns the object number
    pub fn object_number(&self) -> u32 {
        self.object_number
    }

    /// Returns the generation number
    pub fn generation(&self) -> u16 {
        self.generation
    }
}

impl fmt::Display for PdfRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.object_number, self.generation)
    }
}

impl From<u32> for PdfRef {
    fn from(object_number: u32) -> Self {
        PdfRef::new(object_number)
    }
}

impl From<(u32, u16)> for PdfRef {
    fn from((object_number, generation): (u32, u16)) -> Self {
        PdfRef::with_generation(object_number, generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reference_creation() {
        let r = PdfRef::new(1);
        assert_eq!(r.object_number(), 1);
        assert_eq!(r.generation(), 0);
    }

    #[test]
    fn test_reference_with_generation() {
        let r = PdfRef::with_generation(5, 2);
        assert_eq!(r.object_number(), 5);
        assert_eq!(r.generation(), 2);
    }

    #[test]
    fn test_reference_display() {
        let r = PdfRef::new(1);
        assert_eq!(format!("{}", r), "1 0 R");

        let r = PdfRef::with_generation(5, 2);
        assert_eq!(format!("{}", r), "5 2 R");
    }

    #[test]
    fn test_reference_equality() {
        let r1 = PdfRef::new(1);
        let r2 = PdfRef::new(1);
        let r3 = PdfRef::new(2);

        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
    }

    #[test]
    fn test_reference_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(PdfRef::new(1));
        set.insert(PdfRef::new(2));
        set.insert(PdfRef::new(1)); // Duplicate

        assert_eq!(set.len(), 2);
    }
}
