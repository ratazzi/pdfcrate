//! PDF Document Loader
//!
//! This module handles loading and parsing existing PDF documents.

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::objects::{PdfDict, PdfObject, PdfRef, PdfStream};
use crate::parser::{Lexer, Parser, XRefEntry, XRefParser, XRefTable};

/// A loaded PDF document
///
/// This struct represents a PDF document that has been loaded from bytes.
/// It provides lazy object resolution and modification capabilities.
pub struct LoadedDocument {
    /// Raw PDF data
    data: Vec<u8>,
    /// Cross-reference table
    xref: XRefTable,
    /// Cached resolved objects
    cache: HashMap<PdfRef, PdfObject>,
    /// PDF version
    version: String,
}

impl LoadedDocument {
    /// Loads a PDF document from bytes
    pub fn load(data: Vec<u8>) -> Result<Self> {
        // Verify PDF header
        let version = Self::parse_header(&data)?;

        // Parse xref table
        let xref_parser = XRefParser::new(&data);
        let xref = xref_parser.parse_all()?;

        Ok(LoadedDocument {
            data,
            xref,
            cache: HashMap::new(),
            version,
        })
    }

    /// Parses the PDF header and returns the version
    fn parse_header(data: &[u8]) -> Result<String> {
        // PDF header must be in the first 1024 bytes
        let header_data = &data[..std::cmp::min(data.len(), 1024)];

        // Look for %PDF-x.y
        let pdf_marker = b"%PDF-";
        let pos = header_data
            .windows(pdf_marker.len())
            .position(|w| w == pdf_marker)
            .ok_or_else(|| Error::InvalidStructure("Missing PDF header".to_string()))?;

        // Read version
        let version_start = pos + pdf_marker.len();
        let mut version_end = version_start;
        while version_end < header_data.len()
            && (header_data[version_end].is_ascii_digit() || header_data[version_end] == b'.')
        {
            version_end += 1;
        }

        let version = std::str::from_utf8(&header_data[version_start..version_end])
            .map_err(|_| Error::InvalidStructure("Invalid PDF version encoding".to_string()))?
            .to_string();

        Ok(version)
    }

    /// Gets the PDF version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Gets the number of objects in the xref table
    pub fn object_count(&self) -> usize {
        self.xref.len()
    }

    /// Gets the trailer dictionary
    pub fn trailer(&self) -> Option<&PdfDict> {
        self.xref.trailer()
    }

    /// Gets the catalog (root) object reference
    pub fn catalog_ref(&self) -> Option<PdfRef> {
        self.xref.catalog_ref()
    }

    /// Gets the info dictionary reference
    pub fn info_ref(&self) -> Option<PdfRef> {
        self.xref.info_ref()
    }

    /// Resolves an indirect reference to its object
    pub fn resolve(&mut self, reference: PdfRef) -> Result<&PdfObject> {
        // Check cache first
        if self.cache.contains_key(&reference) {
            return Ok(self.cache.get(&reference).unwrap());
        }

        // Get xref entry
        let entry = self.xref.get(reference.object_number()).ok_or_else(|| {
            Error::MissingObject(format!("Object {} not found", reference.object_number()))
        })?;

        let obj = match entry {
            XRefEntry::InUse { offset, generation } => {
                // Verify generation matches
                if *generation != reference.generation() {
                    return Err(Error::MissingObject(format!(
                        "Object {} generation mismatch: expected {}, got {}",
                        reference.object_number(),
                        generation,
                        reference.generation()
                    )));
                }
                self.parse_object_at(*offset as usize)?
            }
            XRefEntry::Compressed { stream_obj, index } => {
                self.parse_compressed_object(*stream_obj, *index)?
            }
            XRefEntry::Free { .. } => {
                return Err(Error::MissingObject(format!(
                    "Object {} is free",
                    reference.object_number()
                )));
            }
        };

        self.cache.insert(reference, obj);
        Ok(self.cache.get(&reference).unwrap())
    }

    /// Parses an object at the given byte offset
    fn parse_object_at(&self, offset: usize) -> Result<PdfObject> {
        use crate::parser::lexer::Token;

        if offset >= self.data.len() {
            return Err(Error::Parse {
                message: "Object offset beyond end of file".to_string(),
                position: offset,
            });
        }

        let mut lexer = Lexer::new(&self.data[offset..]);

        // Read object number (integer)
        let obj_token = lexer.next_token()?;
        if !matches!(obj_token, Token::Integer(_)) {
            return Err(Error::Parse {
                message: format!("Expected object number, got {:?}", obj_token),
                position: offset,
            });
        }

        // Read generation (integer)
        let gen_token = lexer.next_token()?;
        if !matches!(gen_token, Token::Integer(_)) {
            return Err(Error::Parse {
                message: format!("Expected generation number, got {:?}", gen_token),
                position: offset,
            });
        }

        // Expect 'obj' keyword
        let obj_keyword = lexer.next_token()?;
        if obj_keyword != Token::Obj {
            return Err(Error::Parse {
                message: format!("Expected 'obj' keyword, got {:?}", obj_keyword),
                position: offset,
            });
        }

        // Parse the actual object
        let mut parser = Parser::new(&mut lexer);
        parser.parse_object()
    }

