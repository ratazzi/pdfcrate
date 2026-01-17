//! High-level PDF API
//!
//! This module provides the user-facing API for creating and manipulating PDFs.

pub mod image;
pub mod layout;
pub mod link;
pub mod measurements;
pub mod outline;
pub mod page;
pub mod table;

#[cfg(feature = "std")]
use std::time::SystemTime;

#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::Write;
#[cfg(feature = "std")]
use std::path::Path;

use crate::content::ContentBuilder;
use crate::document::{create_catalog, create_page, create_pages, PdfContext};
use crate::error::Result;
#[cfg(feature = "fonts")]
use crate::font::ShapedGlyph;
use crate::font::StandardFont;
use crate::forms::{AcroForm, FormField};
use crate::objects::{PdfArray, PdfDict, PdfName, PdfObject, PdfRef, PdfStream, PdfString};

pub use image::{EmbeddedImage, ImageOptions, ImageSource, Position};
pub use layout::{
    BoundingBox, Color, FontStyle, Grid, GridBox, GridOptions, LayoutDocument, Margin, MultiBox,
    Overflow, PageNumberConfig, PageNumberPosition, RepeaterPages, TextAlign, TextBoxResult,
    TextFragment,
};
pub use link::{DestinationFit, HighlightMode, LinkAction, LinkAnnotation, LinkDestination};
pub use outline::{Outline, OutlineBuilder, OutlineDestination, OutlineItem};
pub use page::{PageLayout, PageSize};
pub use table::{
    BorderLine, Cell, CellContent, CellStyle, ColumnWidths, IntoCell, Table, TableOptions,
    TablePosition, VerticalAlign,
};

/// A PDF Document
///
/// This is the main entry point for creating PDF documents.
///
/// # Example
///
/// ```rust,no_run
/// use pdfcrate::api::Document;
///
/// let mut doc = Document::new();
/// doc.text_at("Hello, World!", [72.0, 700.0]);
/// doc.save("hello.pdf").unwrap();
/// ```
pub struct Document {
    /// PDF object context
    context: PdfContext,
    /// Current page size
    pub(crate) page_size: PageSize,
    /// Current page layout
    pub(crate) page_layout: PageLayout,
    /// Pages (content builders)
    pages: Vec<PageData>,
    /// Current page index
    current_page: usize,
    /// Registered fonts (name -> ref)
    fonts: Vec<(String, PdfRef)>,
    /// Embedded TrueType fonts (name -> font data)
    #[cfg(feature = "fonts")]
    embedded_fonts: std::collections::HashMap<String, std::sync::Arc<crate::font::EmbeddedFont>>,
    /// Used characters per font (for subsetting)
    #[cfg(feature = "fonts")]
    font_used_chars: std::collections::HashMap<String, std::collections::HashSet<char>>,
    /// Used glyphs per font (for shaping-aware subsetting)
    #[cfg(feature = "fonts")]
    font_used_glyphs: std::collections::HashMap<String, std::collections::BTreeMap<u16, String>>,
    /// Current font name
    current_font: String,
    /// Current font size
    pub(crate) current_font_size: f64,
    /// Whether current font is embedded (TrueType)
    current_font_embedded: bool,
    /// Document info
    info: DocumentInfo,
    /// Registered images (XObjects)
    images: Vec<(String, PdfRef, u32, u32)>, // (name, ref, width, height)
    /// Image counter for generating unique names
    image_counter: usize,
    /// Form fields
    form: AcroForm,
    /// ExtGState resources for transparency (name -> ref)
    ext_gstates: Vec<(String, PdfRef)>,
    /// Named destinations (name -> (page_index, fit))
    destinations: std::collections::HashMap<String, (usize, link::DestinationFit)>,
    /// Document outline (bookmarks)
    outline: outline::Outline,
}

/// Internal page data
struct PageData {
    content: ContentBuilder,
    size: PageSize,
    layout: PageLayout,
    /// Link annotations on this page
    annotations: Vec<link::LinkAnnotation>,
}

/// Document metadata
#[derive(Debug)]
pub struct DocumentInfo {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    /// Creation date.
    ///
    /// Defaults to `None` for reproducible builds. Use `Document::creation_date_now()`
    /// or `DocumentInfo::set_creation_date_now()` to set the current timestamp.
    #[cfg(feature = "std")]
    pub creation_date: Option<SystemTime>,
    /// Modification date.
    ///
    /// Defaults to `None` for reproducible builds. Use `Document::mod_date_now()`
    /// or `DocumentInfo::set_mod_date_now()` to set the current timestamp.
    /// NOT automatically set during render() to support reproducible builds.
    #[cfg(feature = "std")]
    pub mod_date: Option<SystemTime>,
}

impl Default for DocumentInfo {
    fn default() -> Self {
        DocumentInfo {
            title: None,
            author: None,
            subject: None,
            creator: None,
            producer: None,
            #[cfg(feature = "std")]
            creation_date: None,
            #[cfg(feature = "std")]
            mod_date: None,
        }
    }
}

impl DocumentInfo {
    /// Sets creation_date to the current time.
    ///
    /// By default, creation_date is `None` for reproducible builds.
    /// Call this method to include the current timestamp in the PDF.
    #[cfg(feature = "std")]
    pub fn set_creation_date_now(&mut self) {
        self.creation_date = Some(SystemTime::now());
    }

    /// Sets mod_date to the current time.
    ///
    /// By default, mod_date is `None` for reproducible builds.
    /// Call this method to include the current timestamp in the PDF.
    #[cfg(feature = "std")]
    pub fn set_mod_date_now(&mut self) {
        self.mod_date = Some(SystemTime::now());
    }
}

/// Options for stroke_axis
///
/// Configures the appearance of coordinate axes drawn by `stroke_axis`.
#[derive(Debug, Clone)]
pub struct AxisOptions {
    /// Origin point for the axes (default: [0, 0])
    pub at: [f64; 2],
    /// Width of the horizontal axis (default: page width - at.x)
    pub width: Option<f64>,
    /// Height of the vertical axis (default: page height - at.y)
    pub height: Option<f64>,
    /// Distance between tick marks (default: 100)
    pub step_length: f64,
    /// How far axes extend below/left of origin (default: 20)
    pub negative_axes_length: f64,
    /// Color for axes and labels as hex string (default: "000000")
    pub color: [f64; 3],
}

impl Default for AxisOptions {
    fn default() -> Self {
        AxisOptions {
            at: [0.0, 0.0],
            width: None,
            height: None,
            step_length: 100.0,
            negative_axes_length: 20.0,
            color: [0.0, 0.0, 0.0], // Black
        }
    }
}

impl AxisOptions {
    /// Creates new axis options with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the origin point for the axes
    pub fn at(mut self, x: f64, y: f64) -> Self {
        self.at = [x, y];
        self
    }

    /// Sets the width of the horizontal axis
    pub fn width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets the height of the vertical axis
    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    /// Sets the distance between tick marks
    pub fn step_length(mut self, step: f64) -> Self {
        self.step_length = step;
        self
    }

    /// Sets how far axes extend below/left of origin
    pub fn negative_axes_length(mut self, length: f64) -> Self {
        self.negative_axes_length = length;
        self
    }

    /// Sets the color for axes and labels (RGB, 0.0-1.0)
    pub fn color(mut self, r: f64, g: f64, b: f64) -> Self {
        self.color = [r, g, b];
        self
    }

    /// Sets the color for axes and labels from hex string (e.g., "ff0000" for red)
    pub fn color_hex(mut self, hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        if hex.len() >= 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                self.color = [r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0];
            }
        }
        self
    }
}

