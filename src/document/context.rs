//! PDF Context
//!
//! Manages the collection of indirect objects in a PDF document.

use std::collections::HashMap;

use crate::objects::{PdfObject, PdfRef};

/// PDF Context
///
/// Maintains the collection of indirect objects and manages object numbering.
#[derive(Debug, Default)]
pub struct PdfContext {
    /// All indirect objects
    objects: HashMap<PdfRef, PdfObject>,
    /// Next available object number
    next_object_number: u32,
}

impl PdfContext {
    /// Creates a new empty context
    pub fn new() -> Self {
        PdfContext {
            objects: HashMap::new(),
            next_object_number: 1, // Object 0 is reserved
        }
    }

    /// Allocates a new object reference
    pub fn alloc_ref(&mut self) -> PdfRef {
        let ref_id = PdfRef::new(self.next_object_number);
        self.next_object_number += 1;
        ref_id
    }

    /// Registers an object and returns its reference
    pub fn register(&mut self, object: PdfObject) -> PdfRef {
        let ref_id = self.alloc_ref();
        self.objects.insert(ref_id, object);
        ref_id
    }

    /// Assigns an object to a specific reference
    pub fn assign(&mut self, ref_id: PdfRef, object: PdfObject) {
        self.objects.insert(ref_id, object);
    }

    /// Looks up an object by reference
    pub fn lookup(&self, ref_id: PdfRef) -> Option<&PdfObject> {
        self.objects.get(&ref_id)
    }

    /// Looks up an object by reference and returns a mutable reference
    pub fn lookup_mut(&mut self, ref_id: PdfRef) -> Option<&mut PdfObject> {
        self.objects.get_mut(&ref_id)
    }

    /// Removes an object
    pub fn delete(&mut self, ref_id: PdfRef) -> Option<PdfObject> {
        self.objects.remove(&ref_id)
    }

    /// Returns the number of objects
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Returns true if there are no objects
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Returns an iterator over all (ref, object) pairs
    pub fn iter(&self) -> impl Iterator<Item = (&PdfRef, &PdfObject)> {
        self.objects.iter()
    }

    /// Returns all objects as a vector of (ref, object) pairs
    pub fn to_vec(&self) -> Vec<(PdfRef, PdfObject)> {
        self.objects
            .iter()
            .map(|(&r, o)| (r, o.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::PdfName;

    #[test]
    fn test_context_alloc() {
        let mut ctx = PdfContext::new();
        let ref1 = ctx.alloc_ref();
        let ref2 = ctx.alloc_ref();

        assert_eq!(ref1.object_number(), 1);
        assert_eq!(ref2.object_number(), 2);
    }

    #[test]
    fn test_context_register() {
        let mut ctx = PdfContext::new();
        let obj = PdfObject::Name(PdfName::new("Test"));
        let ref_id = ctx.register(obj.clone());

        assert_eq!(ctx.lookup(ref_id), Some(&obj));
    }

    #[test]
    fn test_context_assign() {
        let mut ctx = PdfContext::new();
        let ref_id = ctx.alloc_ref();
        let obj = PdfObject::Integer(42);

        ctx.assign(ref_id, obj.clone());
        assert_eq!(ctx.lookup(ref_id), Some(&obj));
    }
}
