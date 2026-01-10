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
    /// Registered images (XObjects)
    images: Vec<(String, PdfRef, u32, u32)>, // (name, ref, width, height)
    /// Image counter for generating unique names
    image_counter: usize,
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
            images: Vec::new(),
            image_counter: 0,
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

    /// Returns the number of pages in the document
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Returns the current page number (1-based)
    pub fn page_number(&self) -> usize {
        self.current_page + 1
    }

    /// Switches to a specific page (0-based index)
    ///
    /// Returns true if the page exists, false otherwise.
    pub fn go_to_page(&mut self, index: usize) -> bool {
        if index < self.pages.len() {
            self.current_page = index;
            true
        } else {
            false
        }
    }

    /// Deletes a page at the specified index (0-based)
    ///
    /// Returns true if the page was deleted, false if index was out of bounds.
    /// The document must have at least one page, so deleting the last page
    /// will add a new blank page.
    pub fn delete_page(&mut self, index: usize) -> bool {
        if index >= self.pages.len() {
            return false;
        }

        self.pages.remove(index);

        // Ensure at least one page exists
        if self.pages.is_empty() {
            self.add_page();
        }

        // Adjust current page if needed
        if self.current_page >= self.pages.len() {
            self.current_page = self.pages.len() - 1;
        }

        true
    }

    /// Inserts a new blank page at the specified index
    ///
    /// Returns true if the page was inserted, false if index was out of bounds.
    pub fn insert_page(&mut self, index: usize) -> bool {
        if index > self.pages.len() {
            return false;
        }

        let page = PageData {
            content: ContentBuilder::new(),
            size: self.page_size,
            layout: self.page_layout,
        };
        self.pages.insert(index, page);

        // Adjust current page if insertion is before current
        if index <= self.current_page {
            self.current_page += 1;
        }

        true
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

    /// Embeds a JPEG image and draws it at the specified position
    ///
    /// Returns the image name for reuse, or an error if the image is invalid.
    pub fn image_jpeg(
        &mut self,
        data: &[u8],
        pos: [f64; 2],
        width: f64,
        height: f64,
    ) -> Result<String> {
        let image_data = crate::image::embed_jpeg(data)?;
        self.draw_image_data(image_data, pos, width, height)
    }

    /// Embeds a PNG image and draws it at the specified position
    ///
    /// Returns the image name for reuse, or an error if the image is invalid.
    /// This method requires the `png` feature.
    #[cfg(feature = "png")]
    pub fn image_png(
        &mut self,
        data: &[u8],
        pos: [f64; 2],
        width: f64,
        height: f64,
    ) -> Result<String> {
        let image_data = crate::image::embed_png(data)?;
        self.draw_image_data(image_data, pos, width, height)
    }

    /// Draws an already-embedded image by name
    pub fn draw_image(&mut self, name: &str, pos: [f64; 2], width: f64, height: f64) -> &mut Self {
        let page = &mut self.pages[self.current_page];
        page.content
            .save_state()
            .concat_matrix(width, 0.0, 0.0, height, pos[0], pos[1])
            .draw_xobject(name)
            .restore_state();
        self
    }

    /// Internal: draws image data and returns the image name
    fn draw_image_data(
        &mut self,
        image_data: crate::image::ImageData,
        pos: [f64; 2],
        width: f64,
        height: f64,
    ) -> Result<String> {
        // Generate unique name
        self.image_counter += 1;
        let name = format!("Im{}", self.image_counter);

        // Create XObject stream
        let xobject = image_data.to_xobject();
        let img_width = image_data.width;
        let img_height = image_data.height;

        // Handle soft mask (alpha channel) if present
        let xobject = if let Some(mask_data) = image_data.soft_mask {
            let mut mask_stream = PdfStream::from_data_compressed(mask_data);
            let mask_dict = mask_stream.dict_mut();
            mask_dict.set(
                "Type",
                PdfObject::Name(crate::objects::PdfName::new("XObject")),
            );
            mask_dict.set(
                "Subtype",
                PdfObject::Name(crate::objects::PdfName::new("Image")),
            );
            mask_dict.set("Width", PdfObject::Integer(img_width as i64));
            mask_dict.set("Height", PdfObject::Integer(img_height as i64));
            mask_dict.set(
                "ColorSpace",
                PdfObject::Name(crate::objects::PdfName::new("DeviceGray")),
            );
            mask_dict.set("BitsPerComponent", PdfObject::Integer(8));

            let mask_ref = self.context.register(PdfObject::Stream(mask_stream));

            // Add SMask to the image XObject
            let mut dict = xobject.dict().clone();
            dict.set("SMask", PdfObject::Reference(mask_ref));
            PdfStream::new(dict, xobject.data().to_vec())
        } else {
            xobject
        };

        let img_ref = self.context.register(PdfObject::Stream(xobject));
        self.images
            .push((name.clone(), img_ref, img_width, img_height));

        // Draw the image
        self.draw_image(&name, pos, width, height);

        Ok(name)
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

        // Build XObject resources dictionary (images)
        let mut xobject_dict = PdfDict::new();
        for (name, img_ref, _, _) in &self.images {
            xobject_dict.set(name, PdfObject::Reference(*img_ref));
        }

        let mut resources = PdfDict::new();
        if !self.fonts.is_empty() {
            resources.set("Font", PdfObject::Dict(font_dict));
        }
        if !self.images.is_empty() {
            resources.set("XObject", PdfObject::Dict(xobject_dict));
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

    /// Sets stroke color (grayscale)
    pub fn gray(&mut self, gray: f64) -> &mut Self {
        self.content.set_stroke_color_gray(gray);
        self
    }

    /// Sets stroke color (CMYK)
    pub fn cmyk(&mut self, c: f64, m: f64, y: f64, k: f64) -> &mut Self {
        self.content.set_stroke_color_cmyk(c, m, y, k);
        self
    }

    /// Sets dash pattern
    pub fn dash(&mut self, pattern: &[f64]) -> &mut Self {
        self.content.set_dash(pattern, 0.0);
        self
    }

    /// Sets dash pattern with phase
    pub fn dash_with_phase(&mut self, pattern: &[f64], phase: f64) -> &mut Self {
        self.content.set_dash(pattern, phase);
        self
    }

    /// Clears dash pattern (solid line)
    pub fn undash(&mut self) -> &mut Self {
        self.content.clear_dash();
        self
    }

    /// Sets line cap style
    pub fn cap(&mut self, cap: crate::content::LineCap) -> &mut Self {
        self.content.set_line_cap(cap);
        self
    }

    /// Sets line join style
    pub fn join(&mut self, join: crate::content::LineJoin) -> &mut Self {
        self.content.set_line_join(join);
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

    /// Draws a rounded rectangle
    pub fn rounded_rectangle(
        &mut self,
        origin: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        self.content
            .rounded_rect(origin[0], origin[1], width, height, radius);
        self
    }

    /// Draws a circle
    pub fn circle(&mut self, center: [f64; 2], radius: f64) -> &mut Self {
        self.content.circle(center[0], center[1], radius);
        self
    }

    /// Draws an ellipse
    pub fn ellipse(&mut self, center: [f64; 2], rx: f64, ry: f64) -> &mut Self {
        self.content.ellipse(center[0], center[1], rx, ry);
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

    /// Draws a cubic Bezier curve
    pub fn curve_to(&mut self, cp1: [f64; 2], cp2: [f64; 2], end: [f64; 2]) -> &mut Self {
        self.content
            .curve_to(cp1[0], cp1[1], cp2[0], cp2[1], end[0], end[1]);
        self
    }

    /// Closes the current path
    pub fn close_path(&mut self) -> &mut Self {
        self.content.close_path();
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

    /// Sets fill color (grayscale)
    pub fn gray(&mut self, gray: f64) -> &mut Self {
        self.content.set_fill_color_gray(gray);
        self
    }

    /// Sets fill color (CMYK)
    pub fn cmyk(&mut self, c: f64, m: f64, y: f64, k: f64) -> &mut Self {
        self.content.set_fill_color_cmyk(c, m, y, k);
        self
    }

    /// Draws a rectangle
    pub fn rectangle(&mut self, origin: [f64; 2], width: f64, height: f64) -> &mut Self {
        self.content.rect(origin[0], origin[1], width, height);
        self
    }

    /// Draws a rounded rectangle
    pub fn rounded_rectangle(
        &mut self,
        origin: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        self.content
            .rounded_rect(origin[0], origin[1], width, height, radius);
        self
    }

    /// Draws a circle
    pub fn circle(&mut self, center: [f64; 2], radius: f64) -> &mut Self {
        self.content.circle(center[0], center[1], radius);
        self
    }

    /// Draws an ellipse
    pub fn ellipse(&mut self, center: [f64; 2], rx: f64, ry: f64) -> &mut Self {
        self.content.ellipse(center[0], center[1], rx, ry);
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

    /// Closes the current path
    pub fn close_path(&mut self) -> &mut Self {
        self.content.close_path();
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

    #[test]
    fn test_page_count() {
        let mut doc = Document::new();
        assert_eq!(doc.page_count(), 1);

        doc.start_new_page();
        assert_eq!(doc.page_count(), 2);

        doc.start_new_page();
        assert_eq!(doc.page_count(), 3);
    }

    #[test]
    fn test_page_number() {
        let mut doc = Document::new();
        assert_eq!(doc.page_number(), 1);

        doc.start_new_page();
        assert_eq!(doc.page_number(), 2);

        doc.go_to_page(0);
        assert_eq!(doc.page_number(), 1);
    }

    #[test]
    fn test_go_to_page() {
        let mut doc = Document::new();
        doc.start_new_page();
        doc.start_new_page();

        assert!(doc.go_to_page(0));
        assert_eq!(doc.current_page, 0);

        assert!(doc.go_to_page(2));
        assert_eq!(doc.current_page, 2);

        assert!(!doc.go_to_page(10)); // Out of bounds
    }

    #[test]
    fn test_delete_page() {
        let mut doc = Document::new();
        doc.start_new_page();
        doc.start_new_page();
        assert_eq!(doc.page_count(), 3);

        // Delete middle page
        assert!(doc.delete_page(1));
        assert_eq!(doc.page_count(), 2);

        // Delete first page
        assert!(doc.delete_page(0));
        assert_eq!(doc.page_count(), 1);

        // Delete last page - should add a new blank page
        assert!(doc.delete_page(0));
        assert_eq!(doc.page_count(), 1);

        // Out of bounds
        assert!(!doc.delete_page(10));
    }

    #[test]
    fn test_insert_page() {
        let mut doc = Document::new();
        assert_eq!(doc.page_count(), 1);

        // Insert at beginning
        assert!(doc.insert_page(0));
        assert_eq!(doc.page_count(), 2);

        // Insert at end
        assert!(doc.insert_page(2));
        assert_eq!(doc.page_count(), 3);

        // Insert in middle
        assert!(doc.insert_page(1));
        assert_eq!(doc.page_count(), 4);

        // Out of bounds
        assert!(!doc.insert_page(10));
    }
}
