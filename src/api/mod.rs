//! High-level PDF API
//!
//! This module provides the user-facing API for creating and manipulating PDFs.

pub mod page;

#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::Write;
#[cfg(feature = "std")]
use std::path::Path;

use crate::content::ContentBuilder;
use crate::document::{create_catalog, create_page, create_pages, PdfContext};
use crate::error::Result;
use crate::font::StandardFont;
use crate::objects::{PdfDict, PdfObject, PdfRef, PdfStream};

pub use page::{PageLayout, PageSize};

/// A PDF Document
///
/// This is the main entry point for creating PDF documents.
///
/// # Example
///
/// ```rust,no_run
/// use pdf_rs::api::Document;
///
/// let mut doc = Document::new();
/// doc.text("Hello, World!");
/// doc.save("hello.pdf").unwrap();
/// ```
pub struct Document {
    /// PDF object context
    context: PdfContext,
    /// Current page size
    page_size: PageSize,
    /// Current page layout
    page_layout: PageLayout,
    /// Pages (content builders)
    pages: Vec<PageData>,
    /// Current page index
    current_page: usize,
    /// Registered fonts
    fonts: Vec<(String, PdfRef)>,
    /// Current font name
    current_font: String,
    /// Current font size
    current_font_size: f64,
    /// Document info
    info: DocumentInfo,
}

/// Internal page data
struct PageData {
    content: ContentBuilder,
    size: PageSize,
    layout: PageLayout,
}

/// Document metadata
#[derive(Debug, Default)]
pub struct DocumentInfo {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
}

impl Document {
    /// Creates a new empty PDF document
    pub fn new() -> Self {
        let mut doc = Document {
            context: PdfContext::new(),
            page_size: PageSize::A4,
            page_layout: PageLayout::Portrait,
            pages: Vec::new(),
            current_page: 0,
            fonts: Vec::new(),
            current_font: "Helvetica".to_string(),
            current_font_size: 12.0,
            info: DocumentInfo {
                producer: Some("pdf_rs".to_string()),
                ..Default::default()
            },
        };

        // Start with one page
        doc.add_page();
        doc
    }

    /// Creates a PDF and saves it to a file
    ///
    /// This method is only available with the `std` feature (enabled by default).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdf_rs::api::Document;
    ///
    /// Document::generate("hello.pdf", |doc| {
    ///     doc.text("Hello, World!");
    ///     Ok(())
    /// }).unwrap();
    /// ```
    #[cfg(feature = "std")]
    pub fn generate<P, F>(path: P, f: F) -> Result<()>
    where
        P: AsRef<Path>,
        F: FnOnce(&mut Document) -> Result<()>,
    {
        let mut doc = Document::new();
        f(&mut doc)?;
        doc.save(path)
    }

    /// Sets the page size for new pages
    pub fn page_size(&mut self, size: PageSize) -> &mut Self {
        self.page_size = size;
        if let Some(page) = self.pages.get_mut(self.current_page) {
            page.size = size;
        }
        self
    }

    /// Sets the page layout (portrait/landscape)
    pub fn page_layout(&mut self, layout: PageLayout) -> &mut Self {
        self.page_layout = layout;
        if let Some(page) = self.pages.get_mut(self.current_page) {
            page.layout = layout;
        }
        self
    }

    /// Starts a new page
    pub fn start_new_page(&mut self) -> &mut Self {
        self.add_page();
        self
    }

    /// Adds a new page
    fn add_page(&mut self) {
        let page = PageData {
            content: ContentBuilder::new(),
            size: self.page_size,
            layout: self.page_layout,
        };
        self.pages.push(page);
        self.current_page = self.pages.len() - 1;
    }

    /// Gets the current page content builder
    #[allow(dead_code)]
    fn current_content(&mut self) -> &mut ContentBuilder {
        &mut self.pages[self.current_page].content
    }

