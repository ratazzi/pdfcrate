//! Layout system for Prawn-style document creation
//!
//! This module provides a wrapper around `Document` that adds cursor-based
//! layout functionality, allowing text and graphics to flow automatically
//! without manual coordinate calculations.
//!
//! # Example
//!
//! ```rust,no_run
//! use pdfcrate::api::{Document, LayoutDocument, Margin};
//!
//! let doc = Document::new();
//! let mut layout = LayoutDocument::new(doc);
//!
//! layout.font("Helvetica").size(24.0);
//! layout.text("Hello, World!");  // Draws at cursor, cursor moves down
//! layout.move_down(20.0);
//! layout.text("Next paragraph");
//! ```

use std::ops::{Deref, DerefMut};

use super::Document;

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    /// Align text to the left edge
    Left,
    /// Center text horizontally
    Center,
    /// Align text to the right edge
    Right,
    /// Justify text (stretch to fill width, except last line)
    Justify,
}

impl Default for TextAlign {
    fn default() -> Self {
        TextAlign::Left
    }
}

/// Specifies which pages a repeater should apply to
#[derive(Debug, Clone)]
pub enum RepeaterPages {
    /// Apply to all pages
    All,
    /// Apply to odd pages only (1, 3, 5, ...)
    Odd,
    /// Apply to even pages only (2, 4, 6, ...)
    Even,
    /// Apply to specific page numbers (1-indexed)
    Pages(Vec<usize>),
    /// Apply to a range of pages (1-indexed, inclusive)
    Range(usize, usize),
    /// Apply to all pages except the specified ones
    Except(Vec<usize>),
}

impl RepeaterPages {
    /// Returns true if this repeater should apply to the given page number (1-indexed)
    pub fn applies_to(&self, page: usize) -> bool {
        match self {
            RepeaterPages::All => true,
            RepeaterPages::Odd => page % 2 == 1,
            RepeaterPages::Even => page % 2 == 0,
            RepeaterPages::Pages(pages) => pages.contains(&page),
            RepeaterPages::Range(start, end) => page >= *start && page <= *end,
            RepeaterPages::Except(pages) => !pages.contains(&page),
        }
    }
}

/// Position for page numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageNumberPosition {
    /// Top left corner
    TopLeft,
    /// Top center
    TopCenter,
    /// Top right corner
    TopRight,
    /// Bottom left corner
    BottomLeft,
    /// Bottom center
    BottomCenter,
    /// Bottom right corner
    BottomRight,
}

/// Configuration for page numbering
#[derive(Debug, Clone)]
pub struct PageNumberConfig {
    /// Position of page numbers
    pub position: PageNumberPosition,
    /// Format string with placeholders: <page>, <total>
    /// e.g., "Page <page> of <total>" or just "<page>"
    pub format: String,
    /// Which pages to number
    pub pages: RepeaterPages,
    /// Starting page number (default 1)
    pub start_count_at: usize,
    /// Font name (default: current font)
    pub font: Option<String>,
    /// Font size (default: 10.0)
    pub font_size: f64,
    /// Color as RGB (default: black)
    pub color: Option<(f64, f64, f64)>,
}

impl Default for PageNumberConfig {
    fn default() -> Self {
        Self {
            position: PageNumberPosition::BottomCenter,
            format: "<page>".to_string(),
            pages: RepeaterPages::All,
            start_count_at: 1,
            font: None,
            font_size: 10.0,
            color: None,
        }
    }
}

impl PageNumberConfig {
    /// Creates a new config with the given format
    pub fn new(format: &str) -> Self {
        Self {
            format: format.to_string(),
            ..Default::default()
        }
    }

    /// Sets the position
    pub fn position(mut self, position: PageNumberPosition) -> Self {
        self.position = position;
        self
    }

    /// Sets which pages to number
    pub fn pages(mut self, pages: RepeaterPages) -> Self {
        self.pages = pages;
        self
    }

    /// Sets the starting count
    pub fn start_at(mut self, start: usize) -> Self {
        self.start_count_at = start;
        self
    }

    /// Sets the font
    pub fn font(mut self, font: &str) -> Self {
        self.font = Some(font.to_string());
        self
    }

    /// Sets the font size
    pub fn font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    /// Sets the color
    pub fn color(mut self, r: f64, g: f64, b: f64) -> Self {
        self.color = Some((r, g, b));
        self
    }
}

/// Stored content for a repeater (header/footer)
#[derive(Clone)]
struct RepeaterContent {
    /// Which pages to apply to
    pages: RepeaterPages,
    /// Text content
    text: String,
    /// Position [x, y]
    position: [f64; 2],
    /// Font name
    font: String,
    /// Font size
    font_size: f64,
    /// Text alignment (reserved for future use)
    #[allow(dead_code)]
    align: TextAlign,
}

/// Page margins in points
///
/// Default is 36pt (0.5 inch) on all sides, matching Prawn's default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margin {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Margin {
    /// Creates margins with the same value on all sides
    pub fn all(value: f64) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Creates margins with vertical (top/bottom) and horizontal (left/right) values
    pub fn symmetric(vertical: f64, horizontal: f64) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Creates margins with individual values (top, right, bottom, left)
    pub fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Creates zero margins
    pub fn zero() -> Self {
        Self::all(0.0)
    }
}

