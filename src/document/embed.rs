//! PDF embedding and page extraction
//!
//! This module handles embedding PDF pages as XObjects and extracting page content.

use crate::error::{Error, Result};
use crate::objects::{PdfArray, PdfDict, PdfName, PdfObject, PdfStream};

use super::LoadedDocument;

/// An embedded PDF page that can be drawn on other pages
#[derive(Debug, Clone)]
pub struct EmbeddedPage {
    /// Name for referencing this page as XObject
    pub name: String,
    /// Original page width
    pub width: f64,
    /// Original page height
    pub height: f64,
    /// The Form XObject stream
    pub(crate) xobject: PdfStream,
    /// Resources dictionary (fonts, images, etc.)
    /// Reserved for future use when resource merging is implemented
    #[allow(dead_code)]
    pub(crate) resources: Option<PdfDict>,
}

impl EmbeddedPage {
    /// Returns the aspect ratio (width / height)
    pub fn aspect_ratio(&self) -> f64 {
        if self.height != 0.0 {
            self.width / self.height
        } else {
            1.0
        }
    }

    /// Calculates dimensions to fit within bounds while preserving aspect ratio
    pub fn fit_dimensions(&self, max_width: f64, max_height: f64) -> (f64, f64) {
        let scale_x = max_width / self.width;
        let scale_y = max_height / self.height;
        let scale = scale_x.min(scale_y);
        (self.width * scale, self.height * scale)
    }
}

impl LoadedDocument {
    /// Extracts a page as an embeddable Form XObject
    ///
    /// This converts a page to a Form XObject that can be embedded
    /// and drawn in another document.
    pub fn extract_page(&mut self, page_index: usize) -> Result<EmbeddedPage> {
        let page = self.page(page_index)?;

        // Get MediaBox for dimensions
        let media_box = self.get_page_media_box(&page)?;
        let width = media_box[2] - media_box[0];
        let height = media_box[3] - media_box[1];

        // Get page content
        let content_data = self.get_page_content(&page)?;

        // Get resources
        let resources = self.get_page_resources(&page)?;

        // Create Form XObject
        let mut xobject_dict = PdfDict::new();
        xobject_dict.set("Type", PdfObject::Name(PdfName::new("XObject")));
        xobject_dict.set("Subtype", PdfObject::Name(PdfName::new("Form")));
        xobject_dict.set("FormType", PdfObject::Integer(1));

        // BBox
        let bbox = PdfArray::from(vec![
            PdfObject::Real(media_box[0]),
            PdfObject::Real(media_box[1]),
            PdfObject::Real(media_box[2]),
            PdfObject::Real(media_box[3]),
        ]);
        xobject_dict.set("BBox", PdfObject::Array(bbox));

        // Matrix (identity, but could apply CropBox offset)
        let matrix = PdfArray::from(vec![
            PdfObject::Integer(1),
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Integer(1),
            PdfObject::Real(-media_box[0]),
            PdfObject::Real(-media_box[1]),
        ]);
        xobject_dict.set("Matrix", PdfObject::Array(matrix));

        // Create the stream
        let xobject = PdfStream::new(xobject_dict, content_data);

        Ok(EmbeddedPage {
            name: format!("Page{}", page_index),
            width,
            height,
            xobject,
            resources,
        })
    }

    /// Gets the MediaBox for a page, inheriting from parent if needed
    fn get_page_media_box(&mut self, page: &PdfDict) -> Result<[f64; 4]> {
        // Try to get MediaBox directly from page
        if let Some(arr) = page.get_array("MediaBox") {
            return self.parse_rect_array(arr);
        }

        // Try to inherit from parent
        if let Some(parent_ref) = page.get_ref("Parent") {
            let parent = self.resolve_dict(parent_ref)?;
            if let Some(arr) = parent.get_array("MediaBox") {
                return self.parse_rect_array(arr);
            }
        }

        // Default to US Letter
        Ok([0.0, 0.0, 612.0, 792.0])
    }

