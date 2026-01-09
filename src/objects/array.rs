//! PDF Array object
//!
//! Arrays are one-dimensional collections of objects enclosed in square brackets.

use std::fmt;
use std::ops::{Index, IndexMut};

use super::PdfObject;

/// A PDF array
///
/// Arrays can contain any mix of PDF objects and can be nested.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PdfArray(Vec<PdfObject>);

impl PdfArray {
    /// Creates a new empty array
    pub fn new() -> Self {
        PdfArray(Vec::new())
    }

    /// Creates an array with the given capacity
    pub fn with_capacity(capacity: usize) -> Self {
        PdfArray(Vec::with_capacity(capacity))
    }

    /// Creates an array from a vector of objects
    pub fn from_vec(objects: Vec<PdfObject>) -> Self {
        PdfArray(objects)
    }

    /// Returns the number of elements in the array
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the array is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Pushes an object onto the end of the array
    pub fn push<T: Into<PdfObject>>(&mut self, object: T) {
        self.0.push(object.into());
    }

    /// Gets a reference to the object at the given index
    pub fn get(&self, index: usize) -> Option<&PdfObject> {
        self.0.get(index)
    }

    /// Gets a mutable reference to the object at the given index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PdfObject> {
        self.0.get_mut(index)
    }

    /// Returns an iterator over the objects
    pub fn iter(&self) -> impl Iterator<Item = &PdfObject> {
        self.0.iter()
    }

    /// Returns a mutable iterator over the objects
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PdfObject> {
        self.0.iter_mut()
    }

    /// Returns the underlying vector
    pub fn into_vec(self) -> Vec<PdfObject> {
        self.0
    }

    /// Returns a slice of the underlying vector
    pub fn as_slice(&self) -> &[PdfObject] {
        &self.0
    }

    // Convenience methods for type-specific access

    /// Gets an integer at the given index
    pub fn get_integer(&self, index: usize) -> Option<i64> {
        self.get(index).and_then(|v| v.as_integer())
    }

    /// Gets a real number at the given index
    pub fn get_real(&self, index: usize) -> Option<f64> {
        self.get(index).and_then(|v| v.as_real())
    }

    /// Gets a name at the given index
    pub fn get_name(&self, index: usize) -> Option<&super::PdfName> {
        self.get(index).and_then(|v| v.as_name())
    }

    /// Gets a reference at the given index
    pub fn get_reference(&self, index: usize) -> Option<super::PdfRef> {
        self.get(index).and_then(|v| v.as_reference())
    }

    /// Gets a dictionary at the given index
    pub fn get_dict(&self, index: usize) -> Option<&super::PdfDict> {
        self.get(index).and_then(|v| v.as_dict())
    }

    /// Gets an array at the given index
    pub fn get_array(&self, index: usize) -> Option<&PdfArray> {
        self.get(index).and_then(|v| v.as_array())
    }
}

impl Index<usize> for PdfArray {
    type Output = PdfObject;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for PdfArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl fmt::Display for PdfArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, obj) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", obj)?;
        }
        write!(f, "]")
    }
}

impl<T: Into<PdfObject>> FromIterator<T> for PdfArray {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        PdfArray(iter.into_iter().map(Into::into).collect())
    }
}

impl IntoIterator for PdfArray {
    type Item = PdfObject;
    type IntoIter = std::vec::IntoIter<PdfObject>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a PdfArray {
    type Item = &'a PdfObject;
    type IntoIter = std::slice::Iter<'a, PdfObject>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<Vec<PdfObject>> for PdfArray {
    fn from(v: Vec<PdfObject>) -> Self {
        PdfArray(v)
    }
}

/// Creates a PDF array from the given elements
#[macro_export]
macro_rules! pdf_array {
    () => {
        $crate::objects::PdfArray::new()
    };
    ($($x:expr),+ $(,)?) => {{
        let mut array = $crate::objects::PdfArray::new();
        $(array.push($x);)+
        array
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_creation() {
        let arr = PdfArray::new();
        assert!(arr.is_empty());
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn test_array_push() {
        let mut arr = PdfArray::new();
        arr.push(PdfObject::Integer(1));
        arr.push(PdfObject::Integer(2));
        arr.push(PdfObject::Integer(3));

        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], PdfObject::Integer(1));
    }

    #[test]
    fn test_array_display() {
        let mut arr = PdfArray::new();
        arr.push(PdfObject::Integer(1));
        arr.push(PdfObject::Integer(2));
        arr.push(PdfObject::Integer(3));

        assert_eq!(format!("{}", arr), "[1 2 3]");
    }

    #[test]
    fn test_array_display_empty() {
        let arr = PdfArray::new();
        assert_eq!(format!("{}", arr), "[]");
    }

    #[test]
    fn test_array_iteration() {
        let arr: PdfArray = vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
            PdfObject::Integer(3),
        ]
        .into();

        let sum: i64 = arr.iter().filter_map(|o| o.as_integer()).sum();
        assert_eq!(sum, 6);
    }
}