impl Default for Margin {
    /// Default margin is 36pt (0.5 inch) on all sides
    fn default() -> Self {
        Self::all(36.0)
    }
}

/// A bounding box defining a rectangular region for layout
///
/// Coordinates use PDF's coordinate system where (0,0) is at the bottom-left
/// of the page and Y increases upward.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundingBox {
    /// Absolute x coordinate of the left edge
    x: f64,
    /// Absolute y coordinate of the top edge
    y: f64,
    /// Width of the bounding box
    width: f64,
    /// Fixed height, or None for stretchy box
    height: Option<f64>,
    /// Actual height used (for stretchy boxes)
    stretched_height: f64,
    /// Left padding (for indent)
    left_padding: f64,
    /// Right padding (for indent)
    right_padding: f64,
}

impl BoundingBox {
    /// Creates a new bounding box
    pub fn new(x: f64, y: f64, width: f64, height: Option<f64>) -> Self {
        Self {
            x,
            y,
            width,
            height,
            stretched_height: 0.0,
            left_padding: 0.0,
            right_padding: 0.0,
        }
    }

    // === Relative coordinates (relative to this box's origin) ===

    /// Left edge in relative coordinates (always 0)
    pub fn left(&self) -> f64 {
        0.0
    }

    /// Right edge in relative coordinates
    pub fn right(&self) -> f64 {
        self.width
    }

    /// Top edge in relative coordinates
    pub fn top(&self) -> f64 {
        self.height()
    }

    /// Bottom edge in relative coordinates (always 0)
    pub fn bottom(&self) -> f64 {
        0.0
    }

    /// Top-left corner in relative coordinates
    pub fn top_left(&self) -> [f64; 2] {
        [self.left(), self.top()]
    }

    /// Top-right corner in relative coordinates
    pub fn top_right(&self) -> [f64; 2] {
        [self.right(), self.top()]
    }

    /// Bottom-left corner in relative coordinates
    pub fn bottom_left(&self) -> [f64; 2] {
        [self.left(), self.bottom()]
    }

    /// Bottom-right corner in relative coordinates
    pub fn bottom_right(&self) -> [f64; 2] {
        [self.right(), self.bottom()]
    }

    // === Absolute coordinates (page coordinates) ===

    /// Absolute left x coordinate
    pub fn absolute_left(&self) -> f64 {
        self.x + self.left_padding
    }

    /// Absolute right x coordinate
    pub fn absolute_right(&self) -> f64 {
        self.x + self.width - self.right_padding
    }

    /// Absolute top y coordinate
    pub fn absolute_top(&self) -> f64 {
        self.y
    }

    /// Absolute bottom y coordinate
    pub fn absolute_bottom(&self) -> f64 {
        self.y - self.height()
    }

    /// Absolute top-left corner
    pub fn absolute_top_left(&self) -> [f64; 2] {
        [self.absolute_left(), self.absolute_top()]
    }

    /// Absolute top-right corner
    pub fn absolute_top_right(&self) -> [f64; 2] {
        [self.absolute_right(), self.absolute_top()]
    }

    /// Absolute bottom-left corner
    pub fn absolute_bottom_left(&self) -> [f64; 2] {
        [self.absolute_left(), self.absolute_bottom()]
    }

    /// Absolute bottom-right corner
    pub fn absolute_bottom_right(&self) -> [f64; 2] {
        [self.absolute_right(), self.absolute_bottom()]
    }

    // === Properties ===

    /// Width of the bounding box
    pub fn width(&self) -> f64 {
        self.width - self.left_padding - self.right_padding
    }

    /// Height of the bounding box
    ///
    /// For fixed-height boxes, returns the fixed height.
    /// For stretchy boxes, returns the actual height used so far.
    pub fn height(&self) -> f64 {
        self.height.unwrap_or(self.stretched_height)
    }

    /// Returns true if this is a stretchy (auto-height) box
    pub fn stretchy(&self) -> bool {
        self.height.is_none()
    }

    /// Updates the stretched height based on cursor position
    pub(crate) fn update_stretched_height(&mut self, cursor_y: f64) {
        if self.stretchy() {
            let used = self.y - cursor_y;
            self.stretched_height = self.stretched_height.max(used);
        }
    }

    /// Adds left padding (for indent)
    pub(crate) fn add_left_padding(&mut self, padding: f64) {
        self.left_padding += padding;
    }

    /// Adds right padding (for indent)
    pub(crate) fn add_right_padding(&mut self, padding: f64) {
        self.right_padding += padding;
    }

    /// Removes left padding
    pub(crate) fn subtract_left_padding(&mut self, padding: f64) {
        self.left_padding -= padding;
    }

    /// Removes right padding
    pub(crate) fn subtract_right_padding(&mut self, padding: f64) {
        self.right_padding -= padding;
    }
}

/// Internal layout state
struct LayoutState {
    /// Current y position (cursor)
    cursor_y: f64,
    /// Stack of bounding boxes (innermost last)
    bounds_stack: Vec<BoundingBox>,
    /// Page margins (reserved for future auto-pagination)
    #[allow(dead_code)]
    margin: Margin,
    /// Current text alignment
    text_align: TextAlign,
    /// Current leading (line spacing multiplier)
    leading: f64,
}