    /// Parses an object from an object stream
    fn parse_compressed_object(&mut self, stream_obj: u32, index: u32) -> Result<PdfObject> {
        // First, resolve the object stream
        let stream_ref = PdfRef::new(stream_obj);
        let stream = self.resolve(stream_ref)?.clone();

        let stream = match stream {
            PdfObject::Stream(s) => s,
            _ => {
                return Err(Error::InvalidStructure(format!(
                    "Object {} is not a stream",
                    stream_obj
                )))
            }
        };

        // Verify it's an object stream
        if stream.dict().get_type() != Some("ObjStm") {
            return Err(Error::InvalidStructure(format!(
                "Object {} is not an object stream",
                stream_obj
            )));
        }

        // Get stream parameters
        let n = stream
            .dict()
            .get_integer("N")
            .ok_or_else(|| Error::InvalidStructure("Object stream missing N".to_string()))?
            as usize;

        let first = stream
            .dict()
            .get_integer("First")
            .ok_or_else(|| Error::InvalidStructure("Object stream missing First".to_string()))?
            as usize;

        // Decode stream data
        let data = stream.decode()?;

        // Parse the header (object numbers and offsets)
        let mut header_lexer = Lexer::new(&data[..first]);
        let mut header_parser = Parser::new(&mut header_lexer);

        let mut obj_offsets: Vec<(u32, usize)> = Vec::with_capacity(n);
        for _ in 0..n {
            let obj_num = header_parser.parse_object()?.as_integer().ok_or_else(|| {
                Error::InvalidStructure("Invalid object stream header".to_string())
            })? as u32;

            let obj_offset = header_parser.parse_object()?.as_integer().ok_or_else(|| {
                Error::InvalidStructure("Invalid object stream header".to_string())
            })? as usize;

            obj_offsets.push((obj_num, obj_offset));
        }

        // Find the requested object
        if index as usize >= obj_offsets.len() {
            return Err(Error::MissingObject(format!(
                "Object index {} out of range in object stream {}",
                index, stream_obj
            )));
        }

        let (_, obj_offset) = obj_offsets[index as usize];
        let abs_offset = first + obj_offset;

        // Parse the object
        let mut lexer = Lexer::new(&data[abs_offset..]);
        let mut parser = Parser::new(&mut lexer);
        parser.parse_object()
    }

    /// Resolves an object and returns a clone if it's the expected type
    pub fn resolve_dict(&mut self, reference: PdfRef) -> Result<PdfDict> {
        let obj = self.resolve(reference)?.clone();
        match obj {
            PdfObject::Dict(d) => Ok(d),
            _ => Err(Error::InvalidObjectType {
                expected: "dictionary".to_string(),
                actual: obj.type_name().to_string(),
            }),
        }
    }

    /// Resolves an object and returns a clone if it's a stream
    pub fn resolve_stream(&mut self, reference: PdfRef) -> Result<PdfStream> {
        let obj = self.resolve(reference)?.clone();
        match obj {
            PdfObject::Stream(s) => Ok(s),
            _ => Err(Error::InvalidObjectType {
                expected: "stream".to_string(),
                actual: obj.type_name().to_string(),
            }),
        }
    }

    /// Gets the catalog dictionary
    pub fn catalog(&mut self) -> Result<PdfDict> {
        let catalog_ref = self.catalog_ref().ok_or_else(|| {
            Error::InvalidStructure("Missing catalog reference in trailer".to_string())
        })?;
        self.resolve_dict(catalog_ref)
    }

    /// Gets the pages tree root reference
    pub fn pages_ref(&mut self) -> Result<PdfRef> {
        let catalog = self.catalog()?;
        catalog
            .get_ref("Pages")
            .ok_or_else(|| Error::InvalidStructure("Catalog missing Pages".to_string()))
    }

    /// Gets the page count
    pub fn page_count(&mut self) -> Result<usize> {
        let pages_ref = self.pages_ref()?;
        let pages = self.resolve_dict(pages_ref)?;
        pages
            .get_integer("Count")
            .map(|c| c as usize)
            .ok_or_else(|| Error::InvalidStructure("Pages missing Count".to_string()))
    }

    /// Gets a specific page by index (0-based)
    pub fn page(&mut self, index: usize) -> Result<PdfDict> {
        let pages_ref = self.pages_ref()?;
        self.find_page(pages_ref, index, &mut 0)
    }