/// Formats a SystemTime as a PDF date string (PDF 1.7 §7.9.4)
///
/// Format: D:YYYYMMDDHHmmSSOHH'mm'
/// Example: D:20240111120000Z00'00'
///
/// Uses UTC timezone. The format matches PDF 1.7 specification §7.9.4.
///
/// Returns `None` if the time is before UNIX_EPOCH (1970-01-01 00:00:00 UTC),
/// as PDF date format cannot represent dates before 1970.
#[cfg(feature = "std")]
fn format_pdf_date(time: SystemTime) -> Option<String> {
    use std::time::UNIX_EPOCH;

    // Get the duration since Unix epoch, return None for pre-1970 dates
    let duration = time.duration_since(UNIX_EPOCH).ok()?;

    let total_seconds = duration.as_secs();

    // Calculate time components
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let total_hours = total_minutes / 60;
    let hours = total_hours % 24;
    let total_days = total_hours / 24;

    // Calculate date components using a simple algorithm
    // Start from 1970-01-01
    let mut year = 1970u32;
    let mut days = total_days;

    // Calculate year
    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let year_days = if is_leap { 366 } else { 365 };
        if days >= year_days {
            days -= year_days;
            year += 1;
        } else {
            break;
        }
    }

    // Calculate month and day
    let mut month = 1u32;
    let mut day = days + 1;

    let days_in_month = |m: u32, y: u32| -> u64 {
        match m {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            _ => 31,
        }
    };

    while month <= 12 && day > days_in_month(month, year) {
        day -= days_in_month(month, year);
        month += 1;
    }

    // Format as PDF date string: D:YYYYMMDDHHmmSSZ00'00'
    // Using UTC (Z) timezone indicator
    Some(format!(
        "D:{:04}{:02}{:02}{:02}{:02}{:02}Z00'00'",
        year, month, day, hours, minutes, seconds
    ))
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
            #[cfg(feature = "fonts")]
            embedded_fonts: std::collections::HashMap::new(),
            #[cfg(feature = "fonts")]
            font_used_chars: std::collections::HashMap::new(),
            #[cfg(feature = "fonts")]
            font_used_glyphs: std::collections::HashMap::new(),
            current_font: "Helvetica".to_string(),
            current_font_size: 12.0,
            current_font_embedded: false,
            info: DocumentInfo {
                producer: Some("pdfcrate".to_string()),
                ..Default::default()
            },
            images: Vec::new(),
            image_counter: 0,
            form: AcroForm::new(),
            ext_gstates: Vec::new(),
            destinations: std::collections::HashMap::new(),
            outline: outline::Outline::new(),
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
    /// use pdfcrate::api::Document;
    ///
    /// Document::generate("hello.pdf", |doc| {
    ///     doc.text_at("Hello, World!", [72.0, 700.0]);
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
            annotations: Vec::new(),
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
            annotations: Vec::new(),
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
        // Check if this is an embedded font
        #[cfg(feature = "fonts")]
        {
            self.current_font_embedded = self.embedded_fonts.contains_key(name);
        }
        #[cfg(not(feature = "fonts"))]
        {
            self.current_font_embedded = false;
        }
        FontBuilder { doc: self }
    }

    /// Draws text at a specific position
    pub fn text_at(&mut self, text: &str, pos: [f64; 2]) -> &mut Self {
        self.ensure_font(&self.current_font.clone());

        let font_name = self.current_font.clone();
        let font_size = self.current_font_size;
        let is_embedded = self.current_font_embedded;

        #[cfg(feature = "fonts")]
        {
            if is_embedded {
                self.draw_embedded_text(text, pos);
                return self;
            }
        }
        #[cfg(not(feature = "fonts"))]
        {
            let _ = is_embedded;
        }

        let content = &mut self.pages[self.current_page].content;
        content.begin_text();
        // Reset spacing to 0 to avoid inheriting from previous text_at_with_spacing calls
        content.set_character_spacing(0.0);
        content.set_word_spacing(0.0);
        content
            .set_font(&font_name, font_size)
            .move_text_pos(pos[0], pos[1])
            .show_text(text)
            .end_text();

        self
    }

    /// Adds a link annotation to the current page
    ///
    /// Creates a clickable region that performs the specified action when clicked.
    ///
    /// # Arguments
    ///
    /// * `annotation` - The link annotation to add
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdfcrate::prelude::*;
    ///
    /// let mut doc = Document::new();
    /// doc.text_at("Click here", [100.0, 700.0]);
    ///
    /// // Add a URL link
    /// doc.link_annotation(LinkAnnotation::url([100.0, 690.0, 200.0, 710.0], "https://example.com"));
    /// ```
    pub fn link_annotation(&mut self, annotation: link::LinkAnnotation) -> &mut Self {
        self.pages[self.current_page].annotations.push(annotation);
        self
    }

    /// Adds a URL link annotation to the current page
    ///
    /// Convenience method for adding a simple URL link.
    ///
    /// # Arguments
    ///
    /// * `rect` - The clickable rectangle [x1, y1, x2, y2]
    /// * `url` - The URL to open when clicked
    pub fn link_url(&mut self, rect: [f64; 4], url: impl Into<String>) -> &mut Self {
        self.link_annotation(link::LinkAnnotation::url(rect, url))
    }

    /// Adds a named destination to the document
    ///
    /// Named destinations allow linking to specific locations within the document
    /// using a human-readable name. They can be referenced from within the document
    /// or from external URLs (e.g., `document.pdf#chapter1`).
    ///
    /// # Arguments
    ///
    /// * `name` - The destination name (e.g., "chapter1", "introduction")
    /// * `page_index` - The target page index (0-based)
    /// * `fit` - How to display the page when navigating to this destination
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdfcrate::prelude::*;
    ///
    /// let mut doc = Document::new();
    ///
    /// // Add a named destination at current page
    /// doc.add_dest("intro", 0, DestinationFit::Fit);
    ///
    /// // Later, create a link to this destination
    /// doc.link_annotation(LinkAnnotation::named([100.0, 700.0, 200.0, 720.0], "intro"));
    /// ```
    pub fn add_dest(
        &mut self,
        name: impl Into<String>,
        page_index: usize,
        fit: link::DestinationFit,
    ) -> &mut Self {
        self.destinations.insert(name.into(), (page_index, fit));
        self
    }

    /// Adds a named destination at the current page
    ///
    /// Convenience method that uses the current page index.
    pub fn add_dest_here(
        &mut self,
        name: impl Into<String>,
        fit: link::DestinationFit,
    ) -> &mut Self {
        let page_index = self.current_page;
        self.add_dest(name, page_index, fit)
    }

    // === Document Outline (Bookmarks) ===

    /// Defines the document outline (bookmarks) using a closure-based DSL
    ///
    /// The outline appears in the PDF viewer's navigation panel and allows
    /// users to quickly jump to different sections of the document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// doc.outline(|o| {
    ///     o.section("Chapter 1", 0, |o| {
    ///         o.page("Introduction", 0);
    ///         o.page("Getting Started", 1);
    ///     });
    ///     o.section("Chapter 2", 2, |o| {
    ///         o.page("Advanced Topics", 2);
    ///         o.section_closed("Subsection 2.1", 3, |o| {
    ///             o.page("Details", 3);
    ///         });
    ///     });
    /// });
    /// ```
    pub fn outline<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut outline::OutlineBuilder),
    {
        let mut builder = outline::OutlineBuilder::new();
        f(&mut builder);
        self.outline.add_from_builder(builder);
        self
    }

    /// Adds a single outline item at the root level
    pub fn add_outline_item(&mut self, item: outline::OutlineItem) -> &mut Self {
        self.outline.add(item);
        self
    }

    /// Sets the entire document outline
    pub fn set_outline(&mut self, outline: outline::Outline) -> &mut Self {
        self.outline = outline;
        self
    }

    /// Returns whether the document has an outline
    pub fn has_outline(&self) -> bool {
        !self.outline.is_empty()
    }

    /// Draws text at a specific position with character and word spacing
    ///
    /// This is used by LayoutDocument to apply Tc/Tw operators correctly
    /// inside the BT..ET text object.
    pub(crate) fn text_at_with_spacing(
        &mut self,
        text: &str,
        pos: [f64; 2],
        char_spacing: f64,
        word_spacing: f64,
    ) -> &mut Self {
        self.ensure_font(&self.current_font.clone());

        let font_name = self.current_font.clone();
        let font_size = self.current_font_size;
        let is_embedded = self.current_font_embedded;

        #[cfg(feature = "fonts")]
        {
            if is_embedded {
                self.draw_embedded_text_with_spacing(text, pos, char_spacing, word_spacing);
                return self;
            }
        }
        #[cfg(not(feature = "fonts"))]
        {
            let _ = is_embedded;
        }

        let content = &mut self.pages[self.current_page].content;
        content.begin_text();

        // Always set spacing inside BT..ET to ensure correct values
        // (text state persists across text objects, so we must reset to 0 if needed)
        content.set_character_spacing(char_spacing);
        content.set_word_spacing(word_spacing);

        content
            .set_font(&font_name, font_size)
            .move_text_pos(pos[0], pos[1])
            .show_text(text)
            .end_text();

        self
    }

    /// Draws text at a specific position without kerning adjustments.
    ///
    /// This is useful for comparing kerning/shaping behavior in embedded fonts.
    #[cfg(feature = "fonts")]
    pub fn text_at_no_kerning(&mut self, text: &str, pos: [f64; 2]) -> &mut Self {
        self.ensure_font(&self.current_font.clone());

        let font_name = self.current_font.clone();
        let font_size = self.current_font_size;

        if self.current_font_embedded {
            if let Some(font) = self.embedded_fonts.get(&font_name) {
                let glyphs = font.shape_text(text);
                if glyphs.is_empty() {
                    return self;
                }

                self.track_font_glyphs(&font_name, &glyphs);

                let mut hex = String::with_capacity(glyphs.len() * 4);
                for glyph in glyphs {
                    hex.push_str(&format!("{:04X}", glyph.gid));
                }

                let content = &mut self.pages[self.current_page].content;
                content.begin_text();
                // Reset spacing to 0 to avoid inheriting from previous calls
                content.set_character_spacing(0.0);
                content.set_word_spacing(0.0);
                content
                    .set_font(&font_name, font_size)
                    .move_text_pos(pos[0], pos[1])
                    .show_text_hex(&hex)
                    .end_text();

                return self;
            }
        }

        let content = &mut self.pages[self.current_page].content;
        content.begin_text();
        // Reset spacing to 0 to avoid inheriting from previous calls
        content.set_character_spacing(0.0);
        content.set_word_spacing(0.0);
        content
            .set_font(&font_name, font_size)
            .move_text_pos(pos[0], pos[1])
            .show_text(text)
            .end_text();

        self
    }

    /// Embeds a TrueType font from bytes
    ///
    /// Returns the font name to use with `font()`.
    /// This method requires the `fonts` feature.
    ///
    /// Note: Font subsetting is applied automatically at render time.
    /// Only glyphs for characters actually used in the document will be included,
    /// which can significantly reduce file size.
    #[cfg(feature = "fonts")]
    pub fn embed_font(&mut self, data: Vec<u8>) -> Result<String> {
        use crate::font::EmbeddedFont;

        let font = EmbeddedFont::from_bytes(data)?;
        let name = font.name.clone();

        // Store font data - PDF objects will be created at render time
        // to allow subsetting based on used characters
        self.embedded_fonts
            .insert(name.clone(), std::sync::Arc::new(font));

        Ok(name)
    }

    /// Creates font PDF objects for an embedded font
    /// Called during render() to apply subsetting
    #[cfg(feature = "fonts")]
    fn create_font_objects(&mut self, name: &str) -> Option<PdfRef> {
        let font_arc = self.embedded_fonts.get(name)?.clone();
        let font = font_arc.as_ref();

        // Apply used glyphs/characters for subsetting - this populates glyph_set
        let mut font_clone = font.clone();
        if let Some(used_glyphs) = self.font_used_glyphs.get(name) {
            font_clone.apply_used_glyphs(used_glyphs);
        } else if let Some(used_chars) = self.font_used_chars.get(name) {
            // Build a string of all used characters and call mark_chars_used
            // This populates both used_chars AND glyph_set
            let text: String = used_chars.iter().collect();
            font_clone.mark_chars_used(&text);
        }

        // 1. Font file stream (with subsetting if chars were used)
        let font_file = font_clone.create_font_file_stream();
        let font_file_ref = self.context.register(PdfObject::Stream(font_file));

        // Determine if we're subsetting (glyph_set is populated)
        let is_subset = !font_clone.glyph_set.is_empty();

        // 2. Font descriptor (use subset name if subsetting)
        let font_descriptor = font_clone.create_font_descriptor(font_file_ref, is_subset);
        let font_descriptor_ref = self.context.register(PdfObject::Dict(font_descriptor));

        // 3. CIDToGIDMap (for subsetting - maps original GIDs to subset GIDs)
        let cid_to_gid_map_ref = if is_subset {
            font_clone
                .create_cid_to_gid_map()
                .map(|stream| self.context.register(PdfObject::Stream(stream)))
        } else {
            None
        };

        // 4. CIDFont (use original GIDs for widths)
        let mut cid_font =
            font_clone.create_cid_font(font_descriptor_ref, cid_to_gid_map_ref, is_subset);
        // Update widths array (uses original GIDs when subsetting)
        let w_array = font_clone.create_widths_array_for_pdf();
        cid_font.set("W", PdfObject::Array(w_array));
        let cid_font_ref = self.context.register(PdfObject::Dict(cid_font));

        // 5. ToUnicode CMap (uses original GIDs)
        let to_unicode = font_clone.create_to_unicode_cmap_for_pdf();
        let to_unicode_ref = self.context.register(PdfObject::Stream(to_unicode));

        // 6. Type0 font (main font object)
        let type0_font = font_clone.create_type0_font(cid_font_ref, to_unicode_ref, is_subset);
        let font_ref = self.context.register(PdfObject::Dict(type0_font));

        Some(font_ref)
    }

    /// Embeds a TrueType font from a file path
    ///
    /// This method requires both the `fonts` and `std` features.
    #[cfg(all(feature = "fonts", feature = "std"))]
    pub fn embed_font_file<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<String> {
        let data = std::fs::read(path)?;
        self.embed_font(data)
    }

    /// Gets an embedded font by name
    #[cfg(feature = "fonts")]
    pub fn get_embedded_font(&self, name: &str) -> Option<&crate::font::EmbeddedFont> {
        self.embedded_fonts.get(name).map(|f| f.as_ref())
    }

    /// Measures the width of text with the current font
    #[cfg(feature = "fonts")]
    pub fn measure_text(&self, text: &str) -> f64 {
        if self.current_font_embedded {
            if let Some(font) = self.embedded_fonts.get(&self.current_font) {
                let glyphs = font.shape_text(text);
                let total_advance: i32 = glyphs.iter().map(|g| g.x_advance).sum();
                return total_advance as f64 * self.current_font_size / 1000.0;
            }
        }
        // Use proper AFM metrics for standard fonts
        self.measure_standard_font_text(text)
    }

    /// Measures text width using standard font AFM metrics
    fn measure_standard_font_text(&self, text: &str) -> f64 {
        use crate::font::StandardFont;

        if let Some(font) = StandardFont::from_name(&self.current_font) {
            font.string_width(text) as f64 * self.current_font_size / 1000.0
        } else {
            // Fallback for unknown fonts
            text.len() as f64 * self.current_font_size * 0.5
        }
    }

    #[cfg(feature = "fonts")]
    fn draw_embedded_text(&mut self, text: &str, pos: [f64; 2]) {
        let font_name = self.current_font.clone();
        let font_size = self.current_font_size;
        let font = match self.embedded_fonts.get(&font_name) {
            Some(font) => font.clone(),
            None => {
                let content = &mut self.pages[self.current_page].content;
                content.begin_text();
                // Reset spacing to 0 to avoid inheriting from previous calls
                content.set_character_spacing(0.0);
                content.set_word_spacing(0.0);
                content
                    .set_font(&font_name, font_size)
                    .move_text_pos(pos[0], pos[1])
                    .show_text(text)
                    .end_text();
                return;
            }
        };

        let glyphs = font.shape_text(text);
        if glyphs.is_empty() {
            return;
        }

        self.track_font_glyphs(&font_name, &glyphs);

        let has_offsets = glyphs
            .iter()
            .any(|g| g.x_offset != 0 || g.y_offset != 0 || g.y_advance != 0);

        let (glyph_ids, adjustments) = if has_offsets {
            (Vec::new(), Vec::new())
        } else {
            Self::build_glyph_adjustments(font.as_ref(), &glyphs)
        };

        let content = &mut self.pages[self.current_page].content;
        content.begin_text();
        // Reset spacing to 0 to avoid inheriting from previous calls
        content.set_character_spacing(0.0);
        content.set_word_spacing(0.0);
        content.set_font(&font_name, font_size);

        if has_offsets {
            Self::show_positioned_glyphs(content, &glyphs, pos, font_size);
        } else {
            content.move_text_pos(pos[0], pos[1]);
            content.show_text_hex_adjusted(&glyph_ids, &adjustments);
        }

        content.end_text();
    }

    #[cfg(feature = "fonts")]
    fn draw_embedded_text_with_spacing(
        &mut self,
        text: &str,
        pos: [f64; 2],
        char_spacing: f64,
        word_spacing: f64,
    ) {
        let font_name = self.current_font.clone();
        let font_size = self.current_font_size;
        let font = match self.embedded_fonts.get(&font_name) {
            Some(font) => font.clone(),
            None => {
                let content = &mut self.pages[self.current_page].content;
                content.begin_text();
                // Always set spacing to ensure correct values
                content.set_character_spacing(char_spacing);
                content.set_word_spacing(word_spacing);
                content
                    .set_font(&font_name, font_size)
                    .move_text_pos(pos[0], pos[1])
                    .show_text(text)
                    .end_text();
                return;
            }
        };

        let glyphs = font.shape_text(text);
        if glyphs.is_empty() {
            return;
        }

        self.track_font_glyphs(&font_name, &glyphs);

        let has_offsets = glyphs
            .iter()
            .any(|g| g.x_offset != 0 || g.y_offset != 0 || g.y_advance != 0);

        let (glyph_ids, adjustments) = if has_offsets {
            (Vec::new(), Vec::new())
        } else {
            Self::build_glyph_adjustments(font.as_ref(), &glyphs)
        };

        let content = &mut self.pages[self.current_page].content;
        content.begin_text();

        // Always set spacing to ensure correct values
        content.set_character_spacing(char_spacing);
        content.set_word_spacing(word_spacing);

        content.set_font(&font_name, font_size);

        if has_offsets {
            // Use spacing-aware version for positioned glyphs since Tc/Tw
            // don't apply when using set_text_matrix for each glyph
            Self::show_positioned_glyphs_with_spacing(
                content,
                &glyphs,
                pos,
                font_size,
                char_spacing,
                word_spacing,
            );
        } else {
            content.move_text_pos(pos[0], pos[1]);
            content.show_text_hex_adjusted(&glyph_ids, &adjustments);
        }

        content.end_text();
    }

    #[cfg(feature = "fonts")]
    fn track_font_glyphs(&mut self, font_name: &str, glyphs: &[ShapedGlyph]) {
        let used_chars = self
            .font_used_chars
            .entry(font_name.to_string())
            .or_default();
        let used_glyphs = self
            .font_used_glyphs
            .entry(font_name.to_string())
            .or_default();

        for glyph in glyphs {
            used_chars.extend(glyph.text.chars());

            match used_glyphs.get_mut(&glyph.gid) {
                Some(existing) => {
                    if existing.is_empty() && !glyph.text.is_empty() {
                        *existing = glyph.text.clone();
                    }
                }
                None => {
                    used_glyphs.insert(glyph.gid, glyph.text.clone());
                }
            }
        }
    }

    #[cfg(feature = "fonts")]
    fn build_glyph_adjustments(
        font: &crate::font::EmbeddedFont,
        glyphs: &[ShapedGlyph],
    ) -> (Vec<u16>, Vec<i32>) {
        let mut glyph_ids = Vec::with_capacity(glyphs.len());
        let mut adjustments = Vec::with_capacity(glyphs.len().saturating_sub(1));

        for (idx, glyph) in glyphs.iter().enumerate() {
            glyph_ids.push(glyph.gid);
            if idx + 1 < glyphs.len() {
                let default_width = font.glyph_width(glyph.gid) as i32;
                adjustments.push(default_width - glyph.x_advance);
            }
        }

        (glyph_ids, adjustments)
    }

    #[cfg(feature = "fonts")]
    fn show_positioned_glyphs(
        content: &mut ContentBuilder,
        glyphs: &[ShapedGlyph],
        pos: [f64; 2],
        font_size: f64,
    ) {
        Self::show_positioned_glyphs_with_spacing(content, glyphs, pos, font_size, 0.0, 0.0);
    }

    #[cfg(feature = "fonts")]
    fn show_positioned_glyphs_with_spacing(
        content: &mut ContentBuilder,
        glyphs: &[ShapedGlyph],
        pos: [f64; 2],
        font_size: f64,
        char_spacing: f64,
        word_spacing: f64,
    ) {
        let scale = font_size / 1000.0;
        // Convert spacing from text space to font units (round to avoid truncation drift)
        let char_spacing_units = (char_spacing / scale).round() as i32;
        let word_spacing_units = (word_spacing / scale).round() as i32;

        let mut pen_x: i32 = 0;
        let mut pen_y: i32 = 0;
        let glyph_count = glyphs.len();

        for (idx, glyph) in glyphs.iter().enumerate() {
            let x = pos[0] + (pen_x + glyph.x_offset) as f64 * scale;
            let y = pos[1] + (pen_y + glyph.y_offset) as f64 * scale;
            content
                .set_text_matrix(1.0, 0.0, 0.0, 1.0, x, y)
                .show_text_hex(&format!("{:04X}", glyph.gid));

            pen_x += glyph.x_advance;
            pen_y += glyph.y_advance;

            // Apply spacing after each glyph (except the last one)
            // Note: For ligatures, PDF's Tc applies per glyph, not per original character.
            // This matches PDF spec behavior where Tc is added after each "character" (glyph shown).
            if idx + 1 < glyph_count {
                // Add character spacing after this glyph
                pen_x += char_spacing_units;

                // Add word spacing for space characters
                // For multi-char glyphs (rare), check if any char is a space
                if glyph.text.contains(' ') {
                    let space_count = glyph.text.chars().filter(|&c| c == ' ').count();
                    pen_x += word_spacing_units * space_count as i32;
                }
            }
        }
    }

    /// Ensures a font is registered
    pub(crate) fn ensure_font(&mut self, name: &str) {
        if !self.fonts.iter().any(|(n, _)| n == name) {
            // Register the font (only for standard fonts - embedded fonts are already registered)
            if let Some(std_font) = StandardFont::from_name(name) {
                let dict = std_font.to_dict();
                let font_ref = self.context.register(PdfObject::Dict(dict));
                self.fonts.push((name.to_string(), font_ref));
            }
        }
    }

    // =========================================================================
    // Image API
    // =========================================================================

    /// Embeds an image and draws it at the specified position (auto-detects format)
    ///
    /// Automatically detects JPEG or PNG format from the image data.
    /// Accepts multiple source types: byte slices (zero-copy), owned bytes,
    /// or file paths (with `std` feature).
    ///
    /// **Note:** PNG support requires the `png` feature. Without it, PNG images
    /// will return an error at runtime.
    ///
    /// # Example
    /// ```ignore
    /// // From file path (requires "std" feature)
    /// doc.image("photo.jpg", [100.0, 500.0], 200.0, 150.0)?;
    ///
    /// // From bytes (zero-copy)
    /// let data = std::fs::read("photo.jpg")?;
    /// doc.image(&data[..], [100.0, 500.0], 200.0, 150.0)?;
    /// ```
    pub fn image<'a>(
        &mut self,
        source: impl ImageSource<'a>,
        pos: [f64; 2],
        width: f64,
        height: f64,
    ) -> Result<String> {
        let data = source.load()?;
        let image_data = crate::image::embed_image(&data)?;
        self.draw_image_data(image_data, pos, width, height)
    }

    /// Embeds an image and draws it with the specified options (auto-detects format)
    ///
    /// Automatically detects JPEG or PNG format from the image data.
    /// Accepts multiple source types: byte slices (zero-copy), owned bytes,
    /// or file paths (with `std` feature).
    ///
    /// **Note:** PNG support requires the `png` feature.
    pub fn image_with<'a>(
        &mut self,
        source: impl ImageSource<'a>,
        options: ImageOptions,
    ) -> Result<EmbeddedImage> {
        let data = source.load()?;
        let image_data = crate::image::embed_image(&data)?;
        self.embed_and_draw_image_data(image_data, options)
    }

    /// Embeds an image without drawing it (auto-detects format)
    ///
    /// Automatically detects JPEG or PNG format from the image data.
    /// Accepts multiple source types: byte slices (zero-copy), owned bytes,
    /// or file paths (with `std` feature).
    ///
    /// **Note:** PNG support requires the `png` feature.
    ///
    /// Use this when you want to embed an image once and draw it multiple times
    /// or on multiple pages using `draw_embedded_image`.
    pub fn embed_image<'a>(&mut self, source: impl ImageSource<'a>) -> Result<EmbeddedImage> {
        let data = source.load()?;
        let image_data = crate::image::embed_image(&data)?;
        self.embed_image_data(image_data)
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

    /// Embeds a JPEG image and draws it with the specified options
    ///
    /// This method provides more control over image placement and sizing.
    pub fn image_jpeg_with(&mut self, data: &[u8], options: ImageOptions) -> Result<EmbeddedImage> {
        let image_data = crate::image::embed_jpeg(data)?;
        self.embed_and_draw_image_data(image_data, options)
    }

    /// Embeds a PNG image and draws it with the specified options
    ///
    /// This method provides more control over image placement and sizing.
    #[cfg(feature = "png")]
    pub fn image_png_with(&mut self, data: &[u8], options: ImageOptions) -> Result<EmbeddedImage> {
        let image_data = crate::image::embed_png(data)?;
        self.embed_and_draw_image_data(image_data, options)
    }

    /// Embeds a JPEG image without drawing it
    ///
    /// Use this when you want to embed an image once and draw it multiple times
    /// or on multiple pages using `draw_embedded_image`.
    pub fn embed_jpeg(&mut self, data: &[u8]) -> Result<EmbeddedImage> {
        let image_data = crate::image::embed_jpeg(data)?;
        self.embed_image_data(image_data)
    }

    /// Embeds a PNG image without drawing it
    ///
    /// Use this when you want to embed an image once and draw it multiple times
    /// or on multiple pages using `draw_embedded_image`.
    #[cfg(feature = "png")]
    pub fn embed_png(&mut self, data: &[u8]) -> Result<EmbeddedImage> {
        let image_data = crate::image::embed_png(data)?;
        self.embed_image_data(image_data)
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

    /// Draws an embedded image with options
    ///
    /// This allows repositioning and resizing an already-embedded image.
    pub fn draw_embedded_image(
        &mut self,
        image: &EmbeddedImage,
        options: ImageOptions,
    ) -> &mut Self {
        let (x, y, width, height) = self.calculate_image_placement(image, &options);
        self.draw_image(&image.name, [x, y], width, height)
    }

    /// Calculates the final position and size for an image based on options
    fn calculate_image_placement(
        &self,
        image: &EmbeddedImage,
        options: &ImageOptions,
    ) -> (f64, f64, f64, f64) {
        let base_pos = options.at.unwrap_or([0.0, 0.0]);

        // Determine dimensions
        let (mut width, mut height) = if let (Some(w), Some(h)) = (options.width, options.height) {
            // Explicit dimensions
            (w, h)
        } else if let Some((max_w, max_h)) = options.fit {
            // Fit within bounds
            image.fit_dimensions(max_w, max_h)
        } else if let Some(w) = options.width {
            // Width specified, calculate height from aspect ratio
            (w, w / image.aspect_ratio())
        } else if let Some(h) = options.height {
            // Height specified, calculate width from aspect ratio
            (h * image.aspect_ratio(), h)
        } else {
            // Use original dimensions (1 pixel = 1 point)
            (image.width as f64, image.height as f64)
        };

        // Apply scale if specified
        if let Some(scale) = options.scale {
            width *= scale;
            height *= scale;
        }

        // Calculate position offset based on alignment
        let (x_offset, y_offset) = if let Some((bounds_w, bounds_h)) = options.fit {
            options
                .position
                .calculate_offset(width, height, bounds_w, bounds_h)
        } else {
            (0.0, 0.0)
        };

        (
            base_pos[0] + x_offset,
            base_pos[1] + y_offset,
            width,
            height,
        )
    }

    /// Internal: embeds image data without drawing
    fn embed_image_data(&mut self, image_data: crate::image::ImageData) -> Result<EmbeddedImage> {
        // Generate unique name
        self.image_counter += 1;
        let name = format!("Im{}", self.image_counter);

        let img_width = image_data.width;
        let img_height = image_data.height;

        // Create XObject stream
        let xobject = image_data.to_xobject();

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

        Ok(EmbeddedImage {
            name,
            width: img_width,
            height: img_height,
        })
    }

    /// Internal: embeds image data and draws it with options
    fn embed_and_draw_image_data(
        &mut self,
        image_data: crate::image::ImageData,
        options: ImageOptions,
    ) -> Result<EmbeddedImage> {
        let image = self.embed_image_data(image_data)?;
        self.draw_embedded_image(&image, options);
        Ok(image)
    }

    /// Internal: draws image data and returns the image name (legacy)
    fn draw_image_data(
        &mut self,
        image_data: crate::image::ImageData,
        pos: [f64; 2],
        width: f64,
        height: f64,
    ) -> Result<String> {
        let image = self.embed_image_data(image_data)?;
        self.draw_image(&image.name, pos, width, height);
        Ok(image.name)
    }

    // =========================================================================
    // SVG API
    // =========================================================================

    /// Renders an SVG at the specified position and size.
    ///
    /// This method supports SVG paths and basic shapes (rect, circle, ellipse, etc.),
    /// which are automatically converted to paths. The SVG is rendered directly into
    /// the PDF content stream.
    ///
    /// # Supported Features
    ///
    /// - Paths (`<path>`)
    /// - Basic shapes (`<rect>`, `<circle>`, `<ellipse>`, `<line>`, `<polygon>`, `<polyline>`)
    /// - Fill and stroke with RGB colors
    /// - Stroke styles (width, linecap, linejoin, dasharray)
    /// - Transforms (translate, rotate, scale, matrix)
    /// - Nested groups (`<g>`)
    /// - Fill rules (even-odd, non-zero)
    ///
    /// # Unsupported Features
    ///
    /// - Gradients (linearGradient, radialGradient)
    /// - Patterns
    /// - Images (`<image>`)
    /// - Text (`<text>`)
    /// - Filters and effects
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::prelude::*;
    ///
    /// let mut doc = Document::new();
    /// let svg = r#"
    ///     <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
    ///         <rect x="10" y="10" width="80" height="80" fill="red"/>
    ///     </svg>
    /// "#;
    /// doc.draw_svg(svg, [100.0, 700.0], 200.0, 200.0)?;
    /// # Ok::<(), pdfcrate::Error>(())
    /// ```
    ///
    /// Requires the `svg` feature.
    #[cfg(feature = "svg")]
    pub fn draw_svg(&mut self, svg: &str, pos: [f64; 2], width: f64, height: f64) -> Result<()> {
        let content = &mut self.pages[self.current_page].content;
        crate::svg::render_svg_paths(content, svg, pos, width, height)
    }

    /// Renders SVG paths at the specified position and size.
    ///
    /// **Deprecated**: Use [`draw_svg`](Self::draw_svg) instead.
    ///
    /// This method is kept for backward compatibility but will be removed in a future version.
    #[cfg(feature = "svg")]
    #[deprecated(note = "Use draw_svg instead")]
    pub fn draw_svg_paths(
        &mut self,
        svg: &str,
        pos: [f64; 2],
        width: f64,
        height: f64,
    ) -> Result<()> {
        self.draw_svg(svg, pos, width, height)
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

    /// Applies transparency to operations within the closure
    ///
    /// # Arguments
    ///
    /// * `opacity` - Opacity value from 0.0 (fully transparent) to 1.0 (fully opaque)
    /// * `f` - Closure containing operations to be drawn with transparency
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::Document;
    ///
    /// let mut doc = Document::new();
    /// doc.transparent(0.5, |doc| {
    ///     doc.fill(|ctx| {
    ///         ctx.color(1.0, 0.0, 0.0);
    ///         ctx.rectangle([100.0, 100.0], 200.0, 100.0);
    ///     });
    /// });
    /// ```
    pub fn transparent<F>(&mut self, opacity: f64, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        // Clamp opacity to valid range
        let opacity = opacity.clamp(0.0, 1.0);

        // Create ExtGState dictionary for transparency
        let gs_ref = self.context.alloc_ref();
        let gs_name = format!("GS{}", gs_ref.object_number());

        let mut gs_dict = PdfDict::new();
        gs_dict.set("Type", PdfObject::Name("ExtGState".into()));
        gs_dict.set("CA", PdfObject::Real(opacity)); // Stroke alpha
        gs_dict.set("ca", PdfObject::Real(opacity)); // Fill alpha

        self.context.assign(gs_ref, PdfObject::Dict(gs_dict));

        // Add to current page resources
        let page = &mut self.pages[self.current_page];
        page.content.save_state();
        page.content.set_graphics_state(&gs_name);

        // Store the gs resource reference for later use during render
        // We'll need to add it to the page's ExtGState resources
        self.ensure_extgstate_resource(&gs_name, gs_ref);

        f(self);

        self.pages[self.current_page].content.restore_state();
        self
    }

    /// Ensures ExtGState resource is registered (internal helper)
    fn ensure_extgstate_resource(&mut self, name: &str, gs_ref: PdfRef) {
        // Add to ext_gstates if not already present
        if !self.ext_gstates.iter().any(|(n, _)| n == name) {
            self.ext_gstates.push((name.to_string(), gs_ref));
        }
    }

    /// Draws X and Y coordinate axes with tick marks and labels
    ///
    /// This is a helper method for visualizing PDF coordinates, similar to Prawn's
    /// `stroke_axis`. It draws dashed axes with tick marks at regular intervals
    /// and coordinate labels.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, AxisOptions};
    ///
    /// let mut doc = Document::new();
    /// // Draw axes with default options
    /// doc.stroke_axis(AxisOptions::default());
    ///
    /// // Or with custom options
    /// doc.stroke_axis(
    ///     AxisOptions::new()
    ///         .at(50.0, 50.0)
    ///         .step_length(50.0)
    ///         .color(0.5, 0.5, 0.5)
    /// );
    /// ```
    pub fn stroke_axis(&mut self, options: AxisOptions) -> &mut Self {
        let page = &self.pages[self.current_page];
        let (page_width, page_height) = page.size.dimensions(page.layout);
        let at = options.at;
        let width = options.width.unwrap_or(page_width - at[0]);
        let height = options.height.unwrap_or(page_height - at[1]);
        let step = options.step_length;
        let neg_len = options.negative_axes_length;
        let [r, g, b] = options.color;

        // Save graphics state
        self.pages[self.current_page].content.save_state();

        // Set colors
        self.pages[self.current_page]
            .content
            .set_stroke_color_rgb(r, g, b);
        self.pages[self.current_page]
            .content
            .set_fill_color_rgb(r, g, b);

        // Draw dashed axes (dash length 1, space 4 - matches Prawn)
        self.pages[self.current_page]
            .content
            .set_dash(&[1.0, 4.0], 0.0);

        // Horizontal axis (X)
        self.pages[self.current_page]
            .content
            .move_to(at[0] - neg_len, at[1])
            .line_to(at[0] + width, at[1])
            .stroke();

        // Vertical axis (Y)
        self.pages[self.current_page]
            .content
            .move_to(at[0], at[1] - neg_len)
            .line_to(at[0], at[1] + height)
            .stroke();

        // Clear dash for circles
        self.pages[self.current_page].content.clear_dash();

        // Draw origin circle
        self.pages[self.current_page]
            .content
            .circle(at[0], at[1], 1.0)
            .fill();

        // Ensure Helvetica font is registered for labels
        self.ensure_font("Helvetica");
        let font_name = "Helvetica";

        // Draw X axis tick marks and labels
        let mut point = step;
        while point <= width {
            let x = at[0] + point;
            // Tick mark circle
            self.pages[self.current_page]
                .content
                .circle(x, at[1], 1.0)
                .fill();

            // Label
            let label = format!("{}", point as i32);
            self.pages[self.current_page]
                .content
                .begin_text()
                .set_font(&font_name, 7.0)
                .set_text_matrix(1.0, 0.0, 0.0, 1.0, x - 5.0, at[1] - 10.0)
                .show_text(&label)
                .end_text();

            point += step;
        }

        // Draw Y axis tick marks and labels
        let mut point = step;
        while point <= height {
            let y = at[1] + point;
            // Tick mark circle
            self.pages[self.current_page]
                .content
                .circle(at[0], y, 1.0)
                .fill();

            // Label
            let label = format!("{}", point as i32);
            self.pages[self.current_page]
                .content
                .begin_text()
                .set_font(&font_name, 7.0)
                .set_text_matrix(1.0, 0.0, 0.0, 1.0, at[0] - 17.0, y - 2.0)
                .show_text(&label)
                .end_text();

            point += step;
        }

        // Restore graphics state
        self.pages[self.current_page].content.restore_state();
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

    /// Sets creation date to the current time.
    ///
    /// By default, creation_date is None for reproducible builds.
    /// Call this method if you want to include the current timestamp.
    #[cfg(feature = "std")]
    pub fn creation_date_now(&mut self) -> &mut Self {
        self.info.set_creation_date_now();
        self
    }

    /// Sets modification date to the current time.
    ///
    /// By default, mod_date is None for reproducible builds.
    /// Call this method if you want to include the current timestamp.
    #[cfg(feature = "std")]
    pub fn mod_date_now(&mut self) -> &mut Self {
        self.info.set_mod_date_now();
        self
    }

    // =========================================================================
    // Form API
    // =========================================================================

    /// Adds a text field to the current page
    ///
    /// # Arguments
    /// * `name` - Unique field name
    /// * `rect` - Bounding rectangle [x1, y1, x2, y2]
    ///
    /// # Example
    /// ```rust,ignore
    /// doc.add_text_field("username", [100.0, 700.0, 300.0, 720.0]);
    /// ```
    pub fn add_text_field(&mut self, name: impl Into<String>, rect: [f64; 4]) -> &mut Self {
        let mut field = FormField::text(name, rect);
        field.page_index = self.current_page;
        self.form.add_field(field);
        self
    }

    /// Adds a text field with custom options using a builder pattern
    pub fn add_text_field_with<F>(
        &mut self,
        name: impl Into<String>,
        rect: [f64; 4],
        f: F,
    ) -> &mut Self
    where
        F: FnOnce(FormField) -> FormField,
    {
        let field = FormField::text(name, rect);
        let mut field = f(field);
        field.page_index = self.current_page;
        self.form.add_field(field);
        self
    }

    /// Adds a multiline text field (text area)
    pub fn add_text_area(&mut self, name: impl Into<String>, rect: [f64; 4]) -> &mut Self {
        let mut field = FormField::text(name, rect).multiline();
        field.page_index = self.current_page;
        self.form.add_field(field);
        self
    }

    /// Adds a checkbox field
    ///
    /// # Arguments
    /// * `name` - Unique field name
    /// * `rect` - Bounding rectangle [x1, y1, x2, y2]
    /// * `checked` - Initial checked state
    pub fn add_checkbox(
        &mut self,
        name: impl Into<String>,
        rect: [f64; 4],
        checked: bool,
    ) -> &mut Self {
        let mut field = FormField::checkbox(name, rect, checked);
        field.page_index = self.current_page;
        self.form.add_field(field);
        self
    }

    /// Adds a dropdown (combo box) field
    ///
    /// # Arguments
    /// * `name` - Unique field name
    /// * `rect` - Bounding rectangle [x1, y1, x2, y2]
    /// * `options` - List of options to choose from
    pub fn add_dropdown<S: Into<String>>(
        &mut self,
        name: impl Into<String>,
        rect: [f64; 4],
        options: Vec<S>,
    ) -> &mut Self {
        let options: Vec<String> = options.into_iter().map(Into::into).collect();
        let mut field = FormField::dropdown(name, rect, options);
        field.page_index = self.current_page;
        self.form.add_field(field);
        self
    }

    /// Adds a list box field
    ///
    /// # Arguments
    /// * `name` - Unique field name
    /// * `rect` - Bounding rectangle [x1, y1, x2, y2]
    /// * `options` - List of options to choose from
    pub fn add_listbox<S: Into<String>>(
        &mut self,
        name: impl Into<String>,
        rect: [f64; 4],
        options: Vec<S>,
    ) -> &mut Self {
        let options: Vec<String> = options.into_iter().map(Into::into).collect();
        let mut field = FormField::listbox(name, rect, options);
        field.page_index = self.current_page;
        self.form.add_field(field);
        self
    }

    /// Returns true if the document has form fields
    pub fn has_form(&self) -> bool {
        self.form.has_fields()
    }

    /// Returns the number of form fields
    pub fn form_field_count(&self) -> usize {
        self.form.fields.len()
    }

    // =========================================================================
    // PDF Embedding API
    // =========================================================================

    /// Embeds a page from a loaded PDF document
    ///
    /// This extracts a page from an existing PDF and embeds it as a Form XObject
    /// that can be drawn on pages in this document.
    ///
    /// # Example
    /// ```rust,ignore
    /// let source_pdf = std::fs::read("source.pdf")?;
    /// let mut source = LoadedDocument::load(source_pdf)?;
    /// let page = doc.embed_pdf_page(&mut source, 0)?;
    /// doc.draw_pdf_page(&page, [50.0, 400.0], 200.0, 300.0);
    /// ```
    pub fn embed_pdf_page(
        &mut self,
        source: &mut crate::document::LoadedDocument,
        page_index: usize,
    ) -> Result<crate::document::EmbeddedPage> {
        let mut embedded = source.extract_page(page_index)?;

        // Generate unique name
        self.image_counter += 1;
        embedded.name = format!("Pg{}", self.image_counter);

        // Register the XObject
        let xobject = embedded.xobject.clone();
        let xobject_ref = self.context.register(PdfObject::Stream(xobject));
        self.images.push((
            embedded.name.clone(),
            xobject_ref,
            embedded.width as u32,
            embedded.height as u32,
        ));

        Ok(embedded)
    }

    /// Embeds all pages from a loaded PDF document
    ///
    /// Returns a vector of embedded pages that can be drawn.
    pub fn embed_pdf(
        &mut self,
        source: &mut crate::document::LoadedDocument,
    ) -> Result<Vec<crate::document::EmbeddedPage>> {
        let count = source.page_count()?;
        let mut pages = Vec::with_capacity(count);
        for i in 0..count {
            pages.push(self.embed_pdf_page(source, i)?);
        }
        Ok(pages)
    }

    /// Draws an embedded PDF page at the specified position and size
    ///
    /// # Arguments
    /// * `page` - The embedded page to draw
    /// * `pos` - Position [x, y] for the bottom-left corner
    /// * `width` - Width to draw the page
    /// * `height` - Height to draw the page
    pub fn draw_pdf_page(
        &mut self,
        page: &crate::document::EmbeddedPage,
        pos: [f64; 2],
        width: f64,
        height: f64,
    ) -> &mut Self {
        // Calculate scale factors
        let scale_x = width / page.width;
        let scale_y = height / page.height;

        let content = &mut self.pages[self.current_page].content;
        content
            .save_state()
            .concat_matrix(scale_x, 0.0, 0.0, scale_y, pos[0], pos[1])
            .draw_xobject(&page.name)
            .restore_state();
        self
    }

    /// Draws an embedded PDF page scaled to fit within bounds
    ///
    /// The page is scaled proportionally to fit within the specified bounds.
    pub fn draw_pdf_page_fit(
        &mut self,
        page: &crate::document::EmbeddedPage,
        pos: [f64; 2],
        max_width: f64,
        max_height: f64,
    ) -> &mut Self {
        let (width, height) = page.fit_dimensions(max_width, max_height);
        self.draw_pdf_page(page, pos, width, height)
    }

    /// Copies pages from a source PDF and appends them to this document
    ///
    /// This is a simplified merge operation that copies entire pages.
    /// For more control, use `embed_pdf_page` and `draw_pdf_page`.
    ///
    /// # Arguments
    /// * `source` - The source PDF document
    /// * `page_indices` - Indices of pages to copy (0-based)
    pub fn copy_pages(
        &mut self,
        source: &mut crate::document::LoadedDocument,
        page_indices: &[usize],
    ) -> Result<&mut Self> {
        for &page_index in page_indices {
            // Extract the page
            let embedded = self.embed_pdf_page(source, page_index)?;

            // Create a new page with the same size
            self.pages.push(PageData {
                content: ContentBuilder::new(),
                size: PageSize::Custom(embedded.width, embedded.height),
                layout: PageLayout::Portrait,
                annotations: Vec::new(),
            });
            self.current_page = self.pages.len() - 1;

            // Draw the embedded page filling the entire new page
            self.draw_pdf_page(&embedded, [0.0, 0.0], embedded.width, embedded.height);
        }
        Ok(self)
    }

    /// Copies all pages from a source PDF and appends them to this document
    pub fn copy_all_pages(
        &mut self,
        source: &mut crate::document::LoadedDocument,
    ) -> Result<&mut Self> {
        let count = source.page_count()?;
        let indices: Vec<usize> = (0..count).collect();
        self.copy_pages(source, &indices)
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
        // Create font objects for embedded fonts (with subsetting)
        #[cfg(feature = "fonts")]
        {
            let embedded_font_names: Vec<String> = self.embedded_fonts.keys().cloned().collect();
            for name in embedded_font_names {
                if let Some(font_ref) = self.create_font_objects(&name) {
                    self.fonts.push((name, font_ref));
                }
            }
        }

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

        // Build ExtGState resources dictionary (transparency)
        let mut extgstate_dict = PdfDict::new();
        for (name, gs_ref) in &self.ext_gstates {
            extgstate_dict.set(name, PdfObject::Reference(*gs_ref));
        }

        let mut resources = PdfDict::new();
        if !self.fonts.is_empty() {
            resources.set("Font", PdfObject::Dict(font_dict.clone()));
        }
        if !self.images.is_empty() {
            resources.set("XObject", PdfObject::Dict(xobject_dict));
        }
        if !self.ext_gstates.is_empty() {
            resources.set("ExtGState", PdfObject::Dict(extgstate_dict));
        }
        let resources_ref = self.context.register(PdfObject::Dict(resources));

        // Create page objects (first pass: without annotations)
        let pages_ref = self.context.alloc_ref();
        let mut page_refs = Vec::new();
        let mut page_annotations: Vec<Vec<link::LinkAnnotation>> = Vec::new();

        for page in self.pages.iter_mut() {
            // Create content stream
            let content_data = std::mem::take(&mut page.content).build();
            let content_stream = PdfStream::from_data_compressed(content_data);
            let content_ref = self.context.register(PdfObject::Stream(content_stream));

            // Create page dictionary (without annotations for now)
            let dims = page.size.dimensions(page.layout);
            let media_box = [0.0, 0.0, dims.0, dims.1];
            let page_dict =
                create_page(pages_ref, media_box, Some(resources_ref), Some(content_ref));

            // Collect annotations for later processing
            let annotations = std::mem::take(&mut page.annotations);
            page_annotations.push(annotations);

            let page_ref = self.context.register(PdfObject::Dict(page_dict));
            page_refs.push(page_ref);
        }

        // Second pass: add link annotations (now that all page_refs are available)
        let mut page_link_annots: std::collections::HashMap<usize, Vec<PdfRef>> =
            std::collections::HashMap::new();

        for (page_idx, annotations) in page_annotations.into_iter().enumerate() {
            if annotations.is_empty() {
                continue;
            }

            let page_ref = page_refs[page_idx];
            let mut annot_refs = Vec::new();

            for annotation in annotations {
                // Pass page_refs to resolve internal page links
                let annot_dict = annotation.to_dict(Some(page_ref), Some(&page_refs));
                let annot_ref = self.context.register(PdfObject::Dict(annot_dict));
                annot_refs.push(annot_ref);
            }

            // Add Annots array to page
            let annots: Vec<PdfObject> = annot_refs
                .iter()
                .map(|r| PdfObject::Reference(*r))
                .collect();

            // Update the page with annotations
            if let Some(PdfObject::Dict(ref page_dict)) = self.context.lookup(page_ref) {
                let mut updated_page = page_dict.clone();
                updated_page.set("Annots", PdfObject::Array(PdfArray::from(annots)));
                self.context.assign(page_ref, PdfObject::Dict(updated_page));
            }

            // Track for potential merging with form fields
            page_link_annots.insert(page_idx, annot_refs);
        }

        // Create pages dictionary
        let pages_dict = create_pages(page_refs.clone(), page_refs.len() as i64);
        self.context.assign(pages_ref, PdfObject::Dict(pages_dict));

        // Create catalog (may be modified later with AcroForm)
        let mut catalog = create_catalog(pages_ref);

        // Handle form fields if present
        let acro_form_ref = if self.form.has_fields() {
            // Create field widget annotations and appearance streams
            // Group field refs by page index
            let mut field_refs_all = Vec::new();
            let mut page_annots: std::collections::HashMap<usize, Vec<PdfRef>> =
                std::collections::HashMap::new();

            for field in &self.form.fields {
                // Get the page reference for this field
                let page_ref = page_refs
                    .get(field.page_index)
                    .copied()
                    .unwrap_or_else(|| page_refs.last().copied().unwrap_or(pages_ref));

                // Generate appearance stream for the field
                let appearance_stream = crate::forms::generate_appearance(field, None);
                let appearance_ref = self.context.register(PdfObject::Stream(appearance_stream));

                // Create widget annotation dictionary with correct page reference
                let widget =
                    crate::forms::create_widget_annotation(field, page_ref, Some(appearance_ref));
                let widget_ref = self.context.register(PdfObject::Dict(widget));
                field_refs_all.push(widget_ref);

                // Group by page index
                page_annots
                    .entry(field.page_index)
                    .or_default()
                    .push(widget_ref);
            }

            // Build font references for AcroForm default resources
            let font_refs: Vec<(String, PdfRef)> = self
                .fonts
                .iter()
                .map(|(name, r)| (name.clone(), *r))
                .collect();

            // Create AcroForm dictionary
            let acro_form_dict = crate::forms::create_acro_form_dict(
                &field_refs_all,
                &font_refs,
                self.form.need_appearances,
                self.form.default_appearance.as_deref(),
            );
            let acro_form_ref = self.context.register(PdfObject::Dict(acro_form_dict));

            // Add form annotations to each page that has fields
            // Merge with existing link annotations if present
            for (page_idx, form_annot_refs) in page_annots {
                if let Some(&page_ref) = page_refs.get(page_idx) {
                    // Combine with any existing link annotations
                    let mut all_annots: Vec<PdfObject> = Vec::new();

                    // Add existing link annotations first
                    if let Some(link_refs) = page_link_annots.get(&page_idx) {
                        for r in link_refs {
                            all_annots.push(PdfObject::Reference(*r));
                        }
                    }

                    // Add form annotations
                    for r in &form_annot_refs {
                        all_annots.push(PdfObject::Reference(*r));
                    }

                    let annots_array = PdfArray::from(all_annots);

                    // Update the page with merged annotations
                    if let Some(PdfObject::Dict(ref page_dict)) = self.context.lookup(page_ref) {
                        let mut updated_page = page_dict.clone();
                        updated_page.set("Annots", PdfObject::Array(annots_array));
                        self.context.assign(page_ref, PdfObject::Dict(updated_page));
                    }
                }
            }

            Some(acro_form_ref)
        } else {
            None
        };

        // Add AcroForm to catalog if present
        if let Some(acro_form_ref) = acro_form_ref {
            catalog.set("AcroForm", PdfObject::Reference(acro_form_ref));
        }

        // Add Named Destinations if present
        if !self.destinations.is_empty() {
            // Build sorted name-destination pairs for the Names array
            let mut dest_pairs: Vec<(&String, &(usize, link::DestinationFit))> =
                self.destinations.iter().collect();
            dest_pairs.sort_by(|a, b| a.0.cmp(b.0));

            let mut names_array = PdfArray::new();
            for (name, (page_index, fit)) in dest_pairs {
                // Add name string
                names_array.push(PdfObject::String(PdfString::from(name.as_str())));

                // Build destination array [page_ref /FitType ...]
                let mut dest_array = PdfArray::new();
                if let Some(&page_ref) = page_refs.get(*page_index) {
                    dest_array.push(PdfObject::Reference(page_ref));
                } else {
                    // Fallback to integer (shouldn't happen)
                    dest_array.push(PdfObject::Integer(*page_index as i64));
                }

                match fit {
                    link::DestinationFit::Fit => {
                        dest_array.push(PdfObject::Name(PdfName::new("Fit")));
                    }
                    link::DestinationFit::FitH(top) => {
                        dest_array.push(PdfObject::Name(PdfName::new("FitH")));
                        dest_array.push(match top {
                            Some(t) => PdfObject::Real(*t),
                            None => PdfObject::Null,
                        });
                    }
                    link::DestinationFit::FitV(left) => {
                        dest_array.push(PdfObject::Name(PdfName::new("FitV")));
                        dest_array.push(match left {
                            Some(l) => PdfObject::Real(*l),
                            None => PdfObject::Null,
                        });
                    }
                    link::DestinationFit::FitR {
                        left,
                        bottom,
                        right,
                        top,
                    } => {
                        dest_array.push(PdfObject::Name(PdfName::new("FitR")));
                        dest_array.push(PdfObject::Real(*left));
                        dest_array.push(PdfObject::Real(*bottom));
                        dest_array.push(PdfObject::Real(*right));
                        dest_array.push(PdfObject::Real(*top));
                    }
                    link::DestinationFit::XYZ { left, top, zoom } => {
                        dest_array.push(PdfObject::Name(PdfName::new("XYZ")));
                        dest_array.push(match left {
                            Some(l) => PdfObject::Real(*l),
                            None => PdfObject::Null,
                        });
                        dest_array.push(match top {
                            Some(t) => PdfObject::Real(*t),
                            None => PdfObject::Null,
                        });
                        dest_array.push(match zoom {
                            Some(z) => PdfObject::Real(*z),
                            None => PdfObject::Null,
                        });
                    }
                }

                names_array.push(PdfObject::Array(dest_array));
            }

            // Create Dests name tree
            let mut dests_dict = PdfDict::new();
            dests_dict.set("Names", PdfObject::Array(names_array));

            // Create Names dictionary
            let mut names_dict = PdfDict::new();
            names_dict.set("Dests", PdfObject::Dict(dests_dict));

            catalog.set("Names", PdfObject::Dict(names_dict));
        }

        // Add Document Outline (Bookmarks) if present
        if !self.outline.is_empty() {
            let outlines_ref = self.build_outline_tree(&page_refs);
            catalog.set("Outlines", PdfObject::Reference(outlines_ref));
        }

        let catalog_ref = self.context.register(PdfObject::Dict(catalog));

        // Note: mod_date is NOT auto-set for reproducible builds.
        // Users can call info.set_mod_date_now() before render() if needed.

        // Pre-format dates to determine if they're valid (post-1970)
        // This ensures has_info is based on actual content, not just Option::is_some()
        #[cfg(feature = "std")]
        let creation_date_str = self.info.creation_date.and_then(|d| format_pdf_date(d));
        #[cfg(feature = "std")]
        let mod_date_str = self.info.mod_date.and_then(|d| format_pdf_date(d));

        // Create info dictionary only if there's actual content
        let has_info = self.info.title.is_some()
            || self.info.author.is_some()
            || self.info.producer.is_some()
            || self.info.subject.is_some()
            || self.info.creator.is_some()
            || {
                #[cfg(feature = "std")]
                {
                    creation_date_str.is_some() || mod_date_str.is_some()
                }
                #[cfg(not(feature = "std"))]
                {
                    false
                }
            };

        let info_ref = if has_info {
            let mut info_dict = PdfDict::new();
            if let Some(title) = &self.info.title {
                info_dict.set("Title", PdfObject::String(title.as_str().into()));
            }
            if let Some(author) = &self.info.author {
                info_dict.set("Author", PdfObject::String(author.as_str().into()));
            }
            if let Some(subject) = &self.info.subject {
                info_dict.set("Subject", PdfObject::String(subject.as_str().into()));
            }
            if let Some(creator) = &self.info.creator {
                info_dict.set("Creator", PdfObject::String(creator.as_str().into()));
            }
            if let Some(producer) = &self.info.producer {
                info_dict.set("Producer", PdfObject::String(producer.as_str().into()));
            }
            #[cfg(feature = "std")]
            {
                if let Some(date_str) = creation_date_str {
                    info_dict.set("CreationDate", PdfObject::String(date_str.into()));
                }
                if let Some(date_str) = mod_date_str {
                    info_dict.set("ModDate", PdfObject::String(date_str.into()));
                }
            }
            Some(self.context.register(PdfObject::Dict(info_dict)))
        } else {
            None
        };

        // Write PDF
        let objects: Vec<(PdfRef, PdfObject)> = self.context.to_vec();
        crate::writer::write_pdf(&objects, catalog_ref, info_ref)
    }

    /// Build the outline tree and return the root reference
    fn build_outline_tree(&mut self, page_refs: &[PdfRef]) -> PdfRef {
        // Pre-allocate root reference
        let root_ref = self.context.alloc_ref();

        // Build all outline items recursively
        // Each item needs: Title, Parent, Prev, Next, First, Last, Count, Dest
        let (first_ref, last_ref, count) =
            self.build_outline_items(&self.outline.items.clone(), root_ref, page_refs);

        // Create root dictionary
        let mut root_dict = PdfDict::new();
        root_dict.set("Type", PdfObject::Name(PdfName::new("Outlines")));
        root_dict.set("Count", PdfObject::Integer(count as i64));
        if let Some(first) = first_ref {
            root_dict.set("First", PdfObject::Reference(first));
        }
        if let Some(last) = last_ref {
            root_dict.set("Last", PdfObject::Reference(last));
        }

        self.context.assign(root_ref, PdfObject::Dict(root_dict));
        root_ref
    }

    /// Recursively build outline items at a given level
    /// Returns (first_ref, last_ref, total_count)
    fn build_outline_items(
        &mut self,
        items: &[outline::OutlineItem],
        parent_ref: PdfRef,
        page_refs: &[PdfRef],
    ) -> (Option<PdfRef>, Option<PdfRef>, usize) {
        if items.is_empty() {
            return (None, None, 0);
        }

        let mut first_ref: Option<PdfRef> = None;
        let mut prev_ref: Option<PdfRef> = None;
        let mut total_count = 0;

        // Pre-allocate refs for all items at this level
        let item_refs: Vec<PdfRef> = items.iter().map(|_| self.context.alloc_ref()).collect();

        for (i, item) in items.iter().enumerate() {
            let item_ref = item_refs[i];
            let next_ref = item_refs.get(i + 1).copied();

            // Build children first to get their count
            let (child_first, child_last, child_count) =
                self.build_outline_items(&item.children, item_ref, page_refs);

            // Create item dictionary
            let mut item_dict = PdfDict::new();
            item_dict.set(
                "Title",
                PdfObject::String(PdfString::from(item.title.as_str())),
            );
            item_dict.set("Parent", PdfObject::Reference(parent_ref));

            // Count: positive if open, negative if closed
            let display_count = if item.closed {
                -(child_count as i64)
            } else {
                child_count as i64
            };
            if child_count > 0 {
                item_dict.set("Count", PdfObject::Integer(display_count));
            }

            // Navigation links
            if let Some(prev) = prev_ref {
                item_dict.set("Prev", PdfObject::Reference(prev));
            }
            if let Some(next) = next_ref {
                item_dict.set("Next", PdfObject::Reference(next));
            }
            if let Some(first) = child_first {
                item_dict.set("First", PdfObject::Reference(first));
            }
            if let Some(last) = child_last {
                item_dict.set("Last", PdfObject::Reference(last));
            }

            // Destination
            if let Some(dest) = &item.destination {
                match dest {
                    outline::OutlineDestination::Page { page_index, fit } => {
                        let mut dest_array = PdfArray::new();
                        if let Some(&page_ref) = page_refs.get(*page_index) {
                            dest_array.push(PdfObject::Reference(page_ref));
                        } else {
                            dest_array.push(PdfObject::Integer(*page_index as i64));
                        }

                        match fit {
                            link::DestinationFit::Fit => {
                                dest_array.push(PdfObject::Name(PdfName::new("Fit")));
                            }
                            link::DestinationFit::FitH(top) => {
                                dest_array.push(PdfObject::Name(PdfName::new("FitH")));
                                dest_array.push(match top {
                                    Some(t) => PdfObject::Real(*t),
                                    None => PdfObject::Null,
                                });
                            }
                            link::DestinationFit::FitV(left) => {
                                dest_array.push(PdfObject::Name(PdfName::new("FitV")));
                                dest_array.push(match left {
                                    Some(l) => PdfObject::Real(*l),
                                    None => PdfObject::Null,
                                });
                            }
                            link::DestinationFit::FitR {
                                left,
                                bottom,
                                right,
                                top,
                            } => {
                                dest_array.push(PdfObject::Name(PdfName::new("FitR")));
                                dest_array.push(PdfObject::Real(*left));
                                dest_array.push(PdfObject::Real(*bottom));
                                dest_array.push(PdfObject::Real(*right));
                                dest_array.push(PdfObject::Real(*top));
                            }
                            link::DestinationFit::XYZ { left, top, zoom } => {
                                dest_array.push(PdfObject::Name(PdfName::new("XYZ")));
                                dest_array.push(match left {
                                    Some(l) => PdfObject::Real(*l),
                                    None => PdfObject::Null,
                                });
                                dest_array.push(match top {
                                    Some(t) => PdfObject::Real(*t),
                                    None => PdfObject::Null,
                                });
                                dest_array.push(match zoom {
                                    Some(z) => PdfObject::Real(*z),
                                    None => PdfObject::Null,
                                });
                            }
                        }
                        item_dict.set("Dest", PdfObject::Array(dest_array));
                    }
                    outline::OutlineDestination::Named(name) => {
                        item_dict.set("Dest", PdfObject::String(PdfString::from(name.as_str())));
                    }
                }
            }

            self.context.assign(item_ref, PdfObject::Dict(item_dict));

            // Track first/last/prev
            if first_ref.is_none() {
                first_ref = Some(item_ref);
            }
            prev_ref = Some(item_ref);

            // Update total count: 1 for this item + children if open
            total_count += 1;
            if !item.closed {
                total_count += child_count;
            }
        }

        (first_ref, prev_ref, total_count)
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

    /// Draws and strokes a line
    pub fn line(&mut self, from: [f64; 2], to: [f64; 2]) -> &mut Self {
        self.content.move_to(from[0], from[1]).line_to(to[0], to[1]);
        self.content.stroke();
        self
    }

    /// Draws and strokes a rectangle
    ///
    /// The origin is the bottom-left corner (PDF native coordinates).
    pub fn rectangle(&mut self, origin: [f64; 2], width: f64, height: f64) -> &mut Self {
        self.content.rect(origin[0], origin[1], width, height);
        self.content.stroke();
        self
    }

    /// Draws and strokes a rectangle with top-left origin (Prawn-style)
    ///
    /// This is an alias for `rectangle` that accepts the top-left corner
    /// as the origin point, matching Prawn's coordinate convention.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::Document;
    ///
    /// let mut doc = Document::new();
    /// // Draw a rectangle with top-left at (100, 500)
    /// doc.stroke(|ctx| {
    ///     ctx.rect_tl([100.0, 500.0], 200.0, 100.0);
    /// });
    /// ```
    pub fn rect_tl(&mut self, top_left: [f64; 2], width: f64, height: f64) -> &mut Self {
        // Convert top-left to bottom-left (PDF native)
        let bottom_left = [top_left[0], top_left[1] - height];
        self.rectangle(bottom_left, width, height)
    }

    /// Draws and strokes a rounded rectangle
    ///
    /// The origin is the bottom-left corner (PDF native coordinates).
    pub fn rounded_rectangle(
        &mut self,
        origin: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        self.content
            .rounded_rect(origin[0], origin[1], width, height, radius);
        self.content.stroke();
        self
    }

    /// Draws and strokes a rounded rectangle with top-left origin (Prawn-style)
    ///
    /// This is an alias for `rounded_rectangle` that accepts the top-left corner
    /// as the origin point, matching Prawn's coordinate convention.
    pub fn rounded_rect_tl(
        &mut self,
        top_left: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        // Convert top-left to bottom-left (PDF native)
        let bottom_left = [top_left[0], top_left[1] - height];
        self.rounded_rectangle(bottom_left, width, height, radius)
    }

    /// Draws and strokes a circle
    pub fn circle(&mut self, center: [f64; 2], radius: f64) -> &mut Self {
        self.content.circle(center[0], center[1], radius);
        self.content.stroke();
        self
    }

    /// Draws and strokes an ellipse
    pub fn ellipse(&mut self, center: [f64; 2], rx: f64, ry: f64) -> &mut Self {
        self.content.ellipse(center[0], center[1], rx, ry);
        self.content.stroke();
        self
    }

    /// Moves to a point (for path building)
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.content.move_to(x, y);
        self
    }

    /// Draws a line to a point (for path building)
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.content.line_to(x, y);
        self
    }

    /// Draws a cubic Bezier curve (for path building)
    pub fn curve_to(&mut self, cp1: [f64; 2], cp2: [f64; 2], end: [f64; 2]) -> &mut Self {
        self.content
            .curve_to(cp1[0], cp1[1], cp2[0], cp2[1], end[0], end[1]);
        self
    }

    /// Closes the current path (for path building)
    pub fn close_path(&mut self) -> &mut Self {
        self.content.close_path();
        self
    }

    /// Strokes the current path (for path building)
    pub fn stroke_path(&mut self) -> &mut Self {
        self.content.stroke();
        self
    }

    /// Draws and strokes a polygon by connecting the given points
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::Document;
    ///
    /// let mut doc = Document::new();
    /// doc.stroke(|ctx| {
    ///     ctx.polygon(&[[100.0, 100.0], [150.0, 150.0], [100.0, 200.0]]);
    /// });
    /// ```
    pub fn polygon(&mut self, points: &[[f64; 2]]) -> &mut Self {
        if points.is_empty() {
            return self;
        }

        // Move to first point
        self.content.move_to(points[0][0], points[0][1]);

        // Draw lines to remaining points
        for point in &points[1..] {
            self.content.line_to(point[0], point[1]);
        }

        // Close and stroke the path
        self.content.close_path();
        self.content.stroke();
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

    /// Draws and fills a rectangle
    ///
    /// The origin is the bottom-left corner (PDF native coordinates).
    pub fn rectangle(&mut self, origin: [f64; 2], width: f64, height: f64) -> &mut Self {
        self.content.rect(origin[0], origin[1], width, height);
        self.content.fill();
        self
    }

    /// Draws and fills a rectangle with top-left origin (Prawn-style)
    ///
    /// This is an alias for `rectangle` that accepts the top-left corner
    /// as the origin point, matching Prawn's coordinate convention.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::Document;
    ///
    /// let mut doc = Document::new();
    /// // Draw a filled rectangle with top-left at (100, 500)
    /// doc.fill(|ctx| {
    ///     ctx.rect_tl([100.0, 500.0], 200.0, 100.0);
    /// });
    /// ```
    pub fn rect_tl(&mut self, top_left: [f64; 2], width: f64, height: f64) -> &mut Self {
        // Convert top-left to bottom-left (PDF native)
        let bottom_left = [top_left[0], top_left[1] - height];
        self.rectangle(bottom_left, width, height)
    }

    /// Draws and fills a rounded rectangle
    ///
    /// The origin is the bottom-left corner (PDF native coordinates).
    pub fn rounded_rectangle(
        &mut self,
        origin: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        self.content
            .rounded_rect(origin[0], origin[1], width, height, radius);
        self.content.fill();
        self
    }

    /// Draws and fills a rounded rectangle with top-left origin (Prawn-style)
    ///
    /// This is an alias for `rounded_rectangle` that accepts the top-left corner
    /// as the origin point, matching Prawn's coordinate convention.
    pub fn rounded_rect_tl(
        &mut self,
        top_left: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        // Convert top-left to bottom-left (PDF native)
        let bottom_left = [top_left[0], top_left[1] - height];
        self.rounded_rectangle(bottom_left, width, height, radius)
    }

    /// Draws and fills a circle
    pub fn circle(&mut self, center: [f64; 2], radius: f64) -> &mut Self {
        self.content.circle(center[0], center[1], radius);
        self.content.fill();
        self
    }

    /// Draws and fills an ellipse
    pub fn ellipse(&mut self, center: [f64; 2], rx: f64, ry: f64) -> &mut Self {
        self.content.ellipse(center[0], center[1], rx, ry);
        self.content.fill();
        self
    }

    /// Moves to a point (for path building)
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.content.move_to(x, y);
        self
    }

    /// Draws a line to a point (for path building)
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.content.line_to(x, y);
        self
    }

    /// Closes the current path (for path building)
    pub fn close_path(&mut self) -> &mut Self {
        self.content.close_path();
        self
    }

    /// Fills the current path (for path building)
    pub fn fill_path(&mut self) -> &mut Self {
        self.content.fill();
        self
    }

    /// Draws and fills a polygon by connecting the given points
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::Document;
    ///
    /// let mut doc = Document::new();
    /// doc.fill(|ctx| {
    ///     ctx.polygon(&[[100.0, 100.0], [150.0, 150.0], [100.0, 200.0]]);
    /// });
    /// ```
    pub fn polygon(&mut self, points: &[[f64; 2]]) -> &mut Self {
        if points.is_empty() {
            return self;
        }

        // Move to first point
        self.content.move_to(points[0][0], points[0][1]);

        // Draw lines to remaining points
        for point in &points[1..] {
            self.content.line_to(point[0], point[1]);
        }

        // Close and fill the path
        self.content.close_path();
        self.content.fill();
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
        doc.text_at("Hello World", [72.0, 700.0]);
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
            doc.text_at("Test", [72.0, 700.0]);
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

    #[test]
    fn test_embed_pdf_page() {
        use crate::document::LoadedDocument;

        // Create source PDF
        let mut source = Document::new();
        source.text_at("Source Page 1", [72.0, 700.0]);
        source.start_new_page();
        source.text_at("Source Page 2", [72.0, 700.0]);
        let source_bytes = source.render().unwrap();

        // Load and embed
        let mut loaded = LoadedDocument::load(source_bytes).unwrap();
        let mut doc = Document::new();
        let embedded = doc.embed_pdf_page(&mut loaded, 0).unwrap();

        assert!(embedded.width > 0.0);
        assert!(embedded.height > 0.0);
        assert!(embedded.name.starts_with("Pg"));
    }

    #[test]
    fn test_embed_pdf_all_pages() {
        use crate::document::LoadedDocument;

        // Create source PDF with 3 pages
        let mut source = Document::new();
        source.text_at("Page 1", [72.0, 700.0]);
        source.start_new_page();
        source.text_at("Page 2", [72.0, 700.0]);
        source.start_new_page();
        source.text_at("Page 3", [72.0, 700.0]);
        let source_bytes = source.render().unwrap();

        // Load and embed all
        let mut loaded = LoadedDocument::load(source_bytes).unwrap();
        let mut doc = Document::new();
        let embedded = doc.embed_pdf(&mut loaded).unwrap();

        assert_eq!(embedded.len(), 3);
    }

    #[test]
    fn test_draw_pdf_page() {
        use crate::document::LoadedDocument;

        // Create source PDF
        let mut source = Document::new();
        source.text_at("Source content", [72.0, 700.0]);
        let source_bytes = source.render().unwrap();

        // Load, embed, and draw
        let mut loaded = LoadedDocument::load(source_bytes).unwrap();
        let mut doc = Document::new();
        let embedded = doc.embed_pdf_page(&mut loaded, 0).unwrap();
        doc.draw_pdf_page(&embedded, [50.0, 400.0], 200.0, 300.0);

        // Should render without error
        let bytes = doc.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_pdf_date_format() {
        use std::time::{Duration, UNIX_EPOCH};

        // Test with a known date: 2024-01-15 12:30:45 UTC
        // Calculation: 2024-01-01 00:00:00 UTC = 1704067200
        // + 14 days (1209600) + 12 hours (43200) + 30 min (1800) + 45 sec = 1705321845
        let test_time = UNIX_EPOCH + Duration::from_secs(1705321845);
        let date_str = format_pdf_date(test_time).expect("valid date should format");

        // Complete string assertion - exact expected output
        assert_eq!(date_str, "D:20240115123045Z00'00'");

        // Verify format structure
        assert!(date_str.starts_with("D:"));
        assert!(date_str.ends_with("Z00'00'"));
        assert_eq!(date_str.len(), 23); // D: (2) + YYYYMMDDHHmmSS (14) + Z (1) + 00'00' (6)
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_pdf_date_format_pre_epoch() {
        use std::time::{Duration, UNIX_EPOCH};

        // Pre-1970 dates should return None
        let pre_epoch = UNIX_EPOCH - Duration::from_secs(1);
        assert!(
            format_pdf_date(pre_epoch).is_none(),
            "pre-1970 dates should return None"
        );

        // Much earlier date
        let way_before = UNIX_EPOCH - Duration::from_secs(365 * 24 * 60 * 60);
        assert!(
            format_pdf_date(way_before).is_none(),
            "dates before UNIX_EPOCH should return None"
        );
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_pdf_date_format_leap_year() {
        use std::time::{Duration, UNIX_EPOCH};

        // Test leap year: 2024-02-29 23:59:59 UTC
        // 2024 is a leap year, Feb 29 exists
        // This is 1709251199 seconds since epoch
        let leap_time = UNIX_EPOCH + Duration::from_secs(1709251199);
        let date_str = format_pdf_date(leap_time).unwrap();
        assert_eq!(date_str, "D:20240229235959Z00'00'");

        // Test non-leap year boundary: 2023-02-28 12:00:00 UTC
        // 2023 is NOT a leap year
        // This is 1677585600 seconds since epoch
        let non_leap_time = UNIX_EPOCH + Duration::from_secs(1677585600);
        let date_str = format_pdf_date(non_leap_time).unwrap();
        assert_eq!(date_str, "D:20230228120000Z00'00'");

        // Test century non-leap year: 1900 is NOT a leap year (divisible by 100 but not 400)
        // But we can't easily test 1900 as it's before UNIX_EPOCH (1970)
        // Test year 2000 which IS a leap year (divisible by 400)
        // 2000-02-29 00:00:00 UTC = 951782400 seconds since epoch
        let y2k_leap = UNIX_EPOCH + Duration::from_secs(951782400);
        let date_str = format_pdf_date(y2k_leap).unwrap();
        assert_eq!(date_str, "D:20000229000000Z00'00'");
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_pdf_date_format_edge_cases() {
        use std::time::{Duration, UNIX_EPOCH};

        // Test epoch start: 1970-01-01 00:00:00 UTC
        let epoch = UNIX_EPOCH;
        let date_str = format_pdf_date(epoch).unwrap();
        assert_eq!(date_str, "D:19700101000000Z00'00'");

        // Test end of year: 2023-12-31 23:59:59 UTC
        // This is 1704067199 seconds since epoch
        let year_end = UNIX_EPOCH + Duration::from_secs(1704067199);
        let date_str = format_pdf_date(year_end).unwrap();
        assert_eq!(date_str, "D:20231231235959Z00'00'");

        // Test start of year: 2024-01-01 00:00:00 UTC
        // This is 1704067200 seconds since epoch
        let year_start = UNIX_EPOCH + Duration::from_secs(1704067200);
        let date_str = format_pdf_date(year_start).unwrap();
        assert_eq!(date_str, "D:20240101000000Z00'00'");
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_document_info_with_dates() {
        let doc = Document::new();

        // Creation date should be None by default (for reproducible builds)
        assert!(doc.info.creation_date.is_none());

        // Mod date should be None by default
        assert!(doc.info.mod_date.is_none());
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_document_info_set_dates() {
        let mut doc = Document::new();

        // Initially None
        assert!(doc.info.creation_date.is_none());
        assert!(doc.info.mod_date.is_none());

        // Opt-in to timestamps
        doc.creation_date_now().mod_date_now();

        // Now should be set
        assert!(doc.info.creation_date.is_some());
        assert!(doc.info.mod_date.is_some());
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_document_info_dates_in_pdf_output() {
        use std::time::{Duration, UNIX_EPOCH};

        let mut doc = Document::new();
        doc.title("Test Doc");

        // Set specific dates (2024-06-15 10:30:00 UTC)
        let test_time = UNIX_EPOCH + Duration::from_secs(1718444400);
        doc.info.creation_date = Some(test_time);
        doc.info.mod_date = Some(test_time);

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Verify CreationDate is in the output
        assert!(
            pdf_str.contains("/CreationDate"),
            "PDF should contain /CreationDate field"
        );
        assert!(
            pdf_str.contains("D:20240615"),
            "PDF should contain formatted creation date"
        );

        // Verify ModDate is in the output
        assert!(
            pdf_str.contains("/ModDate"),
            "PDF should contain /ModDate field"
        );
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_document_info_no_dates_in_pdf_output() {
        // By default, dates should NOT be in PDF (reproducible builds)
        let mut doc = Document::new();
        doc.title("Test Doc");

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Without explicit dates, they should not appear
        assert!(
            !pdf_str.contains("/CreationDate"),
            "PDF should NOT contain /CreationDate when not set"
        );
        assert!(
            !pdf_str.contains("/ModDate"),
            "PDF should NOT contain /ModDate when not set"
        );
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_document_info_pre1970_dates_not_in_output() {
        use std::time::{Duration, UNIX_EPOCH};

        let mut doc = Document::new();
        // Set pre-1970 dates (should be silently skipped)
        let pre_1970 = UNIX_EPOCH - Duration::from_secs(1);
        doc.info.creation_date = Some(pre_1970);
        doc.info.mod_date = Some(pre_1970);
        // No other info fields set

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Pre-1970 dates should NOT appear in output
        assert!(
            !pdf_str.contains("/CreationDate"),
            "PDF should NOT contain /CreationDate for pre-1970 date"
        );
        assert!(
            !pdf_str.contains("/ModDate"),
            "PDF should NOT contain /ModDate for pre-1970 date"
        );
        // Info dict should not be created at all (no empty /Info)
        // The producer is set by default, so Info will still exist
        // But the dates should not be there
    }

    #[test]
    fn test_draw_pdf_page_fit() {
        use crate::document::LoadedDocument;

        // Create source PDF with A4 page
        let mut source = Document::new();
        source.text_at("A4 content", [72.0, 700.0]);
        let source_bytes = source.render().unwrap();

        // Load, embed, and draw with fit
        let mut loaded = LoadedDocument::load(source_bytes).unwrap();
        let mut doc = Document::new();
        let embedded = doc.embed_pdf_page(&mut loaded, 0).unwrap();

        // Draw scaled to fit within 200x200
        doc.draw_pdf_page_fit(&embedded, [50.0, 400.0], 200.0, 200.0);

        let bytes = doc.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_copy_pages() {
        use crate::document::LoadedDocument;

        // Create source PDF with 3 pages
        let mut source = Document::new();
        source.text_at("Page 1", [72.0, 700.0]);
        source.start_new_page();
        source.text_at("Page 2", [72.0, 700.0]);
        source.start_new_page();
        source.text_at("Page 3", [72.0, 700.0]);
        let source_bytes = source.render().unwrap();

        // Copy pages 0 and 2
        let mut loaded = LoadedDocument::load(source_bytes).unwrap();
        let mut doc = Document::new();
        doc.copy_pages(&mut loaded, &[0, 2]).unwrap();

        // Original page + 2 copied pages
        assert_eq!(doc.page_count(), 3);

        let bytes = doc.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_copy_all_pages() {
        use crate::document::LoadedDocument;

        // Create source PDF with 2 pages
        let mut source = Document::new();
        source.text_at("Source Page 1", [72.0, 700.0]);
        source.start_new_page();
        source.text_at("Source Page 2", [72.0, 700.0]);
        let source_bytes = source.render().unwrap();

        // Copy all pages
        let mut loaded = LoadedDocument::load(source_bytes).unwrap();
        let mut doc = Document::new();
        doc.copy_all_pages(&mut loaded).unwrap();

        // Original page + 2 copied pages
        assert_eq!(doc.page_count(), 3);

        let bytes = doc.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_merge_multiple_pdfs() {
        use crate::document::LoadedDocument;

        // Create first source PDF
        let mut source1 = Document::new();
        source1.text_at("Document 1 - Page 1", [72.0, 700.0]);
        let source1_bytes = source1.render().unwrap();

        // Create second source PDF
        let mut source2 = Document::new();
        source2.text_at("Document 2 - Page 1", [72.0, 700.0]);
        source2.start_new_page();
        source2.text_at("Document 2 - Page 2", [72.0, 700.0]);
        let source2_bytes = source2.render().unwrap();

        // Merge both PDFs
        let mut doc = Document::new();
        doc.text_at("Merged Document - Cover", [72.0, 700.0]);

        let mut loaded1 = LoadedDocument::load(source1_bytes).unwrap();
        doc.copy_all_pages(&mut loaded1).unwrap();

        let mut loaded2 = LoadedDocument::load(source2_bytes).unwrap();
        doc.copy_all_pages(&mut loaded2).unwrap();

        // 1 cover + 1 from source1 + 2 from source2 = 4 pages
        assert_eq!(doc.page_count(), 4);

        let bytes = doc.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_stroke_multiple_colors() {
        // Test that multiple shapes with different stroke colors each get their own color
        // Before the fix, all shapes would use the last color set
        // This test checks the ContentBuilder output directly since PDF streams are compressed

        use crate::content::ContentBuilder;

        let mut content = ContentBuilder::new();
        content.save_state();

        // Simulate what stroke() closure does with the fix:
        // Each shape should have its own stroke operation

        // Red rectangle
        content.set_stroke_color_rgb(1.0, 0.0, 0.0);
        content.rect(100.0, 100.0, 50.0, 50.0);
        content.stroke();

        // Green rectangle
        content.set_stroke_color_rgb(0.0, 1.0, 0.0);
        content.rect(200.0, 100.0, 50.0, 50.0);
        content.stroke();

        // Blue rectangle
        content.set_stroke_color_rgb(0.0, 0.0, 1.0);
        content.rect(300.0, 100.0, 50.0, 50.0);
        content.stroke();

        content.restore_state();

        let bytes = content.build();
        let output = String::from_utf8_lossy(&bytes);

        // Each rectangle should have its own stroke (S) operation
        let stroke_count = output.matches("\nS\n").count();
        assert_eq!(
            stroke_count, 3,
            "Expected 3 stroke operations, found {}. Output:\n{}",
            stroke_count, output
        );

        // Verify all three colors are present and in correct order
        assert!(
            output.contains("1 0 0 RG"),
            "Should contain red stroke color"
        );
        assert!(
            output.contains("0 1 0 RG"),
            "Should contain green stroke color"
        );
        assert!(
            output.contains("0 0 1 RG"),
            "Should contain blue stroke color"
        );

        // Verify the pattern: color RG -> re -> S (repeated)
        let red_pos = output.find("1 0 0 RG").unwrap();
        let green_pos = output.find("0 1 0 RG").unwrap();
        let blue_pos = output.find("0 0 1 RG").unwrap();

        // Colors should appear in order
        assert!(red_pos < green_pos, "Red should come before green");
        assert!(green_pos < blue_pos, "Green should come before blue");
    }

    #[test]
    fn test_fill_multiple_colors() {
        // Test that multiple shapes with different fill colors each get their own color
        // Before the fix, all shapes would use the last color set

        use crate::content::ContentBuilder;

        let mut content = ContentBuilder::new();
        content.save_state();

        // Red rectangle
        content.set_fill_color_rgb(1.0, 0.0, 0.0);
        content.rect(100.0, 100.0, 50.0, 50.0);
        content.fill();

        // Green rectangle
        content.set_fill_color_rgb(0.0, 1.0, 0.0);
        content.rect(200.0, 100.0, 50.0, 50.0);
        content.fill();

        // Blue rectangle
        content.set_fill_color_rgb(0.0, 0.0, 1.0);
        content.rect(300.0, 100.0, 50.0, 50.0);
        content.fill();

        content.restore_state();

        let bytes = content.build();
        let output = String::from_utf8_lossy(&bytes);

        // Each rectangle should have its own fill (f) operation
        let fill_count = output.matches("\nf\n").count();
        assert_eq!(
            fill_count, 3,
            "Expected 3 fill operations, found {}. Output:\n{}",
            fill_count, output
        );

        // Verify all three colors are present
        assert!(output.contains("1 0 0 rg"), "Should contain red fill color");
        assert!(
            output.contains("0 1 0 rg"),
            "Should contain green fill color"
        );
        assert!(
            output.contains("0 0 1 rg"),
            "Should contain blue fill color"
        );
    }

    #[test]
    fn test_stroke_context_immediate_stroke() {
        // Test that StrokeContext strokes each shape immediately
        // This is an integration test at the Document level

        let mut doc = Document::new();

        doc.stroke(|ctx| {
            ctx.color(1.0, 0.0, 0.0); // Red
            ctx.rectangle([100.0, 100.0], 50.0, 50.0);

            ctx.color(0.0, 1.0, 0.0); // Green
            ctx.circle([250.0, 125.0], 25.0);
        });

        // If this doesn't panic, the API is working
        let bytes = doc.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_fill_context_immediate_fill() {
        // Test that FillContext fills each shape immediately
        // This is an integration test at the Document level

        let mut doc = Document::new();

        doc.fill(|ctx| {
            ctx.color(1.0, 0.0, 0.0); // Red
            ctx.rectangle([100.0, 100.0], 50.0, 50.0);

            ctx.color(0.0, 1.0, 0.0); // Green
            ctx.circle([250.0, 125.0], 25.0);
        });

        // If this doesn't panic, the API is working
        let bytes = doc.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    // === Link Annotation Tests ===

    #[test]
    fn test_link_annotation_url() {
        use crate::api::link::LinkAnnotation;

        let mut doc = Document::new();
        doc.text_at("Click here", [72.0, 700.0]);

        // Add URL link annotation
        let link = LinkAnnotation::url([72.0, 690.0, 150.0, 710.0], "https://example.com");
        doc.link_annotation(link);

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Verify link annotation structure
        assert!(
            pdf_str.contains("/Subtype /Link"),
            "Should have Link subtype"
        );
        assert!(pdf_str.contains("/URI"), "Should have URI action");
        assert!(
            pdf_str.contains("https://example.com"),
            "Should contain the URL"
        );
    }

    #[test]
    fn test_link_annotation_internal_page() {
        use crate::api::link::LinkAnnotation;

        let mut doc = Document::new();
        doc.text_at("Page 1", [72.0, 700.0]);

        // Add link to page 2
        let link = LinkAnnotation::page([72.0, 690.0, 150.0, 710.0], 1);
        doc.link_annotation(link);

        doc.start_new_page();
        doc.text_at("Page 2", [72.0, 700.0]);

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Verify link annotation with page destination
        assert!(
            pdf_str.contains("/Subtype /Link"),
            "Should have Link subtype"
        );
        assert!(pdf_str.contains("/Dest"), "Should have Dest field");
        assert!(pdf_str.contains("/Fit"), "Should have Fit destination type");
    }

    #[test]
    fn test_link_url_convenience() {
        let mut doc = Document::new();
        doc.text_at("Click here", [72.0, 700.0]);
        doc.link_url([72.0, 690.0, 150.0, 710.0], "https://rust-lang.org");

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        assert!(
            pdf_str.contains("https://rust-lang.org"),
            "Should contain the URL"
        );
    }

    #[test]
    fn test_multiple_links_on_page() {
        use crate::api::link::LinkAnnotation;

        let mut doc = Document::new();
        doc.text_at("Link 1", [72.0, 700.0]);
        doc.link_url([72.0, 690.0, 120.0, 710.0], "https://example1.com");

        doc.text_at("Link 2", [72.0, 650.0]);
        doc.link_url([72.0, 640.0, 120.0, 660.0], "https://example2.com");

        doc.text_at("Link 3", [72.0, 600.0]);
        let link = LinkAnnotation::page([72.0, 590.0, 120.0, 610.0], 0);
        doc.link_annotation(link);

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Verify multiple annotations exist
        assert!(pdf_str.contains("example1.com"), "Should contain first URL");
        assert!(
            pdf_str.contains("example2.com"),
            "Should contain second URL"
        );
        assert!(pdf_str.contains("/Annots"), "Should have Annots array");
    }

    // === Named Destinations Tests ===

    #[test]
    fn test_add_dest() {
        use crate::api::link::DestinationFit;

        let mut doc = Document::new();
        doc.text_at("Chapter 1", [72.0, 700.0]);
        doc.add_dest("chapter1", 0, DestinationFit::Fit);

        assert_eq!(doc.destinations.len(), 1);
        assert!(doc.destinations.contains_key("chapter1"));
    }

    #[test]
    fn test_add_dest_here() {
        use crate::api::link::DestinationFit;

        let mut doc = Document::new();
        doc.text_at("Introduction", [72.0, 700.0]);
        doc.add_dest_here("intro", DestinationFit::FitH(Some(700.0)));

        assert_eq!(doc.destinations.len(), 1);
        assert!(doc.destinations.contains_key("intro"));
    }

    #[test]
    fn test_named_destinations_in_pdf() {
        use crate::api::link::DestinationFit;

        let mut doc = Document::new();
        doc.text_at("Page 1", [72.0, 700.0]);
        doc.add_dest("start", 0, DestinationFit::Fit);

        doc.start_new_page();
        doc.text_at("Page 2", [72.0, 700.0]);
        doc.add_dest("chapter1", 1, DestinationFit::FitH(Some(700.0)));

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Verify Names dictionary in Catalog
        assert!(pdf_str.contains("/Names"), "Should have Names dictionary");
        assert!(pdf_str.contains("/Dests"), "Should have Dests name tree");
        assert!(
            pdf_str.contains("(start)"),
            "Should contain 'start' destination name"
        );
        assert!(
            pdf_str.contains("(chapter1)"),
            "Should contain 'chapter1' destination name"
        );
    }

    #[test]
    fn test_named_destinations_sorted() {
        use crate::api::link::DestinationFit;

        let mut doc = Document::new();
        // Add destinations in reverse alphabetical order
        doc.add_dest("zebra", 0, DestinationFit::Fit);
        doc.add_dest("apple", 0, DestinationFit::Fit);
        doc.add_dest("mango", 0, DestinationFit::Fit);

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Names should be sorted alphabetically in the PDF
        let apple_pos = pdf_str.find("(apple)").expect("apple should exist");
        let mango_pos = pdf_str.find("(mango)").expect("mango should exist");
        let zebra_pos = pdf_str.find("(zebra)").expect("zebra should exist");

        assert!(apple_pos < mango_pos, "apple should come before mango");
        assert!(mango_pos < zebra_pos, "mango should come before zebra");
    }

    #[test]
    fn test_named_destination_link() {
        use crate::api::link::{DestinationFit, LinkAnnotation};

        let mut doc = Document::new();

        // Create link to named destination
        let link = LinkAnnotation::named([72.0, 690.0, 150.0, 710.0], "target");
        doc.link_annotation(link);

        // Add the named destination
        doc.add_dest("target", 0, DestinationFit::Fit);

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Link should reference the named destination
        assert!(
            pdf_str.contains("/Dest (target)"),
            "Link should reference named destination"
        );
        // Names dictionary should contain the destination
        assert!(pdf_str.contains("/Names"), "Should have Names dictionary");
    }

    #[test]
    fn test_destination_fit_types() {
        use crate::api::link::DestinationFit;

        let mut doc = Document::new();

        // Test different fit types
        doc.add_dest("fit", 0, DestinationFit::Fit);
        doc.add_dest("fith", 0, DestinationFit::FitH(Some(500.0)));
        doc.add_dest("fitv", 0, DestinationFit::FitV(Some(100.0)));
        doc.add_dest(
            "fitr",
            0,
            DestinationFit::FitR {
                left: 0.0,
                bottom: 0.0,
                right: 612.0,
                top: 792.0,
            },
        );
        doc.add_dest(
            "xyz",
            0,
            DestinationFit::XYZ {
                left: Some(72.0),
                top: Some(700.0),
                zoom: Some(1.5),
            },
        );

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Verify different fit types are in the PDF
        assert!(pdf_str.contains("/Fit]"), "Should have Fit destination");
        assert!(pdf_str.contains("/FitH"), "Should have FitH destination");
        assert!(pdf_str.contains("/FitV"), "Should have FitV destination");
        assert!(pdf_str.contains("/FitR"), "Should have FitR destination");
        assert!(pdf_str.contains("/XYZ"), "Should have XYZ destination");
    }

    #[test]
    fn test_no_destinations_no_names_dict() {
        let mut doc = Document::new();
        doc.text_at("No destinations", [72.0, 700.0]);

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Without destinations, Names dictionary should not exist
        // (unless other features add it)
        assert!(
            !pdf_str.contains("/Names <<"),
            "Should not have Names dictionary when no destinations"
        );
    }

    // === Outline (Bookmarks) Tests ===

    #[test]
    fn test_outline_builder_page() {
        use crate::api::outline::OutlineBuilder;

        let mut builder = OutlineBuilder::new();
        builder.page("Chapter 1", 0);
        builder.page("Chapter 2", 1);

        let items = builder.build();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Chapter 1");
        assert_eq!(items[1].title, "Chapter 2");
    }

    #[test]
    fn test_outline_builder_section() {
        use crate::api::outline::OutlineBuilder;

        let mut builder = OutlineBuilder::new();
        builder.section("Part 1", 0, |o| {
            o.page("Chapter 1", 0);
            o.page("Chapter 2", 1);
        });
        builder.section("Part 2", 2, |o| {
            o.page("Chapter 3", 2);
        });

        let items = builder.build();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].children.len(), 2);
        assert_eq!(items[1].children.len(), 1);
    }

    #[test]
    fn test_outline_builder_nested() {
        use crate::api::outline::OutlineBuilder;

        let mut builder = OutlineBuilder::new();
        builder.section("Part 1", 0, |o| {
            o.section("Chapter 1", 0, |o| {
                o.page("Section 1.1", 0);
                o.page("Section 1.2", 0);
            });
        });

        let items = builder.build();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].children.len(), 1);
        assert_eq!(items[0].children[0].children.len(), 2);
    }

    #[test]
    fn test_outline_builder_closed() {
        use crate::api::outline::OutlineBuilder;

        let mut builder = OutlineBuilder::new();
        builder.section_closed("Collapsed Section", 0, |o| {
            o.page("Hidden Item 1", 0);
            o.page("Hidden Item 2", 1);
        });

        let items = builder.build();
        assert_eq!(items.len(), 1);
        assert!(items[0].closed);
        assert_eq!(items[0].children.len(), 2);
    }

    #[test]
    fn test_document_outline_method() {
        let mut doc = Document::new();
        doc.start_new_page();
        doc.start_new_page();

        doc.outline(|o| {
            o.page("Page 1", 0);
            o.page("Page 2", 1);
            o.page("Page 3", 2);
        });

        assert!(doc.has_outline());
        assert_eq!(doc.outline.items.len(), 3);
    }

    #[test]
    fn test_outline_in_pdf_output() {
        let mut doc = Document::new();
        doc.text_at("Page 1", [72.0, 700.0]);
        doc.start_new_page();
        doc.text_at("Page 2", [72.0, 700.0]);

        doc.outline(|o| {
            o.page("Introduction", 0);
            o.page("Chapter 1", 1);
        });

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Verify outline structure in PDF
        assert!(
            pdf_str.contains("/Outlines"),
            "Should have Outlines in catalog"
        );
        assert!(
            pdf_str.contains("/Type /Outlines"),
            "Should have Outlines type"
        );
        assert!(
            pdf_str.contains("/Title (Introduction)"),
            "Should have first title"
        );
        assert!(
            pdf_str.contains("/Title (Chapter 1)"),
            "Should have second title"
        );
    }

    #[test]
    fn test_outline_with_sections() {
        let mut doc = Document::new();
        doc.start_new_page();
        doc.start_new_page();
        doc.start_new_page();

        doc.outline(|o| {
            o.section("Part 1", 0, |o| {
                o.page("Chapter 1", 0);
                o.page("Chapter 2", 1);
            });
            o.section("Part 2", 2, |o| {
                o.page("Chapter 3", 2);
            });
        });

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Verify hierarchical structure
        assert!(pdf_str.contains("/Title (Part 1)"), "Should have Part 1");
        assert!(pdf_str.contains("/Title (Part 2)"), "Should have Part 2");
        assert!(
            pdf_str.contains("/Title (Chapter 1)"),
            "Should have Chapter 1"
        );
        assert!(pdf_str.contains("/First"), "Parent should have First child");
        assert!(pdf_str.contains("/Last"), "Parent should have Last child");
        assert!(pdf_str.contains("/Count"), "Parent should have Count");
    }

    #[test]
    fn test_outline_count_calculation() {
        let mut doc = Document::new();
        doc.outline(|o| {
            o.section("Part 1", 0, |o| {
                o.page("Chapter 1", 0);
                o.page("Chapter 2", 0);
            });
            o.page("Appendix", 0);
        });

        // Total: Part 1 (1) + Chapter 1 (1) + Chapter 2 (1) + Appendix (1) = 4
        assert_eq!(doc.outline.total_count(), 4);
    }

    #[test]
    fn test_no_outline_no_outlines_dict() {
        let mut doc = Document::new();
        doc.text_at("No bookmarks", [72.0, 700.0]);

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Without outline, Outlines should not exist in catalog
        assert!(
            !pdf_str.contains("/Outlines"),
            "Should not have Outlines when no outline defined"
        );
    }

    #[test]
    fn test_outline_item_builder() {
        use crate::api::outline::{OutlineDestination, OutlineItem};

        let item = OutlineItem::new("Test")
            .with_destination(0)
            .with_closed(true)
            .with_child(OutlineItem::page("Child", 1));

        assert_eq!(item.title, "Test");
        assert!(item.closed);
        assert_eq!(item.children.len(), 1);
        assert!(matches!(
            item.destination,
            Some(OutlineDestination::Page { page_index: 0, .. })
        ));
    }

    #[test]
    fn test_outline_named_destination() {
        use crate::api::outline::OutlineItem;

        let mut doc = Document::new();

        // Add named destination
        doc.add_dest("chapter1", 0, link::DestinationFit::Fit);

        // Add outline linking to named destination
        doc.add_outline_item(OutlineItem::named("Go to Chapter 1", "chapter1"));

        let bytes = doc.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        // Should have outline with named destination
        assert!(pdf_str.contains("/Outlines"), "Should have Outlines");
        assert!(
            pdf_str.contains("/Title (Go to Chapter 1)"),
            "Should have title"
        );
        assert!(
            pdf_str.contains("/Dest (chapter1)"),
            "Should reference named destination"
        );
    }
}