impl LayoutState {
    fn new(margin: Margin, page_width: f64, page_height: f64) -> Self {
        // Create the margin box as the default bounding box
        let margin_box = BoundingBox::new(
            margin.left,
            page_height - margin.top,
            page_width - margin.left - margin.right,
            Some(page_height - margin.top - margin.bottom),
        );

        let cursor_y = margin_box.absolute_top();

        Self {
            cursor_y,
            bounds_stack: vec![margin_box],
            margin,
            text_align: TextAlign::Left,
            leading: 1.2,
        }
    }

    fn bounds(&self) -> &BoundingBox {
        self.bounds_stack
            .last()
            .expect("bounds stack should never be empty")
    }

    fn bounds_mut(&mut self) -> &mut BoundingBox {
        self.bounds_stack
            .last_mut()
            .expect("bounds stack should never be empty")
    }
}

/// A document with layout capabilities
///
/// This wraps a `Document` and adds cursor-based layout, allowing
/// content to flow automatically without manual coordinate calculations.
///
/// # Example
///
/// ```rust,no_run
/// use pdfcrate::api::{Document, LayoutDocument};
///
/// // Create from existing document
/// let doc = Document::new();
/// let mut layout = LayoutDocument::new(doc);
///
/// // Use layout methods
/// layout.text("Hello!");
/// layout.move_down(20.0);
/// layout.text("World!");
///
/// // Document methods still work via Deref
/// layout.title("My Document");
/// layout.save("output.pdf").unwrap();
/// ```
pub struct LayoutDocument {
    inner: Document,
    state: LayoutState,
    /// Stored repeaters (headers/footers)
    repeaters: Vec<RepeaterContent>,
    /// Page number configuration
    page_number_config: Option<PageNumberConfig>,
}

impl LayoutDocument {
    /// Creates a new LayoutDocument wrapping the given Document
    ///
    /// Uses default margins (36pt / 0.5 inch on all sides).
    pub fn new(doc: Document) -> Self {
        Self::with_margin(doc, Margin::default())
    }

    /// Creates a new LayoutDocument with custom margins
    pub fn with_margin(doc: Document, margin: Margin) -> Self {
        let (width, height) = {
            let dims = doc.page_size.dimensions(doc.page_layout);
            (dims.0, dims.1)
        };

        Self {
            state: LayoutState::new(margin, width, height),
            inner: doc,
            repeaters: Vec::new(),
            page_number_config: None,
        }
    }

    /// Unwraps the LayoutDocument and returns the inner Document
    pub fn into_inner(self) -> Document {
        self.inner
    }

    /// Returns a reference to the inner Document
    pub fn inner(&self) -> &Document {
        &self.inner
    }

    /// Returns a mutable reference to the inner Document
    pub fn inner_mut(&mut self) -> &mut Document {
        &mut self.inner
    }

    // === Cursor methods ===

    /// Returns the current cursor y position (absolute page coordinates)
    pub fn cursor(&self) -> f64 {
        self.state.cursor_y
    }

    /// Moves the cursor down by the specified amount
    pub fn move_down(&mut self, amount: f64) -> &mut Self {
        self.state.cursor_y -= amount;
        self
    }

    /// Moves the cursor up by the specified amount
    pub fn move_up(&mut self, amount: f64) -> &mut Self {
        self.state.cursor_y += amount;
        self
    }

    /// Moves the cursor to the specified y position (relative to bounds top)
    pub fn move_cursor_to(&mut self, y: f64) -> &mut Self {
        let top = self.state.bounds().absolute_top();
        self.state.cursor_y = top - y;
        self
    }

    // === Bounds methods ===

    /// Returns the current bounding box
    pub fn bounds(&self) -> &BoundingBox {
        self.state.bounds()
    }

    /// Draws the current bounding box outline (for debugging)
    pub fn stroke_bounds(&mut self) -> &mut Self {
        let bounds = self.state.bounds();
        let x = bounds.absolute_left();
        let y = bounds.absolute_bottom();
        let w = bounds.width();
        let h = bounds.height();

        self.inner.stroke(|ctx| {
            ctx.rectangle([x, y], w, h);
        });
        self
    }

    // === Text methods ===

    /// Sets the text alignment for subsequent text operations
    pub fn align(&mut self, align: TextAlign) -> &mut Self {
        self.state.text_align = align;
        self
    }

    /// Sets the leading (line spacing multiplier) for subsequent text operations
    ///
    /// Default is 1.2 (20% spacing between lines).
    /// Leading is multiplied by the font size to determine the line height.
    pub fn leading(&mut self, leading: f64) -> &mut Self {
        self.state.leading = leading;
        self
    }

    /// Returns the current line height based on font size and leading
    pub fn line_height(&self) -> f64 {
        self.inner.current_font_size * self.state.leading
    }

