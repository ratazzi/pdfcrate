//! High-level PDF API
//!
//! This module provides the user-facing API for creating and manipulating PDFs.

pub mod image;
pub mod layout;
pub mod page;

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
use crate::objects::{PdfArray, PdfDict, PdfObject, PdfRef, PdfStream};

pub use image::{EmbeddedImage, ImageOptions, Position};
pub use layout::{BoundingBox, LayoutDocument, Margin};
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
}

/// Internal page data
struct PageData {
    content: ContentBuilder,
    size: PageSize,
    layout: PageLayout,
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
                producer: Some("pdf_rs".to_string()),
                ..Default::default()
            },
            images: Vec::new(),
            image_counter: 0,
            form: AcroForm::new(),
            ext_gstates: Vec::new(),
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

        self.pages[self.current_page]
            .content
            .begin_text()
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

                self.pages[self.current_page]
                    .content
                    .begin_text()
                    .set_font(&font_name, font_size)
                    .move_text_pos(pos[0], pos[1])
                    .show_text_hex(&hex)
                    .end_text();

                return self;
            }
        }

        self.pages[self.current_page]
            .content
            .begin_text()
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
        // Fallback for standard fonts (approximate)
        text.len() as f64 * self.current_font_size * 0.5
    }

    #[cfg(feature = "fonts")]
    fn draw_embedded_text(&mut self, text: &str, pos: [f64; 2]) {
        let font_name = self.current_font.clone();
        let font_size = self.current_font_size;
        let font = match self.embedded_fonts.get(&font_name) {
            Some(font) => font.clone(),
            None => {
                self.pages[self.current_page]
                    .content
                    .begin_text()
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
        content.begin_text().set_font(&font_name, font_size);

        if has_offsets {
            Self::show_positioned_glyphs(content, &glyphs, pos, font_size);
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
        let scale = font_size / 1000.0;
        let mut pen_x: i32 = 0;
        let mut pen_y: i32 = 0;

        for glyph in glyphs {
            let x = pos[0] + (pen_x + glyph.x_offset) as f64 * scale;
            let y = pos[1] + (pen_y + glyph.y_offset) as f64 * scale;
            content
                .set_text_matrix(1.0, 0.0, 0.0, 1.0, x, y)
                .show_text_hex(&format!("{:04X}", glyph.gid));

            pen_x += glyph.x_advance;
            pen_y += glyph.y_advance;
        }
    }

    /// Ensures a font is registered
    fn ensure_font(&mut self, name: &str) {
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
    /// use pdf_rs::prelude::*;
    ///
    /// let mut doc = Document::new();
    /// let svg = r#"
    ///     <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
    ///         <rect x="10" y="10" width="80" height="80" fill="red"/>
    ///     </svg>
    /// "#;
    /// doc.draw_svg(svg, [100.0, 700.0], 200.0, 200.0)?;
    /// # Ok::<(), pdf_rs::Error>(())
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
    /// use pdf_rs::api::Document;
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

            // Add annotations to each page that has fields
            for (page_idx, annot_refs) in page_annots {
                if let Some(&page_ref) = page_refs.get(page_idx) {
                    // Create Annots array for this page
                    let annots: Vec<PdfObject> = annot_refs
                        .iter()
                        .map(|r| PdfObject::Reference(*r))
                        .collect();
                    let annots_array = PdfArray::from(annots);

                    // Update the page with annotations
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

    /// Draws a polygon by connecting the given points
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdf_rs::api::Document;
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

        // Close the path
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

    /// Draws a polygon by connecting the given points
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdf_rs::api::Document;
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

        // Close the path
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
}
