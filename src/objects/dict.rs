//! PDF Dictionary object
//!
//! Dictionaries are associative arrays mapping names to objects.
//! They are enclosed in double angle brackets << >>.

use std::collections::HashMap;
use std::fmt;

use super::{PdfName, PdfObject, PdfRef};

/// A PDF dictionary
///
/// Dictionaries are the main building blocks of PDF documents.
/// They map name objects to arbitrary PDF objects.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PdfDict {
    entries: HashMap<PdfName, PdfObject>,
    /// Preserves insertion order for consistent serialization
    order: Vec<PdfName>,
}

impl PdfDict {
    /// Creates a new empty dictionary
    pub fn new() -> Self {
        PdfDict {
            entries: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Creates a dictionary with the given capacity
    pub fn with_capacity(capacity: usize) -> Self {
        PdfDict {
            entries: HashMap::with_capacity(capacity),
            order: Vec::with_capacity(capacity),
        }
    }

    /// Returns the number of entries in the dictionary
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the dictionary is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Inserts a key-value pair into the dictionary
    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: Into<PdfName>,
        V: Into<PdfObject>,
    {
        let key = key.into();
        let value = value.into();

        if !self.entries.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.entries.insert(key, value);
    }

    /// Sets a value using a string key
    pub fn set<V: Into<PdfObject>>(&mut self, key: &str, value: V) {
        self.insert(PdfName::new(key), value);
    }

    /// Gets a reference to the value for the given key
    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<&PdfObject> {
        self.entries.get(&PdfName::new(key.as_ref()))
    }

    /// Gets a mutable reference to the value for the given key
    pub fn get_mut<K: AsRef<str>>(&mut self, key: K) -> Option<&mut PdfObject> {
        self.entries.get_mut(&PdfName::new(key.as_ref()))
    }

    /// Removes a key from the dictionary
    pub fn remove<K: AsRef<str>>(&mut self, key: K) -> Option<PdfObject> {
        let key = PdfName::new(key.as_ref());
        self.order.retain(|k| k != &key);
        self.entries.remove(&key)
    }

    /// Returns true if the dictionary contains the given key
    pub fn contains_key<K: AsRef<str>>(&self, key: K) -> bool {
        self.entries.contains_key(&PdfName::new(key.as_ref()))
    }

    /// Returns an iterator over the key-value pairs in insertion order
    pub fn iter(&self) -> impl Iterator<Item = (&PdfName, &PdfObject)> {
        self.order
            .iter()
            .filter_map(|k| self.entries.get(k).map(|v| (k, v)))
    }

    /// Returns an iterator over the keys in insertion order
    pub fn keys(&self) -> impl Iterator<Item = &PdfName> {
        self.order.iter()
    }

    /// Returns an iterator over the values in insertion order
    pub fn values(&self) -> impl Iterator<Item = &PdfObject> {
        self.order.iter().filter_map(|k| self.entries.get(k))
    }

    // Convenience methods for common PDF dictionary patterns

    /// Gets a value as an integer
    pub fn get_integer<K: AsRef<str>>(&self, key: K) -> Option<i64> {
        self.get(key).and_then(|v| v.as_integer())
    }

    /// Gets a value as a real number
    pub fn get_real<K: AsRef<str>>(&self, key: K) -> Option<f64> {
        self.get(key).and_then(|v| v.as_real())
    }

    /// Gets a value as a name
    pub fn get_name<K: AsRef<str>>(&self, key: K) -> Option<&PdfName> {
        self.get(key).and_then(|v| v.as_name())
    }

    /// Gets a value as a reference
    pub fn get_reference<K: AsRef<str>>(&self, key: K) -> Option<PdfRef> {
        self.get(key).and_then(|v| v.as_reference())
    }

    /// Alias for get_reference
    pub fn get_ref<K: AsRef<str>>(&self, key: K) -> Option<PdfRef> {
        self.get_reference(key)
    }

    /// Gets a value as a dictionary
    pub fn get_dict<K: AsRef<str>>(&self, key: K) -> Option<&PdfDict> {
        self.get(key).and_then(|v| v.as_dict())
    }

    /// Gets a value as an array
    pub fn get_array<K: AsRef<str>>(&self, key: K) -> Option<&super::PdfArray> {
        self.get(key).and_then(|v| v.as_array())
    }

    /// Gets the /Type entry as a name string
    pub fn get_type(&self) -> Option<&str> {
        self.get_name("Type").map(|n| n.as_str())
    }

    /// Gets the /Subtype entry as a name string
    pub fn get_subtype(&self) -> Option<&str> {
        self.get_name("Subtype").map(|n| n.as_str())
    }
}

impl fmt::Display for PdfDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<<")?;
        for (i, (key, value)) in self.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{} {}", key, value)?;
        }
        write!(f, ">>")
    }
}

/// Creates a PDF dictionary from the given key-value pairs
#[macro_export]
macro_rules! pdf_dict {
    () => {
        $crate::objects::PdfDict::new()
    };
    ($($key:expr => $value:expr),+ $(,)?) => {{
        let mut dict = $crate::objects::PdfDict::new();
        $(dict.set($key, $value);)+
        dict
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dict_creation() {
        let dict = PdfDict::new();
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn test_dict_insert() {
        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::new("Page")));
        dict.set("Count", PdfObject::Integer(5));

        assert_eq!(dict.len(), 2);
        assert!(dict.contains_key("Type"));
        assert!(dict.contains_key("Count"));
    }

    #[test]
    fn test_dict_get() {
        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::new("Page")));

        let value = dict.get("Type");
        assert!(value.is_some());

        let name = dict.get_name("Type");
        assert_eq!(name.map(|n| n.as_str()), Some("Page"));
    }

    #[test]
    fn test_dict_display() {
        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::new("Page")));
        dict.set("Count", PdfObject::Integer(5));

        let s = format!("{}", dict);
        assert!(s.starts_with("<<"));
        assert!(s.ends_with(">>"));
        assert!(s.contains("/Type /Page"));
        assert!(s.contains("/Count 5"));
    }

    #[test]
    fn test_dict_order_preserved() {
        let mut dict = PdfDict::new();
        dict.set("First", PdfObject::Integer(1));
        dict.set("Second", PdfObject::Integer(2));
        dict.set("Third", PdfObject::Integer(3));

        let keys: Vec<&str> = dict.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, vec!["First", "Second", "Third"]);
    }

    #[test]
    fn test_get_type() {
        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::new("Catalog")));

        assert_eq!(dict.get_type(), Some("Catalog"));
    }
}