    /// Draws text at the current cursor position
    ///
    /// The cursor is moved down by the line height after drawing.
    /// Text is not wrapped - use `text_wrap` for automatic wrapping.
    ///
    /// Note: Like Prawn, the cursor represents the TOP of the text line.
    /// The text baseline is positioned below the cursor by the font ascender.
    pub fn text(&mut self, text: &str) -> &mut Self {
        let bounds = self.state.bounds();
        let left = bounds.absolute_left();
        let right = bounds.absolute_right();
        let width = right - left;

        // In Prawn, cursor is at the TOP of the text line.
        // Text baseline should be below cursor by approximately (font_size - descender).
        // For most fonts, ascender is about 80% of em height, descender about 20%.
        // So baseline = cursor - ascender ≈ cursor - (font_size * 0.8)
        // But Prawn uses a simpler model: baseline = cursor - (font_size - descender_height)
        // Approximating descender as 20% of font_size: baseline = cursor - font_size * 0.8
        let font_size = self.inner.current_font_size;
        let ascender_offset = font_size * 0.78; // Approximate ascender ratio
        let y = self.state.cursor_y - ascender_offset;

        // Calculate x position based on alignment
        let x = match self.state.text_align {
            TextAlign::Left => left,
            TextAlign::Center => {
                let text_width = self.measure_text_width(text);
                left + (width - text_width) / 2.0
            }
            TextAlign::Right => {
                let text_width = self.measure_text_width(text);
                right - text_width
            }
            TextAlign::Justify => left, // Justify not supported for single-line text
        };

        // Draw text at calculated position (y is the baseline)
        self.inner.text_at(text, [x, y]);

        // Move cursor down by line height
        let line_height = self.line_height();
        self.state.cursor_y -= line_height;

        // Update stretched height if in stretchy box
        let cursor_y = self.state.cursor_y;
        self.state.bounds_mut().update_stretched_height(cursor_y);

        self
    }

    /// Draws wrapped text at the current cursor position
    ///
    /// Text is automatically wrapped to fit within the current bounds width.
    /// Each line respects the current alignment setting.
    pub fn text_wrap(&mut self, text: &str) -> &mut Self {
        let bounds = self.state.bounds();
        let width = bounds.width();

        let lines = self.wrap_text_to_width(text, width);

        for line in lines {
            self.text(&line);
        }

        self
    }