    /// Recursively finds a page in the page tree
    fn find_page(
        &mut self,
        node_ref: PdfRef,
        target_index: usize,
        current_index: &mut usize,
    ) -> Result<PdfDict> {
        let node = self.resolve_dict(node_ref)?;

        match node.get_type() {
            Some("Pages") => {
                // This is a page tree node
                let kids = node
                    .get_array("Kids")
                    .ok_or_else(|| Error::InvalidStructure("Pages missing Kids".to_string()))?
                    .clone();

                for i in 0..kids.len() {
                    let kid_ref = kids
                        .get_reference(i)
                        .ok_or_else(|| Error::InvalidStructure("Invalid Kids entry".to_string()))?;

                    // Check if this is a page or another Pages node
                    let kid = self.resolve_dict(kid_ref)?;

                    match kid.get_type() {
                        Some("Page") => {
                            if *current_index == target_index {
                                return Ok(kid);
                            }
                            *current_index += 1;
                        }
                        Some("Pages") => {
                            let count = kid.get_integer("Count").unwrap_or(0) as usize;
                            if target_index < *current_index + count {
                                return self.find_page(kid_ref, target_index, current_index);
                            }
                            *current_index += count;
                        }
                        _ => {
                            return Err(Error::InvalidStructure(
                                "Invalid page tree node".to_string(),
                            ))
                        }
                    }
                }

                Err(Error::MissingObject(format!(
                    "Page {} not found",
                    target_index
                )))
            }
            Some("Page") => {
                if *current_index == target_index {
                    Ok(node)
                } else {
                    *current_index += 1;
                    Err(Error::MissingObject(format!(
                        "Page {} not found",
                        target_index
                    )))
                }
            }
            _ => Err(Error::InvalidStructure(
                "Invalid page tree node".to_string(),
            )),
        }
    }

    /// Gets the info dictionary if present
    pub fn info(&mut self) -> Result<Option<PdfDict>> {
        if let Some(info_ref) = self.info_ref() {
            Ok(Some(self.resolve_dict(info_ref)?))
        } else {
            Ok(None)
        }
    }

    /// Gets the document title
    pub fn title(&mut self) -> Result<Option<String>> {
        if let Some(info) = self.info()? {
            if let Some(PdfObject::String(s)) = info.get("Title") {
                return Ok(Some(s.decode_text()));
            }
        }
        Ok(None)
    }

    /// Gets the document author
    pub fn author(&mut self) -> Result<Option<String>> {
        if let Some(info) = self.info()? {
            if let Some(PdfObject::String(s)) = info.get("Author") {
                return Ok(Some(s.decode_text()));
            }
        }
        Ok(None)
    }

    /// Gets the raw PDF data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Gets the xref table
    pub fn xref(&self) -> &XRefTable {
        &self.xref
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header() {
        let data = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n";
        let version = LoadedDocument::parse_header(data).unwrap();
        assert_eq!(version, "1.7");
    }

    #[test]
    fn test_parse_header_1_4() {
        let data = b"%PDF-1.4\n";
        let version = LoadedDocument::parse_header(data).unwrap();
        assert_eq!(version, "1.4");
    }

    #[test]
    fn test_parse_header_invalid() {
        let data = b"Not a PDF file";
        let result = LoadedDocument::parse_header(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_generated_pdf() {
        use crate::api::Document;

        // Generate a simple PDF
        let mut doc = Document::new();
        doc.title("Test Document");
        doc.author("Test Author");
        doc.text_at("Hello, World!", [72.0, 700.0]);
        let pdf_data = doc.render().unwrap();

        // Load it back
        let mut loaded = LoadedDocument::load(pdf_data).unwrap();

        // Verify
        assert_eq!(loaded.version(), "1.7");
        assert!(loaded.object_count() > 0);

        // Get page count
        let count = loaded.page_count().unwrap();
        assert_eq!(count, 1);

        // Get page
        let page = loaded.page(0).unwrap();
        assert_eq!(page.get_type(), Some("Page"));
    }

    #[test]
    fn test_load_multi_page_pdf() {
        use crate::api::Document;

        // Generate a multi-page PDF
        let mut doc = Document::new();
        doc.text_at("Page 1", [72.0, 700.0]);
        doc.start_new_page();
        doc.text_at("Page 2", [72.0, 700.0]);
        doc.start_new_page();
        doc.text_at("Page 3", [72.0, 700.0]);
        let pdf_data = doc.render().unwrap();

        // Load it back
        let mut loaded = LoadedDocument::load(pdf_data).unwrap();

        // Verify page count
        let count = loaded.page_count().unwrap();
        assert_eq!(count, 3);

        // Access each page
        for i in 0..3 {
            let page = loaded.page(i).unwrap();
            assert_eq!(page.get_type(), Some("Page"));
        }
    }
}