    /// Sets the current font
    pub fn font(&mut self, name: &str) -> FontBuilder<'_> {
        self.current_font = name.to_string();
        FontBuilder { doc: self }
    }

    /// Draws text at the current position
    pub fn text(&mut self, text: &str) -> &mut Self {
        // Ensure font is registered
        self.ensure_font(&self.current_font.clone());

        let font_name = self.current_font.clone();
        let font_size = self.current_font_size;
        let page = &mut self.pages[self.current_page];
        let dims = page.size.dimensions(page.layout);

        // Position text at top of page, flowing down
        let y = dims.1 - 72.0; // 1 inch from top
        let x = 72.0; // 1 inch from left

        page.content
            .begin_text()
            .set_font(&font_name, font_size)
            .move_text_pos(x, y)
            .show_text(text)
            .end_text();

        self
    }

    /// Draws text at a specific position
    pub fn text_at(&mut self, text: &str, pos: [f64; 2]) -> &mut Self {
        self.ensure_font(&self.current_font.clone());

        let font_name = self.current_font.clone();
        let font_size = self.current_font_size;
        let page = &mut self.pages[self.current_page];

        page.content
            .begin_text()
            .set_font(&font_name, font_size)
            .move_text_pos(pos[0], pos[1])
            .show_text(text)
            .end_text();

        self
    }

    /// Ensures a font is registered
    fn ensure_font(&mut self, name: &str) {
        if !self.fonts.iter().any(|(n, _)| n == name) {
            // Register the font
            if let Some(std_font) = StandardFont::from_name(name) {
                let dict = std_font.to_dict();
                let font_ref = self.context.register(PdfObject::Dict(dict));
                self.fonts.push((name.to_string(), font_ref));
            }
        }
    }

    /// Strokes a path using a closure
    pub fn stroke<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut StrokeContext),
    {
        let mut ctx = StrokeContext {
            content: &mut self.pages[self.current_page].content,
        };
        ctx.content.save_state();
        f(&mut ctx);
        ctx.content.stroke().restore_state();
        self
    }

    /// Fills a path using a closure
    pub fn fill<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut FillContext),
    {
        let mut ctx = FillContext {
            content: &mut self.pages[self.current_page].content,
        };
        ctx.content.save_state();
        f(&mut ctx);
        ctx.content.fill().restore_state();
        self
    }

    /// Sets document title
    pub fn title(&mut self, title: &str) -> &mut Self {
        self.info.title = Some(title.to_string());
        self
    }

    /// Sets document author
    pub fn author(&mut self, author: &str) -> &mut Self {
        self.info.author = Some(author.to_string());
        self
    }

    /// Saves the document to a file
    ///
    /// This method is only available with the `std` feature (enabled by default).
    /// For WASM environments, use [`render()`](Self::render) to get the PDF bytes.
    #[cfg(feature = "std")]
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let bytes = self.render()?;
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    /// Renders the document to bytes
    pub fn render(&mut self) -> Result<Vec<u8>> {
        // Build font resources dictionary
        let mut font_dict = PdfDict::new();
        for (name, font_ref) in &self.fonts {
            font_dict.set(name, PdfObject::Reference(*font_ref));
        }

        let mut resources = PdfDict::new();
        if !self.fonts.is_empty() {
            resources.set("Font", PdfObject::Dict(font_dict));
        }
        let resources_ref = self.context.register(PdfObject::Dict(resources));

        // Create page objects
        let pages_ref = self.context.alloc_ref();
        let mut page_refs = Vec::new();

        for page in &mut self.pages {
            // Create content stream
            let content_data = std::mem::take(&mut page.content).build();
            let content_stream = PdfStream::from_data_compressed(content_data);
            let content_ref = self.context.register(PdfObject::Stream(content_stream));

            // Create page dictionary
            let dims = page.size.dimensions(page.layout);
            let media_box = [0.0, 0.0, dims.0, dims.1];
            let page_dict =
                create_page(pages_ref, media_box, Some(resources_ref), Some(content_ref));
            let page_ref = self.context.register(PdfObject::Dict(page_dict));
            page_refs.push(page_ref);
        }

        // Create pages dictionary
        let pages_dict = create_pages(page_refs.clone(), page_refs.len() as i64);
        self.context.assign(pages_ref, PdfObject::Dict(pages_dict));

        // Create catalog
        let catalog = create_catalog(pages_ref);
        let catalog_ref = self.context.register(PdfObject::Dict(catalog));

        // Create info dictionary
        let info_ref = if self.info.title.is_some()
            || self.info.author.is_some()
            || self.info.producer.is_some()
        {
            let mut info_dict = PdfDict::new();
            if let Some(title) = &self.info.title {
                info_dict.set("Title", PdfObject::String(title.as_str().into()));
            }
            if let Some(author) = &self.info.author {
                info_dict.set("Author", PdfObject::String(author.as_str().into()));
            }
            if let Some(producer) = &self.info.producer {
                info_dict.set("Producer", PdfObject::String(producer.as_str().into()));
            }
            Some(self.context.register(PdfObject::Dict(info_dict)))
        } else {
            None
        };

        // Write PDF
        let objects: Vec<(PdfRef, PdfObject)> = self.context.to_vec();
        crate::writer::write_pdf(&objects, catalog_ref, info_ref)
    }
}