    /// Draws text in a bounding box with automatic wrapping
    ///
    /// This is similar to Prawn's `text_box` method. Text is wrapped to fit
    /// within the specified dimensions, and overflow is silently clipped.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to render
    /// * `point` - Position offset from current cursor [x, y]
    /// * `width` - Width of the text box
    /// * `height` - Fixed height of the text box
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument};
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    /// layout.text_box("Long text that will wrap...", [0.0, 0.0], 200.0, 100.0);
    /// ```
    pub fn text_box(&mut self, text: &str, point: [f64; 2], width: f64, height: f64) -> &mut Self {
        self.bounding_box(point, width, Some(height), |doc| {
            let bounds_height = doc.bounds().height();
            let line_height = doc.line_height();
            let max_lines = (bounds_height / line_height).floor() as usize;

            let lines = doc.wrap_text_to_width(text, width);

            for (i, line) in lines.iter().enumerate() {
                if i >= max_lines {
                    break; // Stop if we exceed the box height
                }
                doc.text(line);
            }
        });

        self
    }

    /// Measures the width of text with the current font
    fn measure_text_width(&self, text: &str) -> f64 {
        #[cfg(feature = "fonts")]
        {
            self.inner.measure_text(text)
        }
        #[cfg(not(feature = "fonts"))]
        {
            // Approximate width for standard fonts
            text.len() as f64 * self.inner.current_font_size * 0.5
        }
    }

    /// Wraps text to fit within the specified width
    fn wrap_text_to_width(&self, text: &str, max_width: f64) -> Vec<String> {
        let mut lines = Vec::new();

        for paragraph in text.split('\n') {
            if paragraph.is_empty() {
                lines.push(String::new());
                continue;
            }

            let words: Vec<&str> = paragraph.split_whitespace().collect();
            if words.is_empty() {
                lines.push(String::new());
                continue;
            }

            let mut current_line = String::new();

            for word in words {
                let test_line = if current_line.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current_line, word)
                };

                let width = self.measure_text_width(&test_line);

                if width <= max_width {
                    current_line = test_line;
                } else {
                    // Line is too long
                    if !current_line.is_empty() {
                        lines.push(current_line.clone());
                        current_line = word.to_string();
                    } else {
                        // Single word is too long, just add it anyway
                        lines.push(word.to_string());
                    }
                }
            }

            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    // === Repeater methods (headers/footers/page numbers) ===

    /// Adds a repeating text element (header or footer)
    ///
    /// The text will be rendered on the specified pages at the given position.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, RepeaterPages};
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    ///
    /// // Add header to all pages
    /// layout.repeat(RepeaterPages::All, "My Document", [36.0, 800.0]);
    ///
    /// // Add footer to odd pages only
    /// layout.repeat(RepeaterPages::Odd, "Confidential", [36.0, 30.0]);
    /// ```
    pub fn repeat(&mut self, pages: RepeaterPages, text: &str, position: [f64; 2]) -> &mut Self {
        self.repeaters.push(RepeaterContent {
            pages,
            text: text.to_string(),
            position,
            font: self.inner.current_font.clone(),
            font_size: self.inner.current_font_size,
            align: self.state.text_align,
        });
        self
    }

    /// Adds a header that repeats on specified pages
    ///
    /// The header is positioned at the top of the page within the margin area.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, RepeaterPages};
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    /// layout.header(RepeaterPages::All, "Chapter 1: Introduction");
    /// ```
    pub fn header(&mut self, pages: RepeaterPages, text: &str) -> &mut Self {
        let margin = self.state.margin;
        let (_, page_height) = self.inner.page_size.dimensions(self.inner.page_layout);
        let y = page_height - margin.top / 2.0;
        self.repeat(pages, text, [margin.left, y])
    }

    /// Adds a footer that repeats on specified pages
    ///
    /// The footer is positioned at the bottom of the page within the margin area.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, RepeaterPages};
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    /// layout.footer(RepeaterPages::All, "© 2024 My Company");
    /// ```
    pub fn footer(&mut self, pages: RepeaterPages, text: &str) -> &mut Self {
        let margin = self.state.margin;
        let y = margin.bottom / 2.0;
        self.repeat(pages, text, [margin.left, y])
    }

    /// Configures automatic page numbering
    ///
    /// Page numbers are rendered when `apply_repeaters()` is called or
    /// when the document is rendered.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, PageNumberConfig, PageNumberPosition};
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    ///
    /// // Simple page numbers at bottom center
    /// layout.number_pages(PageNumberConfig::default());
    ///
    /// // Custom format "Page X of Y" at bottom right
    /// layout.number_pages(
    ///     PageNumberConfig::new("Page <page> of <total>")
    ///         .position(PageNumberPosition::BottomRight)
    /// );
    /// ```
    pub fn number_pages(&mut self, config: PageNumberConfig) -> &mut Self {
        self.page_number_config = Some(config);
        self
    }

    /// Applies all repeaters and page numbers to the document
    ///
    /// This should be called after all content has been added to the document,
    /// as it needs to know the total page count for page numbering.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, PageNumberConfig};
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    /// layout.number_pages(PageNumberConfig::new("<page> / <total>"));
    ///
    /// // Add content...
    /// layout.text("Page 1 content");
    /// layout.start_new_page();
    /// layout.text("Page 2 content");
    ///
    /// // Apply page numbers after all content is added
    /// layout.apply_repeaters();
    /// ```
    pub fn apply_repeaters(&mut self) -> &mut Self {
        let total_pages = self.inner.page_count();
        let (page_width, page_height) = self.inner.page_size.dimensions(self.inner.page_layout);
        let margin = self.state.margin;

        // Clone repeaters to avoid borrow issues
        let repeaters = self.repeaters.clone();
        let page_number_config = self.page_number_config.clone();

        // Apply repeaters to each page
        for page_idx in 0..total_pages {
            let page_num = page_idx + 1; // 1-indexed

            // Apply text repeaters
            for repeater in &repeaters {
                if repeater.pages.applies_to(page_num) {
                    self.inner.go_to_page(page_idx);
                    self.inner.font(&repeater.font).size(repeater.font_size);
                    self.inner.text_at(&repeater.text, repeater.position);
                }
            }

            // Apply page numbers
            if let Some(ref config) = page_number_config {
                if config.pages.applies_to(page_num) {
                    self.inner.go_to_page(page_idx);

                    // Set font
                    let font = config.font.as_deref().unwrap_or("Helvetica");
                    self.inner.font(font).size(config.font_size);

                    // Format the page number string
                    let display_page = page_num - 1 + config.start_count_at;
                    let text = config
                        .format
                        .replace("<page>", &display_page.to_string())
                        .replace("<total>", &total_pages.to_string());

                    // Calculate position based on config
                    let (x, y) = match config.position {
                        PageNumberPosition::TopLeft => {
                            (margin.left, page_height - margin.top / 2.0)
                        }
                        PageNumberPosition::TopCenter => {
                            let text_width = self.measure_text_width(&text);
                            (
                                (page_width - text_width) / 2.0,
                                page_height - margin.top / 2.0,
                            )
                        }
                        PageNumberPosition::TopRight => {
                            let text_width = self.measure_text_width(&text);
                            (
                                page_width - margin.right - text_width,
                                page_height - margin.top / 2.0,
                            )
                        }
                        PageNumberPosition::BottomLeft => (margin.left, margin.bottom / 2.0),
                        PageNumberPosition::BottomCenter => {
                            let text_width = self.measure_text_width(&text);
                            ((page_width - text_width) / 2.0, margin.bottom / 2.0)
                        }
                        PageNumberPosition::BottomRight => {
                            let text_width = self.measure_text_width(&text);
                            (page_width - margin.right - text_width, margin.bottom / 2.0)
                        }
                    };

                    self.inner.text_at(&text, [x, y]);
                }
            }
        }

        // Return to last page
        if total_pages > 0 {
            self.inner.go_to_page(total_pages - 1);
        }

        self
    }

    // === Layout methods ===

    /// Executes a block without affecting the cursor position
    ///
    /// After the block executes, the cursor returns to its original position.
    pub fn float<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        let saved_cursor = self.state.cursor_y;
        f(self);
        self.state.cursor_y = saved_cursor;
        self
    }

    /// Temporarily indents content from left and/or right
    pub fn indent<F>(&mut self, left: f64, right: f64, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        self.state.bounds_mut().add_left_padding(left);
        self.state.bounds_mut().add_right_padding(right);
        f(self);
        self.state.bounds_mut().subtract_left_padding(left);
        self.state.bounds_mut().subtract_right_padding(right);
        self
    }

    /// Adds vertical padding before and after the block
    pub fn pad<F>(&mut self, amount: f64, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        self.move_down(amount);
        f(self);
        self.move_down(amount);
        self
    }

    /// Adds vertical padding before the block
    pub fn pad_top<F>(&mut self, amount: f64, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        self.move_down(amount);
        f(self);
        self
    }

    /// Adds vertical padding after the block
    pub fn pad_bottom<F>(&mut self, amount: f64, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.move_down(amount);
        self
    }

    // === Bounding Box methods ===

    /// Creates a nested bounding box for layout
    ///
    /// Content inside the bounding box uses coordinates relative to the box.
    /// After the closure executes, the cursor moves below the bounding box.
    ///
    /// # Arguments
    ///
    /// * `point` - Position of the box's top-left corner:
    ///   - `point[0]`: x offset from current bounds left edge
    ///   - `point[1]`: y offset downward from current cursor position
    ///     (e.g., `[0.0, 0.0]` = at cursor, `[50.0, 20.0]` = 50pt right, 20pt below cursor)
    /// * `width` - Width of the bounding box
    /// * `height` - Fixed height, or `None` for stretchy box that grows with content
    /// * `f` - Closure to execute within the bounding box
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument};
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    ///
    /// layout.text("Before box");
    ///
    /// // Box at current cursor position
    /// layout.bounding_box([0.0, 0.0], 200.0, Some(100.0), |doc| {
    ///     doc.text("Inside the box");
    ///     doc.stroke_bounds();
    /// });
    ///
    /// // Stretchy box (height determined by content)
    /// layout.bounding_box([0.0, 0.0], 200.0, None, |doc| {
    ///     doc.text("Line 1");
    ///     doc.text("Line 2");
    /// });
    /// ```
    pub fn bounding_box<F>(
        &mut self,
        point: [f64; 2],
        width: f64,
        height: Option<f64>,
        f: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        let parent = self.state.bounds();

        // Calculate absolute coordinates
        // point[0] is x offset from parent's left edge
        // point[1] is y offset downward from cursor
        let abs_x = parent.absolute_left() + point[0];
        let abs_y = self.state.cursor_y - point[1]; // Always relative to cursor

        let bbox = BoundingBox::new(abs_x, abs_y, width, height);

        // Push to stack
        self.state.bounds_stack.push(bbox);

        // Save cursor, set new cursor to bbox top
        let old_cursor = self.state.cursor_y;
        self.state.cursor_y = abs_y;

        // Execute closure
        f(self);

        // Update stretched height before popping
        let cursor_y = self.state.cursor_y;
        if let Some(bbox) = self.state.bounds_stack.last_mut() {
            bbox.update_stretched_height(cursor_y);
        }

        // Pop and get the finished bbox
        let finished_bbox = self.state.bounds_stack.pop().unwrap();

        // Update parent cursor position
        if height.is_some() {
            // Fixed height: cursor moves to below the fixed-height box
            self.state.cursor_y = abs_y - height.unwrap();
        } else {
            // Stretchy: cursor is at the bottom of content
            self.state.cursor_y = abs_y - finished_bbox.height();
        }

        // If cursor moved below old cursor, keep the new position
        // Otherwise, restore to maintain flow from before the box
        if self.state.cursor_y > old_cursor {
            self.state.cursor_y = old_cursor;
        }

        self
    }

    /// Sets the cursor to a specific absolute y position
    pub fn set_cursor(&mut self, y: f64) -> &mut Self {
        self.state.cursor_y = y;
        self
    }

    /// Starts a new page and resets the cursor to the top of the margin box
    ///
    /// This method should be used instead of calling `start_new_page()` directly
    /// on the inner Document, as it properly resets the layout cursor position.
    pub fn start_new_page(&mut self) -> &mut Self {
        self.inner.start_new_page();

        // Reset cursor to top of margin box for the new page
        let (_, page_height) = self.inner.page_size.dimensions(self.inner.page_layout);
        self.state.cursor_y = page_height - self.state.margin.top;

        self
    }
}