    /// Parses a rectangle array
    fn parse_rect_array(&self, arr: &PdfArray) -> Result<[f64; 4]> {
        if arr.len() < 4 {
            return Err(Error::InvalidStructure(
                "Invalid rectangle array".to_string(),
            ));
        }

        let get_num = |i: usize| -> f64 {
            match arr.get(i) {
                Some(PdfObject::Integer(n)) => *n as f64,
                Some(PdfObject::Real(n)) => *n,
                _ => 0.0,
            }
        };

        Ok([get_num(0), get_num(1), get_num(2), get_num(3)])
    }

    /// Gets the content stream data for a page
    fn get_page_content(&mut self, page: &PdfDict) -> Result<Vec<u8>> {
        match page.get("Contents") {
            Some(PdfObject::Reference(content_ref)) => {
                // Single content stream
                let stream = self.resolve_stream(*content_ref)?;
                stream.decode()
            }
            Some(PdfObject::Array(arr)) => {
                // Multiple content streams - concatenate them
                let mut all_content = Vec::new();
                for i in 0..arr.len() {
                    if let Some(content_ref) = arr.get_reference(i) {
                        let stream = self.resolve_stream(content_ref)?;
                        let decoded = stream.decode()?;
                        if !all_content.is_empty() {
                            all_content.push(b'\n');
                        }
                        all_content.extend_from_slice(&decoded);
                    }
                }
                Ok(all_content)
            }
            Some(PdfObject::Stream(stream)) => {
                // Inline stream (rare)
                stream.decode()
            }
            None => {
                // Empty page
                Ok(Vec::new())
            }
            _ => Err(Error::InvalidStructure(
                "Invalid Contents entry in page".to_string(),
            )),
        }
    }

    /// Gets the resources dictionary for a page
    fn get_page_resources(&mut self, page: &PdfDict) -> Result<Option<PdfDict>> {
        match page.get("Resources") {
            Some(PdfObject::Reference(res_ref)) => {
                let resources = self.resolve_dict(*res_ref)?;
                Ok(Some(resources))
            }
            Some(PdfObject::Dict(d)) => Ok(Some(d.clone())),
            None => {
                // Try to inherit from parent
                if let Some(parent_ref) = page.get_ref("Parent") {
                    let parent = self.resolve_dict(parent_ref)?;
                    match parent.get("Resources") {
                        Some(PdfObject::Reference(res_ref)) => {
                            let resources = self.resolve_dict(*res_ref)?;
                            Ok(Some(resources))
                        }
                        Some(PdfObject::Dict(d)) => Ok(Some(d.clone())),
                        _ => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// Extracts all pages as embeddable Form XObjects
    pub fn extract_all_pages(&mut self) -> Result<Vec<EmbeddedPage>> {
        let count = self.page_count()?;
        let mut pages = Vec::with_capacity(count);
        for i in 0..count {
            pages.push(self.extract_page(i)?);
        }
        Ok(pages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Document;

    #[test]
    fn test_extract_page() {
        // Create a simple PDF
        let mut doc = Document::new();
        doc.text_at("Hello, World!", [72.0, 700.0]);
        let pdf_data = doc.render().unwrap();

        // Load and extract
        let mut loaded = LoadedDocument::load(pdf_data).unwrap();
        let embedded = loaded.extract_page(0).unwrap();

        assert!(embedded.width > 0.0);
        assert!(embedded.height > 0.0);
        assert!(!embedded.xobject.data().is_empty());
    }

    #[test]
    fn test_extract_all_pages() {
        // Create a multi-page PDF
        let mut doc = Document::new();
        doc.text_at("Page 1", [72.0, 700.0]);
        doc.start_new_page();
        doc.text_at("Page 2", [72.0, 700.0]);
        let pdf_data = doc.render().unwrap();

        // Load and extract all
        let mut loaded = LoadedDocument::load(pdf_data).unwrap();
        let pages = loaded.extract_all_pages().unwrap();

        assert_eq!(pages.len(), 2);
    }
}