impl Default for Document {
    fn default() -> Self {
        Document::new()
    }
}

/// Font builder for fluent font configuration
pub struct FontBuilder<'a> {
    doc: &'a mut Document,
}

impl<'a> FontBuilder<'a> {
    /// Sets the font size
    pub fn size(self, size: f64) -> &'a mut Document {
        self.doc.current_font_size = size;
        self.doc
    }
}

/// Context for stroke operations
pub struct StrokeContext<'a> {
    content: &'a mut ContentBuilder,
}

impl<'a> StrokeContext<'a> {
    /// Sets line width
    pub fn line_width(&mut self, width: f64) -> &mut Self {
        self.content.set_line_width(width);
        self
    }

    /// Sets stroke color (RGB)
    pub fn color(&mut self, r: f64, g: f64, b: f64) -> &mut Self {
        self.content.set_stroke_color_rgb(r, g, b);
        self
    }

    /// Draws a line
    pub fn line(&mut self, from: [f64; 2], to: [f64; 2]) -> &mut Self {
        self.content.move_to(from[0], from[1]).line_to(to[0], to[1]);
        self
    }

    /// Draws a rectangle
    pub fn rectangle(&mut self, origin: [f64; 2], width: f64, height: f64) -> &mut Self {
        self.content.rect(origin[0], origin[1], width, height);
        self
    }

    /// Moves to a point
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.content.move_to(x, y);
        self
    }

    /// Draws a line to a point
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.content.line_to(x, y);
        self
    }
}

/// Context for fill operations
pub struct FillContext<'a> {
    content: &'a mut ContentBuilder,
}

impl<'a> FillContext<'a> {
    /// Sets fill color (RGB)
    pub fn color(&mut self, r: f64, g: f64, b: f64) -> &mut Self {
        self.content.set_fill_color_rgb(r, g, b);
        self
    }

    /// Draws a rectangle
    pub fn rectangle(&mut self, origin: [f64; 2], width: f64, height: f64) -> &mut Self {
        self.content.rect(origin[0], origin[1], width, height);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new();
        assert_eq!(doc.pages.len(), 1);
    }

    #[test]
    fn test_document_render() {
        let mut doc = Document::new();
        doc.text("Hello World");
        let bytes = doc.render().unwrap();

        // Check PDF header
        assert!(bytes.starts_with(b"%PDF-1.7"));

        // Check for EOF marker
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("%%EOF"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_document_generate() {
        // This test would write to disk, so we just test the API compiles
        let _ = Document::generate("/tmp/test.pdf", |doc| {
            doc.text("Test");
            Ok(())
        });
    }
}