// Allow LayoutDocument to be used like Document
impl Deref for LayoutDocument {
    type Target = Document;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for LayoutDocument {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_margin_default() {
        let margin = Margin::default();
        assert_eq!(margin.top, 36.0);
        assert_eq!(margin.right, 36.0);
        assert_eq!(margin.bottom, 36.0);
        assert_eq!(margin.left, 36.0);
    }

    #[test]
    fn test_margin_all() {
        let margin = Margin::all(72.0);
        assert_eq!(margin.top, 72.0);
        assert_eq!(margin.right, 72.0);
        assert_eq!(margin.bottom, 72.0);
        assert_eq!(margin.left, 72.0);
    }

    #[test]
    fn test_margin_symmetric() {
        let margin = Margin::symmetric(50.0, 100.0);
        assert_eq!(margin.top, 50.0);
        assert_eq!(margin.bottom, 50.0);
        assert_eq!(margin.left, 100.0);
        assert_eq!(margin.right, 100.0);
    }

    #[test]
    fn test_bounding_box_fixed() {
        let bbox = BoundingBox::new(72.0, 720.0, 468.0, Some(648.0));

        assert_eq!(bbox.absolute_left(), 72.0);
        assert_eq!(bbox.absolute_right(), 540.0); // 72 + 468
        assert_eq!(bbox.absolute_top(), 720.0);
        assert_eq!(bbox.absolute_bottom(), 72.0); // 720 - 648

        assert_eq!(bbox.left(), 0.0);
        assert_eq!(bbox.right(), 468.0);
        assert_eq!(bbox.top(), 648.0);
        assert_eq!(bbox.bottom(), 0.0);

        assert!(!bbox.stretchy());
    }

    #[test]
    fn test_bounding_box_stretchy() {
        let mut bbox = BoundingBox::new(72.0, 720.0, 468.0, None);
        assert!(bbox.stretchy());
        assert_eq!(bbox.height(), 0.0);

        // Simulate cursor moving down
        bbox.update_stretched_height(620.0); // cursor moved from 720 to 620
        assert_eq!(bbox.height(), 100.0);

        bbox.update_stretched_height(520.0); // cursor moved further
        assert_eq!(bbox.height(), 200.0);
    }

    #[test]
    fn test_layout_document_cursor() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Cursor should start at top of margin box
        // A4 size: 595.28 x 841.89, margin 36pt
        // Top of margin box = 841.89 - 36 = 805.89
        let initial_cursor = layout.cursor();
        assert!((initial_cursor - 805.89).abs() < 0.01);

        // Move down
        layout.move_down(100.0);
        assert!((layout.cursor() - 705.89).abs() < 0.01);

        // Move up
        layout.move_up(50.0);
        assert!((layout.cursor() - 755.89).abs() < 0.01);

        // Move to specific position (relative to bounds top)
        layout.move_cursor_to(200.0);
        assert!((layout.cursor() - 605.89).abs() < 0.01); // 805.89 - 200
    }

    #[test]
    fn test_layout_document_bounds() {
        let doc = Document::new();
        let layout = LayoutDocument::new(doc);

        let bounds = layout.bounds();

        // A4 size: 595.28 x 841.89, margin 36pt
        assert_eq!(bounds.absolute_left(), 36.0);
        assert!((bounds.absolute_right() - 559.28).abs() < 0.01); // 595.28 - 36
        assert!((bounds.absolute_top() - 805.89).abs() < 0.01); // 841.89 - 36
        assert_eq!(bounds.absolute_bottom(), 36.0);

        assert!((bounds.width() - 523.28).abs() < 0.01); // 595.28 - 36 - 36
        assert!((bounds.height() - 769.89).abs() < 0.01); // 841.89 - 36 - 36
    }

    #[test]
    fn test_layout_document_text() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.font("Helvetica").size(12.0);
        let cursor_before = layout.cursor();

        layout.text("Hello, World!");

        // Cursor should have moved down by line height
        let expected_line_height = 12.0 * 1.2;
        assert_eq!(layout.cursor(), cursor_before - expected_line_height);
    }

    #[test]
    fn test_layout_document_float() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        let cursor_before = layout.cursor();

        layout.float(|doc| {
            doc.move_down(200.0);
            // Cursor is now 200pt lower
        });

        // After float, cursor should be restored
        assert_eq!(layout.cursor(), cursor_before);
    }

    #[test]
    fn test_layout_document_indent() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        let original_left = layout.bounds().absolute_left();
        let original_right = layout.bounds().absolute_right();

        layout.indent(50.0, 30.0, |doc| {
            assert_eq!(doc.bounds().absolute_left(), original_left + 50.0);
            assert_eq!(doc.bounds().absolute_right(), original_right - 30.0);
        });

        // After indent, bounds should be restored
        assert_eq!(layout.bounds().absolute_left(), original_left);
        assert_eq!(layout.bounds().absolute_right(), original_right);
    }

    #[test]
    fn test_layout_document_pad() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        let cursor_before = layout.cursor();

        layout.pad(20.0, |doc| {
            // Inside pad, cursor has moved down by 20
            assert_eq!(doc.cursor(), cursor_before - 20.0);
        });

        // After pad, cursor should have moved down by 40 total
        assert_eq!(layout.cursor(), cursor_before - 40.0);
    }

    #[test]
    fn test_layout_document_deref() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Should be able to use Document methods via Deref
        layout.title("Test Document");
        layout.author("Test Author");

        // text_at should work
        layout.text_at("Absolute position", [100.0, 500.0]);
    }

    #[test]
    fn test_layout_document_render() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.font("Helvetica").size(24.0);
        layout.text("Hello, World!");
        layout.move_down(20.0);
        layout.text("This is a test.");

        let bytes = layout.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_layout_document_custom_margin() {
        let doc = Document::new();
        let layout = LayoutDocument::with_margin(doc, Margin::all(72.0));

        let bounds = layout.bounds();

        // A4 size: 595.28 x 841.89, margin 72pt
        assert_eq!(bounds.absolute_left(), 72.0);
        assert!((bounds.absolute_right() - 523.28).abs() < 0.01); // 595.28 - 72
        assert!((bounds.absolute_top() - 769.89).abs() < 0.01); // 841.89 - 72
        assert_eq!(bounds.absolute_bottom(), 72.0);
    }

    #[test]
    fn test_bounding_box_fixed_height() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        let initial_cursor = layout.cursor();

        // Create a fixed-height bounding box at cursor position
        layout.bounding_box([50.0, 0.0], 200.0, Some(150.0), |doc| {
            // Inside the box, bounds should reflect the new box
            let inner_bounds = doc.bounds();
            assert_eq!(inner_bounds.absolute_left(), 36.0 + 50.0); // margin + offset
            assert!((inner_bounds.width() - 200.0).abs() < 0.01);
            assert!((inner_bounds.height() - 150.0).abs() < 0.01);

            doc.text("Inside box");
        });

        // After the box, cursor should be below the box
        // Box top was at cursor (initial_cursor - 0 offset)
        // Box bottom is at: initial_cursor - 150 (fixed height)
        let expected_cursor = initial_cursor - 150.0;
        assert!((layout.cursor() - expected_cursor).abs() < 0.01);
    }

    #[test]
    fn test_bounding_box_method_stretchy() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.font("Helvetica").size(12.0);
        let initial_cursor = layout.cursor();

        // Create a stretchy bounding box at cursor position
        layout.bounding_box([0.0, 0.0], 200.0, None, |doc| {
            doc.text("Line 1");
            doc.text("Line 2");
            doc.text("Line 3");
        });

        // The box should have stretched to fit 3 lines
        // Each line is 12.0 * 1.2 = 14.4pt
        // Total height should be about 43.2pt
        let expected_height = 12.0 * 1.2 * 3.0;
        assert!((initial_cursor - layout.cursor() - expected_height).abs() < 0.1);
    }

    #[test]
    fn test_bounding_box_method_nested() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        let outer_left = layout.bounds().absolute_left();

        // Outer box at cursor position
        layout.bounding_box([20.0, 0.0], 300.0, Some(200.0), |doc| {
            let inner_bounds = doc.bounds();
            assert_eq!(inner_bounds.absolute_left(), outer_left + 20.0);

            // Nested bounding box (offset from inner cursor)
            doc.bounding_box([30.0, 0.0], 150.0, Some(100.0), |doc| {
                let nested_bounds = doc.bounds();
                assert_eq!(nested_bounds.absolute_left(), outer_left + 20.0 + 30.0);
                assert!((nested_bounds.width() - 150.0).abs() < 0.01);
                assert!((nested_bounds.height() - 100.0).abs() < 0.01);

                doc.text("Deeply nested");
            });
        });
    }

    #[test]
    fn test_bounding_box_with_stroke() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.bounding_box([50.0, 50.0], 200.0, Some(100.0), |doc| {
            doc.stroke_bounds();
            doc.text("Boxed content");
        });

        // Should render without errors
        let bytes = layout.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_repeater_pages_all() {
        let pages = RepeaterPages::All;
        assert!(pages.applies_to(1));
        assert!(pages.applies_to(2));
        assert!(pages.applies_to(100));
    }

    #[test]
    fn test_repeater_pages_odd_even() {
        let odd = RepeaterPages::Odd;
        let even = RepeaterPages::Even;

        assert!(odd.applies_to(1));
        assert!(!odd.applies_to(2));
        assert!(odd.applies_to(3));

        assert!(!even.applies_to(1));
        assert!(even.applies_to(2));
        assert!(!even.applies_to(3));
    }

    #[test]
    fn test_repeater_pages_specific() {
        let pages = RepeaterPages::Pages(vec![1, 3, 5]);
        assert!(pages.applies_to(1));
        assert!(!pages.applies_to(2));
        assert!(pages.applies_to(3));
        assert!(!pages.applies_to(4));
        assert!(pages.applies_to(5));
    }

    #[test]
    fn test_repeater_pages_range() {
        let pages = RepeaterPages::Range(2, 5);
        assert!(!pages.applies_to(1));
        assert!(pages.applies_to(2));
        assert!(pages.applies_to(3));
        assert!(pages.applies_to(5));
        assert!(!pages.applies_to(6));
    }

    #[test]
    fn test_repeater_pages_except() {
        let pages = RepeaterPages::Except(vec![1, 10]);
        assert!(!pages.applies_to(1));
        assert!(pages.applies_to(2));
        assert!(pages.applies_to(5));
        assert!(!pages.applies_to(10));
    }

    #[test]
    fn test_page_number_config() {
        let config = PageNumberConfig::new("Page <page> of <total>")
            .position(PageNumberPosition::BottomRight)
            .font_size(12.0)
            .start_at(1);

        assert_eq!(config.format, "Page <page> of <total>");
        assert_eq!(config.position, PageNumberPosition::BottomRight);
        assert_eq!(config.font_size, 12.0);
        assert_eq!(config.start_count_at, 1);
    }

    #[test]
    fn test_number_pages_render() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.number_pages(PageNumberConfig::new("<page>"));
        layout.text("Page 1 content");
        layout.start_new_page();
        layout.text("Page 2 content");

        layout.apply_repeaters();

        let bytes = layout.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_header_footer() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.header(RepeaterPages::All, "Document Header");
        layout.footer(RepeaterPages::All, "Document Footer");

        layout.text("Page content");

        layout.apply_repeaters();

        let bytes = layout.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_start_new_page_resets_cursor() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        let initial_cursor = layout.cursor();

        // Move cursor down
        layout.move_down(200.0);
        assert_eq!(layout.cursor(), initial_cursor - 200.0);

        // Start new page - cursor should reset to top of margin box
        layout.start_new_page();

        // Cursor should be at the same position as the initial cursor
        // (top of margin box for the new page)
        assert!((layout.cursor() - initial_cursor).abs() < 0.01);
    }
}
