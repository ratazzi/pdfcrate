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

pub use super::color::{Color, ColorInput};
use super::measurements::Measurement;
use super::{Document, FillAndStrokeContext, FillContext, StrokeContext};

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// Align text to the left edge
    #[default]
    Left,
    /// Center text horizontally
    Center,
    /// Align text to the right edge
    Right,
    /// Justify text (stretch to fill width, except last line)
    Justify,
}

/// Text overflow behavior for text boxes
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Overflow {
    /// Truncate text that exceeds the box height (default behavior)
    ///
    /// Text that doesn't fit is silently discarded.
    #[default]
    Truncate,

    /// Shrink the font size to fit all text within the box
    ///
    /// The font will be reduced until all text fits, down to the specified
    /// minimum size. If text still doesn't fit at min size, it will be truncated.
    ///
    /// The parameter is the minimum font size (default: 6.0 if using `ShrinkToFit::default()`).
    ShrinkToFit(f64),

    /// Expand the box height to accommodate all text
    ///
    /// The box will grow vertically as needed. The actual height used
    /// is returned in `TextBoxResult::height`.
    Expand,
}

impl Overflow {
    /// Create a ShrinkToFit overflow with default minimum size (6.0pt)
    pub fn shrink_to_fit() -> Self {
        Overflow::ShrinkToFit(6.0)
    }

    /// Create a ShrinkToFit overflow with custom minimum size
    pub fn shrink_to_fit_min(min_size: f64) -> Self {
        Overflow::ShrinkToFit(min_size)
    }
}

/// Result information from text_box rendering
#[derive(Debug, Clone)]
pub struct TextBoxResult {
    /// Actual height used by the text box
    ///
    /// For `Overflow::Expand`, this may be larger than the requested height.
    /// For other modes, this equals the requested height.
    pub height: f64,

    /// Whether text was truncated due to overflow
    pub truncated: bool,

    /// Actual font size used for rendering
    ///
    /// For `Overflow::ShrinkToFit`, this may be smaller than the original font size.
    pub font_size: f64,

    /// Number of lines actually rendered
    pub lines_rendered: usize,

    /// Total number of lines in the wrapped text
    ///
    /// If `truncated` is true, `total_lines > lines_rendered`.
    pub total_lines: usize,
}

/// Font style for text rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStyle {
    /// Normal (regular) font style
    #[default]
    Normal,
    /// Bold font style
    Bold,
    /// Italic font style
    Italic,
    /// Bold and italic font style
    BoldItalic,
}

/// A fragment of formatted text with optional styling
///
/// Used with `formatted_text` to render text with mixed styles.
#[derive(Debug, Clone)]
pub struct TextFragment {
    /// The text content
    pub text: String,
    /// Font style (normal, bold, italic, bold-italic)
    pub style: FontStyle,
    /// Text color (None = use current color)
    pub color: Option<Color>,
    /// Font size (None = use current font size)
    pub size: Option<f64>,
    /// Font name (None = use current font family)
    pub font: Option<String>,
    /// Whether to draw an underline
    pub underline: bool,
    /// Whether to draw a strikethrough line
    pub strikethrough: bool,
    /// Whether to render as superscript (smaller, raised)
    pub superscript: bool,
    /// Whether to render as subscript (smaller, lowered)
    pub subscript: bool,
    /// Hyperlink URL (None = no link)
    pub link: Option<String>,
}

impl TextFragment {
    /// Creates a new text fragment with the given text
    pub fn new(text: impl Into<String>) -> Self {
        TextFragment {
            text: text.into(),
            style: FontStyle::Normal,
            color: None,
            size: None,
            font: None,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            link: None,
        }
    }

    /// Sets the font style
    pub fn style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the text color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the font size
    pub fn size(mut self, size: f64) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets the font name
    pub fn font(mut self, font: impl Into<String>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets bold style (convenience method)
    pub fn bold(self) -> Self {
        self.style(FontStyle::Bold)
    }

    /// Sets italic style (convenience method)
    pub fn italic(self) -> Self {
        self.style(FontStyle::Italic)
    }

    /// Sets bold-italic style (convenience method)
    pub fn bold_italic(self) -> Self {
        self.style(FontStyle::BoldItalic)
    }

    /// Enables underline decoration
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Enables strikethrough decoration
    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    /// Enables superscript rendering (smaller, raised)
    pub fn superscript(mut self) -> Self {
        self.superscript = true;
        self
    }

    /// Enables subscript rendering (smaller, lowered)
    pub fn subscript(mut self) -> Self {
        self.subscript = true;
        self
    }

    /// Sets a hyperlink URL
    pub fn link(mut self, url: impl Into<String>) -> Self {
        self.link = Some(url.into());
        self
    }
}

/// A run of text that should be rendered with a specific font
///
/// Used internally for font fallback analysis.
#[derive(Debug, Clone)]
struct FontRun {
    /// The text content
    text: String,
    /// The font name to use
    font: String,
}

/// Options for text rendering
///
/// Used with `text_opts` to provide per-call options like fallback fonts.
///
/// # Example
///
/// ```rust,no_run
/// use pdfcrate::api::{Document, LayoutDocument, TextOptions};
///
/// let mut doc = LayoutDocument::new(Document::new());
/// let lxgw = doc.embed_font_file("fonts/LXGWWenKai-Regular.ttf").unwrap();
///
/// // Use fallback fonts for this specific text call
/// doc.text_opts("Hello 你好", TextOptions::new().fallback_fonts(vec![lxgw]));
/// ```
#[derive(Debug, Clone, Default)]
pub struct TextOptions {
    /// Fallback fonts for this text (overrides global fallback fonts)
    pub fallback_fonts: Option<Vec<String>>,
}

impl TextOptions {
    /// Creates new empty text options
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the fallback fonts for this text
    ///
    /// These fonts override any globally configured fallback fonts.
    pub fn fallback_fonts(mut self, fonts: Vec<String>) -> Self {
        self.fallback_fonts = Some(fonts);
        self
    }

    /// Adds a single fallback font
    pub fn fallback_font(mut self, font: impl Into<String>) -> Self {
        let fonts = self.fallback_fonts.get_or_insert_with(Vec::new);
        fonts.push(font.into());
        self
    }
}

/// Options for defining a grid layout system
///
/// A grid divides the page into rows and columns with optional gutters
/// (spacing between cells). This allows for precise, responsive layouts.
///
/// # Example
///
/// ```rust
/// use pdfcrate::api::GridOptions;
///
/// // Create a 12-column, 20-row grid with 10pt gutters
/// let options = GridOptions::new(20, 12).gutter(10.0);
///
/// // Or with separate row and column gutters
/// let options = GridOptions::new(20, 12)
///     .row_gutter(15.0)
///     .column_gutter(10.0);
/// ```
#[derive(Debug, Clone)]
pub struct GridOptions {
    /// Number of rows in the grid
    pub rows: usize,
    /// Number of columns in the grid
    pub columns: usize,
    /// Row gutter (vertical spacing between rows)
    pub row_gutter: f64,
    /// Column gutter (horizontal spacing between columns)
    pub column_gutter: f64,
}

impl GridOptions {
    /// Creates a new grid with the specified number of rows and columns
    ///
    /// Gutters default to 0.0.
    pub fn new(rows: usize, columns: usize) -> Self {
        GridOptions {
            rows,
            columns,
            row_gutter: 0.0,
            column_gutter: 0.0,
        }
    }

    /// Sets both row and column gutters to the same value
    pub fn gutter(mut self, gutter: f64) -> Self {
        self.row_gutter = gutter;
        self.column_gutter = gutter;
        self
    }

    /// Sets the row gutter (vertical spacing between rows)
    pub fn row_gutter(mut self, gutter: f64) -> Self {
        self.row_gutter = gutter;
        self
    }

    /// Sets the column gutter (horizontal spacing between columns)
    pub fn column_gutter(mut self, gutter: f64) -> Self {
        self.column_gutter = gutter;
        self
    }
}

/// A grid system for page layout
///
/// The grid calculates cell sizes based on the current bounds and the
/// specified number of rows, columns, and gutters.
#[derive(Debug, Clone)]
pub struct Grid {
    /// Number of rows
    pub rows: usize,
    /// Number of columns
    pub columns: usize,
    /// Row gutter size
    pub row_gutter: f64,
    /// Column gutter size
    pub column_gutter: f64,
    /// Calculated width of each column
    pub column_width: f64,
    /// Calculated height of each row
    pub row_height: f64,
    /// Total width of the grid area
    pub total_width: f64,
    /// Total height of the grid area
    pub total_height: f64,
}

impl Grid {
    /// Creates a new grid with the given options and bounds dimensions
    pub fn new(options: &GridOptions, width: f64, height: f64) -> Self {
        let column_width = Self::subdivide(width, options.columns, options.column_gutter);
        let row_height = Self::subdivide(height, options.rows, options.row_gutter);

        Grid {
            rows: options.rows,
            columns: options.columns,
            row_gutter: options.row_gutter,
            column_gutter: options.column_gutter,
            column_width,
            row_height,
            total_width: width,
            total_height: height,
        }
    }

    /// Calculates the size of each subdivision given total size, count, and gutter
    fn subdivide(total: f64, count: usize, gutter: f64) -> f64 {
        (total - (gutter * (count - 1) as f64)) / count as f64
    }

    /// Returns a GridBox for the specified row and column (0-indexed)
    pub fn cell(&self, row: usize, column: usize) -> GridBox {
        GridBox::new(self, row, column)
    }

    /// Returns a MultiBox spanning from one cell to another
    ///
    /// The span includes both the start and end cells and everything in between.
    pub fn span(&self, start: (usize, usize), end: (usize, usize)) -> MultiBox {
        let box1 = self.cell(start.0, start.1);
        let box2 = self.cell(end.0, end.1);
        MultiBox::new(box1, box2)
    }
}

/// A single cell in the grid
///
/// Provides coordinates and dimensions for positioning content.
#[derive(Debug, Clone)]
pub struct GridBox {
    /// Row index (0-indexed)
    pub row: usize,
    /// Column index (0-indexed)
    pub column: usize,
    /// Width of this cell
    pub width: f64,
    /// Height of this cell
    pub height: f64,
    /// Left x-coordinate (relative to grid origin)
    pub left: f64,
    /// Top y-coordinate (relative to grid origin, in PDF coordinates)
    pub top: f64,
}

impl GridBox {
    /// Creates a new GridBox from grid parameters
    fn new(grid: &Grid, row: usize, column: usize) -> Self {
        let width = grid.column_width;
        let height = grid.row_height;
        let left = (width + grid.column_gutter) * column as f64;
        let top = grid.total_height - ((height + grid.row_gutter) * row as f64);

        GridBox {
            row,
            column,
            width,
            height,
            left,
            top,
        }
    }

    /// Returns the right x-coordinate
    pub fn right(&self) -> f64 {
        self.left + self.width
    }

    /// Returns the bottom y-coordinate
    pub fn bottom(&self) -> f64 {
        self.top - self.height
    }

    /// Returns the top-left corner coordinates [x, y]
    pub fn top_left(&self) -> [f64; 2] {
        [self.left, self.top]
    }

    /// Returns the top-right corner coordinates [x, y]
    pub fn top_right(&self) -> [f64; 2] {
        [self.right(), self.top]
    }

    /// Returns the bottom-left corner coordinates [x, y]
    pub fn bottom_left(&self) -> [f64; 2] {
        [self.left, self.bottom()]
    }

    /// Returns the bottom-right corner coordinates [x, y]
    pub fn bottom_right(&self) -> [f64; 2] {
        [self.right(), self.bottom()]
    }

    /// Returns the name of this cell as "row,column"
    pub fn name(&self) -> String {
        format!("{},{}", self.row, self.column)
    }
}

/// A span of multiple grid cells
///
/// Represents a rectangular region spanning from one cell to another.
#[derive(Debug, Clone)]
pub struct MultiBox {
    /// Width of the span
    pub width: f64,
    /// Height of the span
    pub height: f64,
    /// Left x-coordinate
    pub left: f64,
    /// Top y-coordinate
    pub top: f64,
    /// Name showing the span range
    name: String,
}

impl MultiBox {
    /// Creates a new MultiBox spanning between two grid boxes
    fn new(box1: GridBox, box2: GridBox) -> Self {
        let left = box1.left.min(box2.left);
        let right = box1.right().max(box2.right());
        let top = box1.top.max(box2.top);
        let bottom = box1.bottom().min(box2.bottom());

        MultiBox {
            width: right - left,
            height: top - bottom,
            left,
            top,
            name: format!("{}:{}", box1.name(), box2.name()),
        }
    }

    /// Returns the right x-coordinate
    pub fn right(&self) -> f64 {
        self.left + self.width
    }

    /// Returns the bottom y-coordinate
    pub fn bottom(&self) -> f64 {
        self.top - self.height
    }

    /// Returns the top-left corner coordinates [x, y]
    pub fn top_left(&self) -> [f64; 2] {
        [self.left, self.top]
    }

    /// Returns the top-right corner coordinates [x, y]
    pub fn top_right(&self) -> [f64; 2] {
        [self.right(), self.top]
    }

    /// Returns the bottom-left corner coordinates [x, y]
    pub fn bottom_left(&self) -> [f64; 2] {
        [self.left, self.bottom()]
    }

    /// Returns the bottom-right corner coordinates [x, y]
    pub fn bottom_right(&self) -> [f64; 2] {
        [self.right(), self.bottom()]
    }

    /// Returns the name of this span
    pub fn name(&self) -> &str {
        &self.name
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
            RepeaterPages::Even => page.is_multiple_of(2),
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
    /// Fallback fonts for text rendering
    fallback_fonts: Vec<String>,
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
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    pub fn all(value: impl Measurement) -> Self {
        let v = value.to_pt();
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    /// Creates margins with vertical (top/bottom) and horizontal (left/right) values
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    pub fn symmetric(vertical: impl Measurement, horizontal: impl Measurement) -> Self {
        let v = vertical.to_pt();
        let h = horizontal.to_pt();
        Self {
            top: v,
            right: h,
            bottom: v,
            left: h,
        }
    }

    /// Creates margins with individual values (top, right, bottom, left)
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    pub fn new(
        top: impl Measurement,
        right: impl Measurement,
        bottom: impl Measurement,
        left: impl Measurement,
    ) -> Self {
        Self {
            top: top.to_pt(),
            right: right.to_pt(),
            bottom: bottom.to_pt(),
            left: left.to_pt(),
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

/// Options for multi-column layout
#[derive(Debug, Clone)]
pub struct ColumnBoxOptions {
    /// Number of columns (default: 3)
    pub columns: usize,
    /// Space between columns in points (default: current font_size)
    pub spacer: Option<f64>,
    /// Fixed height of the column area in points.
    /// When None, columns extend to the bottom margin.
    pub height: Option<f64>,
    /// Whether to reflow margins when entering column mode
    pub reflow_margins: bool,
}

impl Default for ColumnBoxOptions {
    fn default() -> Self {
        Self {
            columns: 3,
            spacer: None,
            height: None,
            reflow_margins: false,
        }
    }
}

impl ColumnBoxOptions {
    /// Create column options with the given number of columns
    pub fn new(columns: usize) -> Self {
        Self {
            columns,
            ..Default::default()
        }
    }

    /// Set the spacer (gap between columns) in points
    pub fn spacer(mut self, spacer: f64) -> Self {
        self.spacer = Some(spacer);
        self
    }

    /// Set the fixed height of the column area in points
    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    /// Set whether to reflow margins
    pub fn reflow_margins(mut self, reflow: bool) -> Self {
        self.reflow_margins = reflow;
        self
    }
}

/// Internal state for multi-column layout
#[derive(Debug, Clone)]
struct ColumnState {
    /// Number of columns
    columns: usize,
    /// Space between columns in points
    spacer: f64,
    /// Current column index (0-based)
    current_column: usize,
    /// Width of each column
    column_width: f64,
    /// X origin (left edge of the first column)
    origin_x: f64,
    /// Y origin (top of the column area)
    origin_y: f64,
    /// Bottom boundary for column overflow detection
    bottom_y: f64,
    /// Lowest cursor y reached across all columns (for end-of-box cursor placement)
    max_depth_y: f64,
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
    /// Current character spacing (in points)
    character_spacing: f64,
    /// Current word spacing (in points)
    word_spacing: f64,
    /// Fallback fonts list (in priority order)
    fallback_fonts: Vec<String>,
    /// Active column layout state (None when not in column mode)
    column_state: Option<ColumnState>,
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
            leading: 1.0, // Default 1.0 matches Prawn's default (no extra leading)
            character_spacing: 0.0,
            word_spacing: 0.0,
            fallback_fonts: Vec::new(),
            column_state: None,
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
    /// Current grid system (if defined)
    grid: Option<Grid>,
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
            grid: None,
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

    /// Returns the current cursor y position relative to the bottom of the bounding box.
    ///
    /// Follows PDF/Prawn coordinate convention where Y increases upward.
    /// Initial value equals bounds height (cursor starts at top).
    /// Value decreases as cursor moves down toward bottom.
    pub fn cursor(&self) -> f64 {
        self.state.cursor_y - self.state.bounds().absolute_bottom()
    }

    /// Moves the cursor down by the specified amount
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    pub fn move_down(&mut self, amount: impl Measurement) -> &mut Self {
        self.state.cursor_y -= amount.to_pt();
        self
    }

    /// Moves the cursor up by the specified amount
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    pub fn move_up(&mut self, amount: impl Measurement) -> &mut Self {
        self.state.cursor_y += amount.to_pt();
        self
    }

    /// Moves the cursor to the specified y position relative to the bottom of the bounding box.
    ///
    /// Follows PDF/Prawn coordinate convention where Y increases upward.
    /// `move_cursor_to(bounds.height)` moves to top, `move_cursor_to(0)` moves to bottom.
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    pub fn move_cursor_to(&mut self, y: impl Measurement) -> &mut Self {
        let bottom = self.state.bounds().absolute_bottom();
        self.state.cursor_y = bottom + y.to_pt();
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
        let y = bounds.absolute_top(); // top-left origin for rectangle
        let w = bounds.width();
        let h = bounds.height();

        self.inner.stroke(|ctx| {
            ctx.rectangle([x, y], w, h);
        });
        self
    }

    /// Draws the current bounding box outline with a specific color
    pub fn stroke_bounds_color(&mut self, color: Color) -> &mut Self {
        let bounds = self.state.bounds();
        let x = bounds.absolute_left();
        let y = bounds.absolute_top(); // top-left origin for rectangle
        let w = bounds.width();
        let h = bounds.height();

        self.inner.stroke(|ctx| {
            ctx.color(color).rectangle([x, y], w, h);
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
    /// Default is 1.0 (no extra spacing, matching Prawn's default).
    /// Leading is multiplied by the font height (ascender - descender) to determine line height.
    pub fn leading(&mut self, leading: f64) -> &mut Self {
        self.state.leading = leading;
        self
    }

    /// Sets the character spacing (space between characters)
    ///
    /// The value is in points. Default is 0.
    /// Positive values increase spacing, negative values decrease.
    ///
    /// # Example
    /// ```ignore
    /// doc.character_spacing(2.0)
    ///    .text("S P A C E D");
    /// ```
    pub fn character_spacing(&mut self, spacing: impl Measurement) -> &mut Self {
        self.state.character_spacing = spacing.to_pt();
        self
    }

    /// Sets the word spacing (extra space added to space characters)
    ///
    /// Default is 0. This only affects the space character (U+0020).
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    ///
    /// # Example
    /// ```ignore
    /// doc.word_spacing(5.0)
    ///    .text("Words with extra spacing");
    /// ```
    pub fn word_spacing(&mut self, spacing: impl Measurement) -> &mut Self {
        self.state.word_spacing = spacing.to_pt();
        self
    }

    // === Font Fallback Methods ===

    /// Sets the fallback font list for automatic font substitution
    ///
    /// When rendering text, if the primary font doesn't have a glyph for a character,
    /// the fallback fonts are tried in order. This is useful for mixed-script text
    /// (e.g., Latin + CJK + emoji).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument};
    ///
    /// let mut doc = LayoutDocument::new(Document::new());
    /// let roboto = doc.embed_font_file("fonts/Roboto-Regular.ttf").unwrap();
    /// let lxgw = doc.embed_font_file("fonts/LXGWWenKai-Regular.ttf").unwrap();
    ///
    /// doc.font(&roboto).size(14.0);
    /// doc.fallback_fonts(vec![lxgw]);
    ///
    /// // Mixed text will automatically use LXGW for Chinese characters
    /// doc.text("Hello 你好 World 世界");
    /// ```
    pub fn fallback_fonts(&mut self, fonts: Vec<String>) -> &mut Self {
        self.state.fallback_fonts = fonts;
        self
    }

    /// Adds a fallback font to the end of the fallback list
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument};
    ///
    /// let mut doc = LayoutDocument::new(Document::new());
    /// doc.add_fallback_font("LXGW");
    /// doc.add_fallback_font("NotoEmoji");
    /// ```
    pub fn add_fallback_font(&mut self, font: &str) -> &mut Self {
        self.state.fallback_fonts.push(font.to_string());
        self
    }

    /// Clears all fallback fonts
    pub fn clear_fallback_fonts(&mut self) -> &mut Self {
        self.state.fallback_fonts.clear();
        self
    }

    /// Returns the current list of fallback fonts
    pub fn get_fallback_fonts(&self) -> &[String] {
        &self.state.fallback_fonts
    }

    /// Checks if a character is supported by WinAnsiEncoding
    ///
    /// WinAnsiEncoding (CP1252) includes:
    /// - ASCII (0x00-0x7F)
    /// - Latin-1 Supplement (0xA0-0xFF) - note: NOT the C1 control range 0x80-0x9F
    /// - Special characters mapped from higher Unicode code points to positions 0x80-0x9F
    ///
    /// Important: Unicode code points U+0080-U+009F are C1 control characters and are
    /// NOT the same as WinAnsi positions 0x80-0x9F. WinAnsi maps those positions to
    /// characters like € (U+20AC), " (U+201C), etc.
    fn is_winansi_char(c: char) -> bool {
        let cp = c as u32;

        // ASCII range (0x00-0x7F)
        if cp < 128 {
            return true;
        }

        // C1 control characters (U+0080-U+009F) are NOT in WinAnsiEncoding
        // These are distinct from the WinAnsi byte positions 0x80-0x9F
        if (0x80..=0x9F).contains(&cp) {
            return false;
        }

        // Latin-1 Supplement printable range (U+00A0-U+00FF)
        // All characters in this range are supported by WinAnsiEncoding
        if (0xA0..=0xFF).contains(&cp) {
            return true;
        }

        // Unicode characters mapped to WinAnsiEncoding positions 0x80-0x9F
        // These are the actual characters that WinAnsi encodes at those byte positions
        matches!(
            cp,
            0x20AC  // € Euro sign -> 0x80
            | 0x201A  // ‚ Single low-9 quotation mark -> 0x82
            | 0x0192  // ƒ Latin small letter f with hook -> 0x83
            | 0x201E  // „ Double low-9 quotation mark -> 0x84
            | 0x2026  // … Horizontal ellipsis -> 0x85
            | 0x2020  // † Dagger -> 0x86
            | 0x2021  // ‡ Double dagger -> 0x87
            | 0x02C6  // ˆ Modifier letter circumflex accent -> 0x88
            | 0x2030  // ‰ Per mille sign -> 0x89
            | 0x0160  // Š Latin capital letter S with caron -> 0x8A
            | 0x2039  // ‹ Single left-pointing angle quotation -> 0x8B
            | 0x0152  // Œ Latin capital ligature OE -> 0x8C
            | 0x017D  // Ž Latin capital letter Z with caron -> 0x8E
            | 0x2018  // ' Left single quotation mark -> 0x91
            | 0x2019  // ' Right single quotation mark -> 0x92
            | 0x201C  // " Left double quotation mark -> 0x93
            | 0x201D  // " Right double quotation mark -> 0x94
            | 0x2022  // • Bullet -> 0x95
            | 0x2013  // – En dash -> 0x96
            | 0x2014  // — Em dash -> 0x97
            | 0x02DC  // ˜ Small tilde -> 0x98
            | 0x2122  // ™ Trade mark sign -> 0x99
            | 0x0161  // š Latin small letter s with caron -> 0x9A
            | 0x203A  // › Single right-pointing angle quotation -> 0x9B
            | 0x0153  // œ Latin small ligature oe -> 0x9C
            | 0x017E  // ž Latin small letter z with caron -> 0x9E
            | 0x0178 // Ÿ Latin capital letter Y with diaeresis -> 0x9F
        )
    }

    /// Checks if a font has a glyph for the given character
    #[cfg(feature = "fonts")]
    fn glyph_present(&self, c: char, font_name: &str) -> bool {
        // Check embedded fonts first
        if let Some(font) = self.inner.embedded_fonts.get(font_name) {
            return font.has_glyph(c);
        }

        // Standard fonts use WinAnsiEncoding (except Symbol and ZapfDingbats)
        if let Some(std_font) = crate::font::StandardFont::from_name(font_name) {
            return match std_font {
                crate::font::StandardFont::Symbol | crate::font::StandardFont::ZapfDingbats => {
                    // Symbol fonts have their own encoding, conservatively return false
                    // for non-ASCII to trigger fallback for regular text
                    c.is_ascii()
                }
                _ => Self::is_winansi_char(c),
            };
        }

        false
    }

    /// Fallback for non-fonts feature - always returns false for non-standard fonts
    #[cfg(not(feature = "fonts"))]
    fn glyph_present(&self, c: char, font_name: &str) -> bool {
        // Standard fonts use WinAnsiEncoding (except Symbol and ZapfDingbats)
        if let Some(std_font) = crate::font::StandardFont::from_name(font_name) {
            return match std_font {
                crate::font::StandardFont::Symbol | crate::font::StandardFont::ZapfDingbats => {
                    c.is_ascii()
                }
                _ => Self::is_winansi_char(c),
            };
        }
        false
    }

    /// Finds the best font for a character from the primary font and fallback list
    fn find_font_for_glyph_with(
        &self,
        c: char,
        primary_font: &str,
        fallback_fonts: &[String],
    ) -> String {
        // First check the primary font
        if self.glyph_present(c, primary_font) {
            return primary_font.to_string();
        }

        // Try each fallback font in order
        for fallback in fallback_fonts {
            if self.glyph_present(c, fallback) {
                return fallback.clone();
            }
        }

        // No font has the glyph, return primary (will render .notdef)
        primary_font.to_string()
    }

    /// Analyzes text and splits it into runs by font
    ///
    /// Returns a list of (text, font_name) pairs where each run uses a single font.
    fn analyze_text_for_fallback_with(
        &self,
        text: &str,
        primary_font: &str,
        fallback_fonts: &[String],
    ) -> Vec<FontRun> {
        // Fast path: no fallback fonts configured
        if fallback_fonts.is_empty() {
            return vec![FontRun {
                text: text.to_string(),
                font: primary_font.to_string(),
            }];
        }

        let mut runs = Vec::new();
        let mut current_run = String::new();
        let mut current_font = String::new();

        for c in text.chars() {
            let font = self.find_font_for_glyph_with(c, primary_font, fallback_fonts);

            if current_font.is_empty() {
                current_font = font;
                current_run.push(c);
            } else if font == current_font {
                current_run.push(c);
            } else {
                // Font changed, save current run and start new one
                runs.push(FontRun {
                    text: current_run,
                    font: current_font,
                });
                current_run = c.to_string();
                current_font = font;
            }
        }

        // Save the last run
        if !current_run.is_empty() {
            runs.push(FontRun {
                text: current_run,
                font: current_font,
            });
        }

        runs
    }

    /// Returns the current line height based on font metrics and leading
    ///
    /// Uses actual AFM metrics (ascender - descender) for standard fonts,
    /// multiplied by the leading factor. This provides more accurate
    /// line spacing that matches Prawn's behavior.
    pub fn line_height(&self) -> f64 {
        self.font_height() * self.state.leading
    }

    /// Returns the natural height of the current font (ascender - descender + line_gap)
    ///
    /// This matches Prawn's line height calculation which includes line_gap.
    pub fn font_height(&self) -> f64 {
        use crate::font::StandardFont;

        let font_size = self.inner.current_font_size;

        // Try standard fonts (AFM metrics)
        if let Some(font) = StandardFont::from_name(&self.inner.current_font) {
            let metrics = font.metrics();
            let height_units = metrics.ascender - metrics.descender + metrics.line_gap;
            return height_units as f64 * font_size / 1000.0;
        }

        // Try embedded fonts (TTF metrics)
        #[cfg(feature = "fonts")]
        if let Some(embedded) = self.inner.get_embedded_font(&self.inner.current_font) {
            let height_units = embedded.ascender - embedded.descender + embedded.line_gap;
            return height_units as f64 * font_size / 1000.0;
        }

        font_size * 1.15
    }

    /// Returns the ascender height of the current font
    ///
    /// This is the height from baseline to top of tallest character.
    pub fn ascender_height(&self) -> f64 {
        use crate::font::StandardFont;

        let font_size = self.inner.current_font_size;

        if let Some(font) = StandardFont::from_name(&self.inner.current_font) {
            let metrics = font.metrics();
            return metrics.ascender as f64 * font_size / 1000.0;
        }

        #[cfg(feature = "fonts")]
        if let Some(embedded) = self.inner.get_embedded_font(&self.inner.current_font) {
            return embedded.ascender as f64 * font_size / 1000.0;
        }

        font_size * 0.72
    }

    /// Returns font_height for a specific font and size
    fn font_height_for(&self, font_name: &str, font_size: f64) -> f64 {
        use crate::font::StandardFont;

        if let Some(font) = StandardFont::from_name(font_name) {
            let metrics = font.metrics();
            let height_units = metrics.ascender - metrics.descender + metrics.line_gap;
            return height_units as f64 * font_size / 1000.0;
        }

        #[cfg(feature = "fonts")]
        if let Some(embedded) = self.inner.get_embedded_font(font_name) {
            let height_units = embedded.ascender - embedded.descender + embedded.line_gap;
            return height_units as f64 * font_size / 1000.0;
        }

        font_size * 1.15
    }

    /// Returns ascender_height for a specific font and size
    fn ascender_height_for(&self, font_name: &str, font_size: f64) -> f64 {
        use crate::font::StandardFont;

        if let Some(font) = StandardFont::from_name(font_name) {
            let metrics = font.metrics();
            return metrics.ascender as f64 * font_size / 1000.0;
        }

        #[cfg(feature = "fonts")]
        if let Some(embedded) = self.inner.get_embedded_font(font_name) {
            return embedded.ascender as f64 * font_size / 1000.0;
        }

        font_size * 0.72
    }

    /// Returns the current font name
    pub fn current_font(&self) -> &str {
        &self.inner.current_font
    }

    /// Returns the current font size in points
    pub fn current_font_size(&self) -> f64 {
        self.inner.current_font_size
    }

    /// Measures the width of text with the current font settings
    ///
    /// This includes character spacing but not word spacing.
    pub fn measure_text(&self, text: &str) -> f64 {
        self.measure_text_width_with_spacing(text)
    }

    /// Draws text at the current cursor position
    ///
    /// The cursor is moved down by the line height after drawing.
    /// Text is not wrapped - use `text_wrap` for automatic wrapping.
    ///
    /// When fallback fonts are configured (globally or per-call), the text is
    /// automatically analyzed and split into runs using the appropriate font
    /// for each character.
    ///
    /// Note: Like Prawn, the cursor represents the TOP of the text line.
    /// The text baseline is positioned below the cursor by the font ascender.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument};
    ///
    /// let mut doc = LayoutDocument::new(Document::new());
    /// doc.text("Hello World");
    /// ```
    pub fn text(&mut self, text: &str) -> &mut Self {
        // Empty text is a no-op (matches Prawn behavior)
        if text.is_empty() {
            return self;
        }

        // If fallback fonts are configured, use fallback-aware rendering
        if !self.state.fallback_fonts.is_empty() {
            let fallback_fonts = self.state.fallback_fonts.clone();
            return self.text_with_fallback_fonts(text, &fallback_fonts);
        }

        // Original implementation (no fallback)
        self.text_simple(text)
    }

    /// Draws text with options at the current cursor position
    ///
    /// This method allows passing per-call options like fallback fonts,
    /// similar to Prawn's text method with options hash.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, TextOptions};
    ///
    /// let mut doc = LayoutDocument::new(Document::new());
    /// let lxgw = doc.embed_font_file("fonts/LXGWWenKai-Regular.ttf").unwrap();
    ///
    /// // Use fallback fonts for this specific text call
    /// doc.text_opts("Hello 你好", TextOptions::new().fallback_fonts(vec![lxgw]));
    /// ```
    pub fn text_opts(&mut self, text: &str, options: TextOptions) -> &mut Self {
        // Per-call fallback fonts take precedence over global ones
        let fallback_fonts = options
            .fallback_fonts
            .unwrap_or_else(|| self.state.fallback_fonts.clone());

        if !fallback_fonts.is_empty() {
            return self.text_with_fallback_fonts(text, &fallback_fonts);
        }

        self.text_simple(text)
    }

    /// Internal: renders text without fallback processing
    fn text_simple(&mut self, text: &str) -> &mut Self {
        let bounds = self.state.bounds();
        let left = bounds.absolute_left();
        let right = bounds.absolute_right();
        let width = right - left;

        // In Prawn, cursor is at the TOP of the text line.
        // Text baseline = cursor - ascender_height (matches Prawn behavior)
        let ascender_offset = self.ascender_height();
        let y = self.state.cursor_y - ascender_offset;

        // Calculate text width including character and word spacing
        let text_width = self.measure_text_width_with_spacing(text);

        // Calculate x position based on alignment
        let x = match self.state.text_align {
            TextAlign::Left => left,
            TextAlign::Center => left + (width - text_width) / 2.0,
            TextAlign::Right => right - text_width,
            TextAlign::Justify => left, // Justify not supported for single-line text
        };

        // Draw text with spacing applied
        self.draw_text_with_spacing(text, [x, y]);

        // Move cursor down by line height
        let line_height = self.line_height();
        self.state.cursor_y -= line_height;

        // Update stretched height if in stretchy box
        let cursor_y = self.state.cursor_y;
        self.state.bounds_mut().update_stretched_height(cursor_y);

        // Column overflow check
        self.check_column_overflow();

        self
    }

    /// Internal: renders text with font fallback support
    fn text_with_fallback_fonts(&mut self, text: &str, fallback_fonts: &[String]) -> &mut Self {
        let primary_font = &self.inner.current_font;
        let runs = self.analyze_text_for_fallback_with(text, primary_font, fallback_fonts);

        // Convert runs to TextFragments
        let fragments: Vec<TextFragment> = runs
            .into_iter()
            .map(|run| {
                let mut frag = TextFragment::new(run.text);
                if run.font != *primary_font {
                    frag = frag.font(run.font);
                }
                frag
            })
            .collect();

        self.formatted_text(&fragments)
    }

    /// Draws formatted text with mixed styles at the current cursor position
    ///
    /// This method allows rendering text with different styles (bold, italic),
    /// colors, and sizes within the same line, similar to Prawn's `formatted_text`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{LayoutDocument, Document, TextFragment, FontStyle, Color};
    ///
    /// let doc = Document::new();
    /// let mut layout = LayoutDocument::new(doc);
    /// layout.formatted_text(&[
    ///     TextFragment::new("Bold ").bold(),
    ///     TextFragment::new("and "),
    ///     TextFragment::new("Red").color(Color::RED),
    /// ]);
    /// ```
    pub fn formatted_text(&mut self, fragments: &[TextFragment]) -> &mut Self {
        if fragments.is_empty() {
            return self;
        }

        let bounds = self.state.bounds();
        let left = bounds.absolute_left();
        let right = bounds.absolute_right();
        let width = right - left;

        let base_font = self.inner.current_font.clone();
        let base_size = self.inner.current_font_size;
        let fallback_fonts = self.state.fallback_fonts.clone();

        // Calculate total width for alignment (including spacing)
        let char_spacing = self.state.character_spacing;
        let word_spacing = self.state.word_spacing;

        // Count non-empty fragments for inter-fragment spacing
        let non_empty_count = fragments.iter().filter(|f| !f.text.is_empty()).count();

        let fragment_widths: f64 = fragments
            .iter()
            .filter(|f| !f.text.is_empty())
            .map(|f| {
                let primary_font = if let Some(ref font) = f.font {
                    font.as_str()
                } else {
                    base_font.as_str()
                };
                let styled_font = if f.font.is_some() {
                    None
                } else {
                    Some(Self::get_styled_font_name(primary_font, f.style))
                };
                let mut font_size = f.size.unwrap_or(base_size);
                // Superscript/subscript use smaller font size
                if f.superscript || f.subscript {
                    font_size *= 0.583;
                }

                // Calculate width with fallback support
                let base_width = if fallback_fonts.is_empty() {
                    match styled_font.as_deref() {
                        Some(styled) => {
                            self.measure_fragment_width_inner(styled, &f.text, font_size)
                        }
                        None => self.measure_fragment_width_inner(primary_font, &f.text, font_size),
                    }
                } else {
                    // Split by fallback fonts and measure each run
                    let primary_font = styled_font.as_deref().unwrap_or(primary_font);
                    let runs =
                        self.analyze_text_for_fallback_with(&f.text, primary_font, &fallback_fonts);
                    runs.iter()
                        .map(|run| {
                            self.measure_fragment_width_inner(&run.font, &run.text, font_size)
                        })
                        .sum()
                };

                // Add spacing within fragment
                let char_count = f.text.chars().count();
                let char_spacing_width = if char_count > 0 {
                    char_spacing * (char_count as f64 - 1.0)
                } else {
                    0.0
                };
                let space_count = f.text.chars().filter(|&c| c == ' ').count();
                let word_spacing_width = word_spacing * space_count as f64;

                base_width + char_spacing_width + word_spacing_width
            })
            .sum();

        // Add inter-fragment character spacing (one between each pair of adjacent fragments)
        let inter_fragment_spacing = if non_empty_count > 1 {
            char_spacing * (non_empty_count as f64 - 1.0)
        } else {
            0.0
        };

        let total_width = fragment_widths + inter_fragment_spacing;

        // Calculate starting x position based on alignment
        let start_x = match self.state.text_align {
            TextAlign::Left => left,
            TextAlign::Center => left + (width - total_width) / 2.0,
            TextAlign::Right => right - total_width,
            TextAlign::Justify => left,
        };

        // Calculate max ascender and line height across all fragments
        let mut max_ascender = self.ascender_height_for(&base_font, base_size);
        let mut max_line_height = self.font_height_for(&base_font, base_size);
        for f in fragments {
            if f.text.is_empty() {
                continue;
            }
            let frag_font = if let Some(ref font) = f.font {
                font.clone()
            } else {
                Self::get_styled_font_name(&base_font, f.style)
            };
            let mut frag_size = f.size.unwrap_or(base_size);
            if f.superscript || f.subscript {
                frag_size *= 0.583;
            }
            let asc = self.ascender_height_for(&frag_font, frag_size);
            let fh = self.font_height_for(&frag_font, frag_size);
            if asc > max_ascender {
                max_ascender = asc;
            }
            if fh > max_line_height {
                max_line_height = fh;
            }
        }

        // Calculate y position (baseline) using max ascender across fragments
        let ascender_offset = max_ascender;
        let y = self.state.cursor_y - ascender_offset;

        let mut x = start_x;

        let mut is_first_fragment = true;

        for fragment in fragments {
            if fragment.text.is_empty() {
                continue;
            }

            // Determine primary font name based on style
            let primary_font = if let Some(ref font) = fragment.font {
                font.as_str()
            } else {
                base_font.as_str()
            };
            let styled_font = if fragment.font.is_some() {
                None
            } else {
                Some(Self::get_styled_font_name(primary_font, fragment.style))
            };

            let mut frag_font_size = fragment.size.unwrap_or(base_size);
            let char_spacing = self.state.character_spacing;
            let word_spacing = self.state.word_spacing;

            // Superscript/subscript: reduce size and adjust y offset
            let mut frag_y = y;
            if fragment.superscript {
                frag_font_size *= 0.583;
                frag_y += base_size * 0.33; // raise
            } else if fragment.subscript {
                frag_font_size *= 0.583;
                frag_y -= base_size * 0.17; // lower
            }

            // Add inter-fragment spacing BEFORE drawing (not first fragment)
            if !is_first_fragment {
                x += char_spacing;
            }

            // Record fragment start x for decorations
            let frag_start_x = x;

            // Split text into runs by fallback fonts (or single run if no fallback)
            let runs = if fallback_fonts.is_empty() {
                vec![FontRun {
                    text: fragment.text.clone(),
                    font: styled_font.unwrap_or_else(|| primary_font.to_string()),
                }]
            } else {
                let primary_font = styled_font.as_deref().unwrap_or(primary_font);
                self.analyze_text_for_fallback_with(&fragment.text, primary_font, &fallback_fonts)
            };

            // Render each run with the appropriate font
            let mut is_first_run = true;
            for run in &runs {
                if run.text.is_empty() {
                    continue;
                }

                // Ensure font is registered
                self.inner.ensure_font(&run.font);

                // Check if this is an embedded font
                #[cfg(feature = "fonts")]
                let is_embedded = self.inner.embedded_fonts.contains_key(&run.font);
                #[cfg(not(feature = "fonts"))]
                let is_embedded = false;

                // Add character spacing between runs within same fragment
                if !is_first_run {
                    x += char_spacing;
                }

                let text_width = if is_embedded {
                    #[cfg(feature = "fonts")]
                    {
                        self.draw_embedded_fragment(
                            &run.font,
                            &run.text,
                            frag_font_size,
                            x,
                            frag_y,
                            char_spacing,
                            word_spacing,
                            fragment.color,
                        )
                    }
                    #[cfg(not(feature = "fonts"))]
                    {
                        0.0
                    }
                } else {
                    // Standard font path
                    let page = &mut self.inner.pages[self.inner.current_page];

                    // Save state if we need to change color
                    let needs_color_change = fragment.color.is_some();
                    if needs_color_change {
                        page.content.save_state();
                        if let Some(color) = fragment.color {
                            page.content.set_fill_color_rgb(color.r, color.g, color.b);
                        }
                    }

                    page.content.begin_text();
                    page.content.set_character_spacing(char_spacing);
                    page.content.set_word_spacing(word_spacing);
                    page.content
                        .set_font(&run.font, frag_font_size)
                        .move_text_pos(x, frag_y);

                    // Apply kerning for standard fonts
                    use crate::font::StandardFont;
                    if let Some(std_font) = StandardFont::from_name(&run.font) {
                        let chunks = crate::font::kern_tables::kern_text(&std_font, &run.text);
                        page.content.show_text_kerned(&chunks);
                    } else {
                        page.content.show_text(&run.text);
                    }
                    page.content.end_text();

                    if needs_color_change {
                        page.content.restore_state();
                    }

                    Self::measure_fragment_width_kerned(&run.font, &run.text, frag_font_size)
                };

                // Calculate extra width from character and word spacing within this run
                let char_count = run.text.chars().count();
                let char_spacing_extra = if char_count > 0 {
                    char_spacing * (char_count as f64 - 1.0)
                } else {
                    0.0
                };
                let space_count = run.text.chars().filter(|&c| c == ' ').count();
                let word_spacing_extra = word_spacing * space_count as f64;
                x += text_width + char_spacing_extra + word_spacing_extra;

                is_first_run = false;
            }

            // Fragment end x (for decorations)
            let frag_end_x = x;

            // Draw underline decoration
            if fragment.underline {
                let line_y = y - base_size * 0.15; // below baseline
                let line_width = base_size * 0.05;
                let page = &mut self.inner.pages[self.inner.current_page];
                page.content.save_state();
                if let Some(color) = fragment.color {
                    page.content.set_stroke_color_rgb(color.r, color.g, color.b);
                }
                page.content.set_line_width(line_width);
                page.content.move_to(frag_start_x, line_y);
                page.content.line_to(frag_end_x, line_y);
                page.content.stroke();
                page.content.restore_state();
            }

            // Draw strikethrough decoration
            if fragment.strikethrough {
                let line_y = y + base_size * 0.25; // middle of text
                let line_width = base_size * 0.05;
                let page = &mut self.inner.pages[self.inner.current_page];
                page.content.save_state();
                if let Some(color) = fragment.color {
                    page.content.set_stroke_color_rgb(color.r, color.g, color.b);
                }
                page.content.set_line_width(line_width);
                page.content.move_to(frag_start_x, line_y);
                page.content.line_to(frag_end_x, line_y);
                page.content.stroke();
                page.content.restore_state();
            }

            // Add link annotation
            if let Some(ref url) = fragment.link {
                let rect = [
                    frag_start_x,
                    y - base_size * 0.22, // below descender
                    frag_end_x,
                    y + base_size * 0.78, // above ascender
                ];
                self.inner
                    .link_annotation(super::link::LinkAnnotation::url(rect, url));
            }

            is_first_fragment = false;
        }

        // Move cursor down by max line height across all fragments
        let line_height = max_line_height * self.state.leading;
        self.state.cursor_y -= line_height;

        // Update stretched height if in stretchy box
        let cursor_y = self.state.cursor_y;
        self.state.bounds_mut().update_stretched_height(cursor_y);

        // Column overflow check
        self.check_column_overflow();

        self
    }

    /// Draws a text fragment using an embedded font, returns the text width
    #[cfg(feature = "fonts")]
    #[allow(clippy::too_many_arguments)]
    fn draw_embedded_fragment(
        &mut self,
        font_name: &str,
        text: &str,
        font_size: f64,
        x: f64,
        y: f64,
        char_spacing: f64,
        word_spacing: f64,
        color: Option<Color>,
    ) -> f64 {
        let font = match self.inner.embedded_fonts.get(font_name) {
            Some(font) => font.clone(),
            None => return 0.0,
        };

        let glyphs = font.shape_text(text);
        if glyphs.is_empty() {
            return 0.0;
        }

        // Track used glyphs for subsetting
        self.inner.track_font_glyphs(font_name, &glyphs);

        // Calculate text width from glyph advances
        let total_advance: i32 = glyphs.iter().map(|g| g.x_advance).sum();
        let text_width = total_advance as f64 * font_size / 1000.0;

        // Build hex string for glyph IDs
        let mut hex = String::with_capacity(glyphs.len() * 4);
        for glyph in &glyphs {
            hex.push_str(&format!("{:04X}", glyph.gid));
        }

        let page = &mut self.inner.pages[self.inner.current_page];

        // Save state if we need to change color
        let needs_color_change = color.is_some();
        if needs_color_change {
            page.content.save_state();
            if let Some(c) = color {
                page.content.set_fill_color_rgb(c.r, c.g, c.b);
            }
        }

        page.content.begin_text();
        page.content.set_character_spacing(char_spacing);
        page.content.set_word_spacing(word_spacing);
        page.content
            .set_font(font_name, font_size)
            .move_text_pos(x, y)
            .show_text_hex(&hex)
            .end_text();

        if needs_color_change {
            page.content.restore_state();
        }

        text_width
    }

    /// Measures fragment width, supporting both standard and embedded fonts
    fn measure_fragment_width_inner(&self, font_name: &str, text: &str, font_size: f64) -> f64 {
        #[cfg(feature = "fonts")]
        {
            if let Some(font) = self.inner.embedded_fonts.get(font_name) {
                let glyphs = font.shape_text(text);
                let total_advance: i32 = glyphs.iter().map(|g| g.x_advance).sum();
                return total_advance as f64 * font_size / 1000.0;
            }
        }
        Self::measure_fragment_width_kerned(font_name, text, font_size)
    }

    /// Returns the styled font name for standard fonts
    fn get_styled_font_name(base_font: &str, style: FontStyle) -> String {
        let base = if base_font.starts_with("Helvetica") {
            "Helvetica"
        } else if base_font.starts_with("Times") {
            "Times"
        } else if base_font.starts_with("Courier") {
            "Courier"
        } else {
            return base_font.to_string();
        };

        match (base, style) {
            ("Helvetica", FontStyle::Normal) => "Helvetica".to_string(),
            ("Helvetica", FontStyle::Bold) => "Helvetica-Bold".to_string(),
            ("Helvetica", FontStyle::Italic) => "Helvetica-Oblique".to_string(),
            ("Helvetica", FontStyle::BoldItalic) => "Helvetica-BoldOblique".to_string(),
            ("Times", FontStyle::Normal) => "Times-Roman".to_string(),
            ("Times", FontStyle::Bold) => "Times-Bold".to_string(),
            ("Times", FontStyle::Italic) => "Times-Italic".to_string(),
            ("Times", FontStyle::BoldItalic) => "Times-BoldItalic".to_string(),
            ("Courier", FontStyle::Normal) => "Courier".to_string(),
            ("Courier", FontStyle::Bold) => "Courier-Bold".to_string(),
            ("Courier", FontStyle::Italic) => "Courier-Oblique".to_string(),
            ("Courier", FontStyle::BoldItalic) => "Courier-BoldOblique".to_string(),
            _ => base_font.to_string(),
        }
    }

    /// Measures text width for a given font (static helper)
    fn measure_fragment_width(font_name: &str, text: &str, font_size: f64) -> f64 {
        use crate::font::StandardFont;
        if let Some(font) = StandardFont::from_name(font_name) {
            font.string_width(text) as f64 * font_size / 1000.0
        } else {
            text.len() as f64 * font_size * 0.5
        }
    }

    /// Measures text width including kerning adjustments
    fn measure_fragment_width_kerned(font_name: &str, text: &str, font_size: f64) -> f64 {
        use crate::font::StandardFont;
        if let Some(font) = StandardFont::from_name(font_name) {
            let raw = font.string_width(text) as f64;
            let kern = crate::font::kern_tables::total_kern_adjustment(&font, text) as f64;
            (raw + kern) * font_size / 1000.0
        } else {
            text.len() as f64 * font_size * 0.5
        }
    }

    /// Draws wrapped text at the current cursor position
    ///
    /// Text is automatically wrapped to fit within the current bounds width.
    /// Each line respects the current alignment setting.
    ///
    /// For `TextAlign::Justify`, lines are stretched to fill the width by
    /// adjusting word spacing. The last line of each paragraph uses left
    /// alignment instead of being justified.
    pub fn text_wrap(&mut self, text: &str) -> &mut Self {
        let bounds = self.state.bounds();
        let width = bounds.width();

        // Check if we need special justify handling
        if self.state.text_align == TextAlign::Justify {
            // Use justify-specific wrapping (excludes word_spacing from calculations)
            let lines = self.wrap_text_for_justify(text, width);

            for (line, is_paragraph_end) in lines {
                if is_paragraph_end || line.is_empty() {
                    // Last line of paragraph or empty line - use left alignment
                    self.text_line_left(&line);
                } else {
                    // Non-last line - justify by stretching word spacing
                    self.text_line_justified(&line, width);
                }
            }
        } else {
            // Non-justify alignment - use regular text() method
            let lines = self.wrap_text_to_width(text, width);
            for line in lines {
                self.text(&line);
            }
        }

        self
    }

    /// Draws inline-formatted text at the current cursor position
    ///
    /// Parses HTML-like tags (`<b>`, `<i>`, `<u>`, etc.) and renders them
    /// as a single formatted line using `formatted_text()`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument};
    ///
    /// let mut doc = LayoutDocument::new(Document::new());
    /// doc.text_inline("Hello <b>bold</b> and <i>italic</i>");
    /// ```
    pub fn text_inline(&mut self, text: &str) -> &mut Self {
        let fragments = crate::api::inline_format::parse(text);
        if fragments.is_empty() {
            return self;
        }

        // Split fragments at newline boundaries and render each line separately
        let mut current_line: Vec<TextFragment> = Vec::new();
        for frag in &fragments {
            if frag.text == "\n" {
                // Flush current line
                if current_line.is_empty() {
                    // Empty line — just advance cursor
                    let line_height = self.line_height();
                    self.state.cursor_y -= line_height;
                } else {
                    self.formatted_text(&current_line);
                    current_line.clear();
                }
            } else {
                current_line.push(frag.clone());
            }
        }
        // Flush remaining
        if !current_line.is_empty() {
            self.formatted_text(&current_line);
        }

        self
    }

    /// Draws inline-formatted text with automatic word wrapping
    ///
    /// Parses HTML-like tags and wraps the resulting fragments across
    /// multiple lines to fit within the current bounds width.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument};
    ///
    /// let mut doc = LayoutDocument::new(Document::new());
    /// doc.text_wrap_inline("This is a <b>long</b> paragraph with <i>mixed</i> styles that will wrap automatically.");
    /// ```
    pub fn text_wrap_inline(&mut self, text: &str) -> &mut Self {
        let fragments = crate::api::inline_format::parse(text);
        if fragments.is_empty() {
            return self;
        }
        self.formatted_text_wrap(&fragments)
    }

    /// Draws formatted text fragments with automatic word wrapping
    ///
    /// Splits fragments into lines that fit within the current bounds width,
    /// preserving the styling of each fragment across line breaks.
    pub fn formatted_text_wrap(&mut self, fragments: &[TextFragment]) -> &mut Self {
        if fragments.is_empty() {
            return self;
        }

        let bounds = self.state.bounds();
        let max_width = bounds.width();
        let lines = self.wrap_fragments_to_lines(fragments, max_width);

        for line in &lines {
            self.formatted_text(line);
        }

        self
    }

    /// Wraps a sequence of fragments into lines that fit within `max_width`.
    ///
    /// Splitting happens at word boundaries (whitespace). Each fragment may
    /// be split across multiple lines, and each resulting piece retains the
    /// original fragment's styling.
    fn wrap_fragments_to_lines(
        &self,
        fragments: &[TextFragment],
        max_width: f64,
    ) -> Vec<Vec<TextFragment>> {
        // Flatten fragments into (word, style) tokens, honoring newlines.
        // A "word" here is a whitespace-delimited run of text.
        // We track leading/trailing spaces to reconstruct spacing faithfully.

        struct Token {
            text: String,
            frag_index: usize, // which original fragment this came from
            is_newline: bool,
        }

        let mut tokens: Vec<Token> = Vec::new();

        for (idx, frag) in fragments.iter().enumerate() {
            if frag.text == "\n" {
                tokens.push(Token {
                    text: String::new(),
                    frag_index: idx,
                    is_newline: true,
                });
                continue;
            }

            // Split fragment text into words, preserving whitespace boundaries
            let text = &frag.text;
            let mut start = 0;
            let chars: Vec<char> = text.chars().collect();
            let len = chars.len();

            while start < len {
                // Skip whitespace (we'll add a space when joining words)
                if chars[start].is_whitespace() {
                    start += 1;
                    continue;
                }

                // Collect word (non-whitespace run)
                let word_start = start;
                while start < len && !chars[start].is_whitespace() {
                    start += 1;
                }

                let word: String = chars[word_start..start].iter().collect();

                // Check if there was a leading space before this word in the fragment
                let has_leading_space = word_start > 0 || {
                    // If this is not the first token, and the previous fragment
                    // ended in whitespace or this fragment starts with whitespace
                    idx > 0
                        && !tokens.is_empty()
                        && !tokens.last().is_none_or(|t| t.is_newline)
                        && frag.text.starts_with(|c: char| c.is_whitespace())
                        && word_start == 0
                };

                let _ = has_leading_space; // We handle spacing in line assembly below

                tokens.push(Token {
                    text: word,
                    frag_index: idx,
                    is_newline: false,
                });
            }

            // Handle fragment that is only whitespace (like " ")
            // This acts as a separator; if no words were extracted, the spacing
            // will naturally be handled when assembling lines.
        }

        // Now assemble lines by accumulating words until width is exceeded.
        let base_font = &self.inner.current_font;
        let base_size = self.inner.current_font_size;

        let measure_word = |word: &str, frag: &TextFragment| -> f64 {
            let font_name = if let Some(ref f) = frag.font {
                f.as_str()
            } else {
                base_font.as_str()
            };
            let styled = if frag.font.is_some() {
                font_name.to_string()
            } else {
                Self::get_styled_font_name(font_name, frag.style)
            };
            let font_size = frag.size.unwrap_or(base_size);
            self.measure_fragment_width_inner(&styled, word, font_size)
        };

        // Measure a single space in the base font
        let space_width = { Self::measure_fragment_width(base_font, " ", base_size) };

        let mut lines: Vec<Vec<TextFragment>> = Vec::new();
        let mut current_line: Vec<(String, usize)> = Vec::new(); // (word, frag_index)
        let mut current_width: f64 = 0.0;

        for token in &tokens {
            if token.is_newline {
                // Flush current line
                lines.push(self.assemble_line(&current_line, fragments));
                current_line.clear();
                current_width = 0.0;
                continue;
            }

            let frag = &fragments[token.frag_index];
            let word_w = measure_word(&token.text, frag);

            let needed = if current_line.is_empty() {
                word_w
            } else {
                space_width + word_w
            };

            if current_width + needed <= max_width || current_line.is_empty() {
                // Fits on current line (or first word, always add)
                current_width += needed;
                current_line.push((token.text.clone(), token.frag_index));
            } else {
                // Line break
                lines.push(self.assemble_line(&current_line, fragments));
                current_line.clear();
                current_width = word_w;
                current_line.push((token.text.clone(), token.frag_index));
            }
        }

        // Flush remaining
        if !current_line.is_empty() {
            lines.push(self.assemble_line(&current_line, fragments));
        }

        // Ensure at least one line (empty)
        if lines.is_empty() {
            lines.push(Vec::new());
        }

        lines
    }

    /// Assemble a line from (word, frag_index) pairs into fragments,
    /// merging consecutive words that share the same fragment style.
    fn assemble_line(
        &self,
        words: &[(String, usize)],
        original_fragments: &[TextFragment],
    ) -> Vec<TextFragment> {
        if words.is_empty() {
            return Vec::new();
        }

        let mut line_frags: Vec<TextFragment> = Vec::new();
        let mut current_text = String::new();
        let mut current_idx = words[0].1;

        for (i, (word, frag_idx)) in words.iter().enumerate() {
            if *frag_idx != current_idx {
                // Flush accumulated text
                if !current_text.is_empty() {
                    let orig = &original_fragments[current_idx];
                    line_frags.push(self.clone_fragment_with_text(orig, &current_text));
                    current_text.clear();
                }
                current_idx = *frag_idx;
            }

            if i > 0 && !current_text.is_empty() {
                current_text.push(' ');
            } else if i > 0 {
                // Different fragment, add space to previous fragment if non-empty
                // or add space prefix here
                // Actually, inter-fragment spacing: if we're starting a new fragment
                // and it's not the first word on the line, add a space
                if !line_frags.is_empty() {
                    // Append space to previous fragment
                    if let Some(prev) = line_frags.last_mut() {
                        prev.text.push(' ');
                    }
                }
            }

            current_text.push_str(word);
        }

        // Flush last accumulated text
        if !current_text.is_empty() {
            let orig = &original_fragments[current_idx];
            line_frags.push(self.clone_fragment_with_text(orig, &current_text));
        }

        line_frags
    }

    /// Clone a fragment's style with new text content
    fn clone_fragment_with_text(&self, original: &TextFragment, text: &str) -> TextFragment {
        TextFragment {
            text: text.to_string(),
            style: original.style,
            color: original.color,
            size: original.size,
            font: original.font.clone(),
            underline: original.underline,
            strikethrough: original.strikethrough,
            superscript: original.superscript,
            subscript: original.subscript,
            link: original.link.clone(),
        }
    }

    /// Internal: draws a single line with left alignment (for justify paragraph ends)
    ///
    /// In justify mode, this is used for paragraph-ending lines which should NOT
    /// have user-set word_spacing applied (only character_spacing is used).
    fn text_line_left(&mut self, text: &str) {
        let bounds = self.state.bounds();
        let left = bounds.absolute_left();
        let font_size = self.inner.current_font_size;

        let ascender_offset = self.ascender_height();
        let y = self.state.cursor_y - ascender_offset;

        // For justify mode paragraph ends, use character_spacing but zero word_spacing
        // This matches the wrapping calculation which excludes word_spacing
        let char_spacing = self.state.character_spacing;

        // Support font fallback
        if self.state.fallback_fonts.is_empty() {
            self.inner
                .text_at_with_spacing(text, [left, y], char_spacing, 0.0);
        } else {
            let primary_font = self.inner.current_font.clone();
            let runs = {
                let fallback_fonts = &self.state.fallback_fonts;
                self.analyze_text_for_fallback_with(text, &primary_font, fallback_fonts)
            };

            let mut x = left;
            let mut is_first_run = true;
            for run in &runs {
                if run.text.is_empty() {
                    continue;
                }
                // Add character_spacing between runs (not before first run)
                if !is_first_run {
                    x += char_spacing;
                }
                is_first_run = false;

                self.inner.ensure_font(&run.font);
                self.inner.font(&run.font);
                self.inner
                    .text_at_with_spacing(&run.text, [x, y], char_spacing, 0.0);
                x += self.measure_fragment_width_inner(&run.font, &run.text, font_size)
                    + char_spacing * (run.text.chars().count().saturating_sub(1)) as f64;
            }
            // Restore primary font
            self.inner.font(&primary_font);
        }

        let line_height = self.line_height();
        self.state.cursor_y -= line_height;

        let cursor_y = self.state.cursor_y;
        self.state.bounds_mut().update_stretched_height(cursor_y);

        // Column overflow check
        self.check_column_overflow();
    }

    /// Internal: draws a single line with justified alignment
    fn text_line_justified(&mut self, text: &str, width: f64) {
        let ascender_offset = self.ascender_height();
        let y = self.state.cursor_y - ascender_offset;

        self.draw_text_justified(text, y, width);

        let line_height = self.line_height();
        self.state.cursor_y -= line_height;

        let cursor_y = self.state.cursor_y;
        self.state.bounds_mut().update_stretched_height(cursor_y);

        // Column overflow check
        self.check_column_overflow();
    }

    /// Draws text in a bounding box with automatic wrapping
    ///
    /// This is similar to Prawn's `text_box` method. Text is wrapped to fit
    /// within the specified dimensions. Overflow behavior is controlled by
    /// the `overflow` parameter.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to render
    /// * `point` - Position offset from current cursor [x, y]
    /// * `width` - Width of the text box
    /// * `height` - Fixed height of the text box
    /// * `overflow` - How to handle text that doesn't fit
    ///
    /// # Returns
    ///
    /// A `TextBoxResult` containing information about the rendered text,
    /// including actual height used and whether text was truncated.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, Overflow};
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    ///
    /// // Simple truncation (default)
    /// let result = layout.text_box(
    ///     "Long text that will wrap...",
    ///     [0.0, 0.0], 200.0, 100.0,
    ///     Overflow::Truncate,
    /// );
    ///
    /// // Shrink font to fit
    /// let result = layout.text_box(
    ///     "Text that must fit...",
    ///     [0.0, 0.0], 200.0, 50.0,
    ///     Overflow::ShrinkToFit(6.0),  // min 6pt
    /// );
    ///
    /// // Expand box height
    /// let result = layout.text_box(
    ///     "Lots of text...",
    ///     [0.0, 0.0], 200.0, 50.0,  // initial height
    ///     Overflow::Expand,
    /// );
    /// println!("Actual height: {}", result.height);
    /// ```
    pub fn text_box(
        &mut self,
        text: &str,
        point: [f64; 2],
        width: f64,
        height: f64,
        overflow: Overflow,
    ) -> TextBoxResult {
        match overflow {
            Overflow::Truncate => self.text_box_truncate(text, point, width, height),
            Overflow::ShrinkToFit(min_size) => {
                self.text_box_shrink(text, point, width, height, min_size)
            }
            Overflow::Expand => self.text_box_expand(text, point, width, height),
        }
    }

    /// Text box with truncation (default behavior)
    fn text_box_truncate(
        &mut self,
        text: &str,
        point: [f64; 2],
        width: f64,
        height: f64,
    ) -> TextBoxResult {
        let original_size = self.inner.current_font_size;
        let line_height = self.line_height();
        // Prawn calculates that last line only needs ascender height, not full line_height
        // Space for n lines = (n-1) * line_height + ascender
        // So max_lines = floor((height - ascender) / line_height) + 1
        let ascender = self.ascender_height();
        let max_lines = if height >= ascender {
            ((height - ascender) / line_height + 1.0).floor() as usize
        } else {
            0
        };
        let lines = self.wrap_text_to_width(text, width);
        let total_lines = lines.len();
        let lines_to_render = lines.len().min(max_lines);

        // Prawn-style: point is relative to bounds (same as bounding_box)
        // point[0] is x offset from bounds.left
        // point[1] is y position from bounds.bottom (Y increases upward)
        self.bounding_box(point, width, Some(height), |doc| {
            for line in lines.iter().take(lines_to_render) {
                doc.text(line);
            }
        });

        TextBoxResult {
            height,
            truncated: total_lines > lines_to_render,
            font_size: original_size,
            lines_rendered: lines_to_render,
            total_lines,
        }
    }

    /// Text box with font shrinking to fit
    fn text_box_shrink(
        &mut self,
        text: &str,
        point: [f64; 2],
        width: f64,
        height: f64,
        min_size: f64,
    ) -> TextBoxResult {
        let original_size = self.inner.current_font_size;
        let mut current_size = original_size;

        // Binary search for the best font size
        let mut low = min_size;
        let mut high = original_size;

        // First check if original size fits
        let lines_at_original = self.wrap_text_to_width(text, width);
        let line_height_at_original = self.line_height();
        let ascender_at_original = self.ascender_height();
        let max_lines_at_original = if height >= ascender_at_original {
            ((height - ascender_at_original) / line_height_at_original + 1.0).floor() as usize
        } else {
            0
        };

        if lines_at_original.len() <= max_lines_at_original {
            // Original size fits, use it
            return self.text_box_truncate(text, point, width, height);
        }

        // Need to shrink - binary search for best size
        for _ in 0..10 {
            // Max 10 iterations for precision
            let mid = (low + high) / 2.0;
            self.inner.current_font_size = mid;

            let lines = self.wrap_text_to_width(text, width);
            let line_height = self.line_height();
            let ascender = self.ascender_height();
            let max_lines = if height >= ascender {
                ((height - ascender) / line_height + 1.0).floor() as usize
            } else {
                0
            };

            if lines.len() <= max_lines {
                // Fits at this size, try larger
                low = mid;
                current_size = mid;
            } else {
                // Doesn't fit, try smaller
                high = mid;
            }

            if high - low < 0.5 {
                break;
            }
        }

        // Use the found size (or min_size if nothing fits)
        current_size = current_size.max(min_size);
        self.inner.current_font_size = current_size;

        let lines = self.wrap_text_to_width(text, width);
        let line_height = self.line_height();
        let ascender = self.ascender_height();
        let max_lines = if height >= ascender {
            ((height - ascender) / line_height + 1.0).floor() as usize
        } else {
            0
        };
        let total_lines = lines.len();
        let lines_to_render = lines.len().min(max_lines);

        // Prawn-style: point is relative to bounds (same as bounding_box)
        self.bounding_box(point, width, Some(height), |doc| {
            for line in lines.iter().take(lines_to_render) {
                doc.text(line);
            }
        });

        // Restore original font size
        self.inner.current_font_size = original_size;

        TextBoxResult {
            height,
            truncated: total_lines > lines_to_render,
            font_size: current_size,
            lines_rendered: lines_to_render,
            total_lines,
        }
    }

    /// Text box that expands to fit content
    fn text_box_expand(
        &mut self,
        text: &str,
        point: [f64; 2],
        width: f64,
        min_height: f64,
    ) -> TextBoxResult {
        let original_size = self.inner.current_font_size;
        let line_height = self.line_height();
        let ascender = self.ascender_height();
        let lines = self.wrap_text_to_width(text, width);
        let total_lines = lines.len();

        // Calculate required height: (n-1) * line_height + ascender
        // This matches Prawn's calculation where last line only needs ascender height
        let required_height = if total_lines > 0 {
            (total_lines - 1) as f64 * line_height + ascender
        } else {
            0.0
        };
        let actual_height = required_height.max(min_height);

        // Prawn-style: point is relative to bounds (same as bounding_box)
        self.bounding_box(point, width, Some(actual_height), |doc| {
            for line in &lines {
                doc.text(line);
            }
        });

        TextBoxResult {
            height: actual_height,
            truncated: false,
            font_size: original_size,
            lines_rendered: total_lines,
            total_lines,
        }
    }

    /// Measures the width of text with the current font
    fn measure_text_width(&self, text: &str) -> f64 {
        #[cfg(feature = "fonts")]
        {
            self.inner.measure_text(text)
        }
        #[cfg(not(feature = "fonts"))]
        {
            // Use proper AFM metrics for standard fonts (with kerning)
            use crate::font::kern_tables;
            use crate::font::StandardFont;
            if let Some(font) = StandardFont::from_name(&self.inner.current_font) {
                let raw_width = font.string_width(text) as f64;
                let kern_adj = kern_tables::total_kern_adjustment(&font, text) as f64;
                (raw_width + kern_adj) * self.inner.current_font_size / 1000.0
            } else {
                // Fallback for unknown fonts
                text.len() as f64 * self.inner.current_font_size * 0.5
            }
        }
    }

    /// Measures text width including character and word spacing
    ///
    /// This method considers fallback fonts when measuring width, ensuring
    /// accurate text wrapping for mixed-font content.
    fn measure_text_width_with_spacing(&self, text: &str) -> f64 {
        // Calculate base width considering fallback fonts
        let base_width = if self.state.fallback_fonts.is_empty() {
            self.measure_text_width(text)
        } else {
            let primary_font = self.inner.current_font.clone();
            let fallback_fonts = &self.state.fallback_fonts;
            let font_size = self.inner.current_font_size;
            let runs = self.analyze_text_for_fallback_with(text, &primary_font, fallback_fonts);
            runs.iter()
                .map(|run| self.measure_fragment_width_inner(&run.font, &run.text, font_size))
                .sum()
        };

        // Add character spacing (applied between characters)
        let char_count = text.chars().count();
        let char_spacing_width = if char_count > 0 {
            self.state.character_spacing * (char_count as f64 - 1.0)
        } else {
            0.0
        };

        // Add word spacing (applied to space characters only)
        let space_count = text.chars().filter(|&c| c == ' ').count();
        let word_spacing_width = self.state.word_spacing * space_count as f64;

        base_width + char_spacing_width + word_spacing_width
    }

    /// Measures text width for justify mode (only character spacing, no word spacing)
    ///
    /// In justify mode, word spacing is calculated dynamically to fill the line,
    /// so we should not include user-set word_spacing in wrap calculations.
    /// This method considers fallback fonts when measuring width.
    fn measure_text_width_for_justify(&self, text: &str) -> f64 {
        // Calculate base width considering fallback fonts
        let base_width = if self.state.fallback_fonts.is_empty() {
            self.measure_text_width(text)
        } else {
            let primary_font = self.inner.current_font.clone();
            let fallback_fonts = &self.state.fallback_fonts;
            let font_size = self.inner.current_font_size;
            let runs = self.analyze_text_for_fallback_with(text, &primary_font, fallback_fonts);
            runs.iter()
                .map(|run| self.measure_fragment_width_inner(&run.font, &run.text, font_size))
                .sum()
        };

        // Add character spacing (applied between characters)
        let char_count = text.chars().count();
        let char_spacing_width = if char_count > 0 {
            self.state.character_spacing * (char_count as f64 - 1.0)
        } else {
            0.0
        };

        base_width + char_spacing_width
    }

    /// Draws text with character and word spacing applied
    fn draw_text_with_spacing(&mut self, text: &str, pos: [f64; 2]) {
        let char_spacing = self.state.character_spacing;
        let word_spacing = self.state.word_spacing;

        // Support font fallback
        if self.state.fallback_fonts.is_empty() {
            self.inner
                .text_at_with_spacing(text, pos, char_spacing, word_spacing);
        } else {
            let font_size = self.inner.current_font_size;
            let primary_font = self.inner.current_font.clone();
            let runs = {
                let fallback_fonts = &self.state.fallback_fonts;
                self.analyze_text_for_fallback_with(text, &primary_font, fallback_fonts)
            };

            let mut x = pos[0];
            let y = pos[1];
            let mut is_first_run = true;
            for run in &runs {
                if run.text.is_empty() {
                    continue;
                }
                // Add character_spacing between runs (not before first run)
                if !is_first_run {
                    x += char_spacing;
                }
                is_first_run = false;

                self.inner.ensure_font(&run.font);
                self.inner.font(&run.font);
                self.inner
                    .text_at_with_spacing(&run.text, [x, y], char_spacing, word_spacing);

                let run_char_count = run.text.chars().count();
                let run_space_count = run.text.chars().filter(|&c| c == ' ').count();
                x += self.measure_fragment_width_inner(&run.font, &run.text, font_size)
                    + char_spacing * (run_char_count.saturating_sub(1)) as f64
                    + word_spacing * run_space_count as f64;
            }
            // Restore primary font
            self.inner.font(&primary_font);
        }
    }

    /// Draws text with justified alignment (word spacing calculated to fill width)
    ///
    /// This calculates the word spacing needed to make the text fill the available width,
    /// then draws the text with that spacing applied.
    fn draw_text_justified(&mut self, text: &str, y: f64, available_width: f64) {
        let bounds = self.state.bounds();
        let left = bounds.absolute_left();

        // Count spaces in text
        let space_count = text.chars().filter(|&c| c == ' ').count();

        if space_count == 0 {
            // No spaces - fall back to left alignment
            self.draw_text_with_spacing(text, [left, y]);
            return;
        }

        let font_size = self.inner.current_font_size;
        let char_spacing = self.state.character_spacing;
        let char_count = text.chars().count();

        // Calculate base width considering fallback fonts
        let base_width = if self.state.fallback_fonts.is_empty() {
            self.measure_text_width(text)
        } else {
            let primary_font = self.inner.current_font.clone();
            let runs = {
                let fallback_fonts = &self.state.fallback_fonts;
                self.analyze_text_for_fallback_with(text, &primary_font, fallback_fonts)
            };
            runs.iter()
                .map(|run| self.measure_fragment_width_inner(&run.font, &run.text, font_size))
                .sum()
        };

        let char_spacing_width = if char_count > 0 {
            char_spacing * (char_count as f64 - 1.0)
        } else {
            0.0
        };
        let width_with_char_spacing = base_width + char_spacing_width;

        // Calculate word spacing to fill the remaining space
        let extra_space = available_width - width_with_char_spacing;
        let justify_word_spacing = extra_space / space_count as f64;

        // Don't apply negative spacing (would compress text beyond natural width)
        // Use word_spacing=0 to match the justify wrap calculation
        let word_spacing = if justify_word_spacing < 0.0 {
            0.0
        } else {
            justify_word_spacing
        };

        // Draw text with fallback support
        if self.state.fallback_fonts.is_empty() {
            self.inner
                .text_at_with_spacing(text, [left, y], char_spacing, word_spacing);
        } else {
            let primary_font = self.inner.current_font.clone();
            let runs = {
                let fallback_fonts = &self.state.fallback_fonts;
                self.analyze_text_for_fallback_with(text, &primary_font, fallback_fonts)
            };

            let mut x = left;
            let mut is_first_run = true;
            for run in &runs {
                if run.text.is_empty() {
                    continue;
                }
                // Add character_spacing between runs (not before first run)
                if !is_first_run {
                    x += char_spacing;
                }
                is_first_run = false;

                self.inner.ensure_font(&run.font);
                self.inner.font(&run.font);
                self.inner
                    .text_at_with_spacing(&run.text, [x, y], char_spacing, word_spacing);

                // Advance x by text width + spacing
                let run_char_count = run.text.chars().count();
                let run_space_count = run.text.chars().filter(|&c| c == ' ').count();
                x += self.measure_fragment_width_inner(&run.font, &run.text, font_size)
                    + char_spacing * (run_char_count.saturating_sub(1)) as f64
                    + word_spacing * run_space_count as f64;
            }
            // Restore primary font
            self.inner.font(&primary_font);
        }
    }

    /// Wraps text to fit within the specified width
    fn wrap_text_to_width(&self, text: &str, max_width: f64) -> Vec<String> {
        self.wrap_text_to_width_with_flags(text, max_width)
            .into_iter()
            .map(|(line, _)| line)
            .collect()
    }

    /// Wraps text to fit within the specified width, returning paragraph-end flags
    ///
    /// Returns a vector of (line_text, is_paragraph_end) tuples.
    /// is_paragraph_end is true for the last line of each paragraph (should not be justified).
    fn wrap_text_to_width_with_flags(&self, text: &str, max_width: f64) -> Vec<(String, bool)> {
        let mut lines = Vec::new();

        for paragraph in text.split('\n') {
            if paragraph.is_empty() {
                lines.push((String::new(), true)); // Empty line is a paragraph end
                continue;
            }

            let words: Vec<&str> = paragraph.split_whitespace().collect();
            if words.is_empty() {
                lines.push((String::new(), true));
                continue;
            }

            let mut current_line = String::new();
            let mut paragraph_lines: Vec<String> = Vec::new();

            for word in words {
                let test_line = if current_line.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current_line, word)
                };

                let width = self.measure_text_width_with_spacing(&test_line);

                if width <= max_width {
                    current_line = test_line;
                } else {
                    // Line is too long
                    if !current_line.is_empty() {
                        paragraph_lines.push(current_line.clone());
                        current_line = word.to_string();
                    } else {
                        // Single word is too long, just add it anyway
                        paragraph_lines.push(word.to_string());
                    }
                }
            }

            if !current_line.is_empty() {
                paragraph_lines.push(current_line);
            }

            // Add all lines from this paragraph, marking the last one
            let para_len = paragraph_lines.len();
            for (i, line) in paragraph_lines.into_iter().enumerate() {
                let is_last = i == para_len - 1;
                lines.push((line, is_last));
            }
        }

        if lines.is_empty() {
            lines.push((String::new(), true));
        }

        lines
    }

    /// Wraps text for justify mode (without word_spacing in calculations)
    ///
    /// In justify mode, word spacing is calculated dynamically to fill each line,
    /// so we should not include user-set word_spacing in wrap calculations.
    fn wrap_text_for_justify(&self, text: &str, max_width: f64) -> Vec<(String, bool)> {
        let mut lines = Vec::new();

        for paragraph in text.split('\n') {
            if paragraph.is_empty() {
                lines.push((String::new(), true));
                continue;
            }

            let words: Vec<&str> = paragraph.split_whitespace().collect();
            if words.is_empty() {
                lines.push((String::new(), true));
                continue;
            }

            let mut current_line = String::new();
            let mut paragraph_lines: Vec<String> = Vec::new();

            for word in words {
                let test_line = if current_line.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current_line, word)
                };

                // Use justify-specific measurement (no word_spacing)
                let width = self.measure_text_width_for_justify(&test_line);

                if width <= max_width {
                    current_line = test_line;
                } else {
                    // Line is too long
                    if !current_line.is_empty() {
                        paragraph_lines.push(current_line.clone());
                        current_line = word.to_string();
                    } else {
                        // Single word is too long, just add it anyway
                        paragraph_lines.push(word.to_string());
                    }
                }
            }

            if !current_line.is_empty() {
                paragraph_lines.push(current_line);
            }

            // Add all lines from this paragraph, marking the last one
            let para_len = paragraph_lines.len();
            for (i, line) in paragraph_lines.into_iter().enumerate() {
                let is_last = i == para_len - 1;
                lines.push((line, is_last));
            }
        }

        if lines.is_empty() {
            lines.push((String::new(), true));
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
            fallback_fonts: self.state.fallback_fonts.clone(),
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

                    // Render with fallback support
                    if repeater.fallback_fonts.is_empty() {
                        self.inner.text_at(&repeater.text, repeater.position);
                    } else {
                        let runs = self.analyze_text_for_fallback_with(
                            &repeater.text,
                            &repeater.font,
                            &repeater.fallback_fonts,
                        );
                        let mut x = repeater.position[0];
                        let y = repeater.position[1];
                        for run in &runs {
                            if run.text.is_empty() {
                                continue;
                            }
                            self.inner.ensure_font(&run.font);
                            self.inner.font(&run.font);
                            self.inner.text_at(&run.text, [x, y]);
                            x += self.measure_fragment_width_inner(
                                &run.font,
                                &run.text,
                                repeater.font_size,
                            );
                        }
                        // Restore primary font
                        self.inner.font(&repeater.font);
                    }
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

                    // Page numbers are typically just digits, no fallback needed
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
    /// If the block creates new pages, float will return to the original page.
    /// This matches Prawn's float() behavior.
    pub fn float<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        let original_page = self.inner.page_number();
        let saved_cursor = self.state.cursor_y;
        let saved_font = self.inner.current_font.clone();
        let saved_font_size = self.inner.current_font_size;
        f(self);
        if self.inner.page_number() != original_page {
            self.inner.go_to_page(original_page - 1); // go_to_page uses 0-based index
        }
        self.state.cursor_y = saved_cursor;
        self.inner.current_font = saved_font;
        self.inner.current_font_size = saved_font_size;
        self
    }

    /// Temporarily indents content from left and/or right
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    pub fn indent<F>(&mut self, left: impl Measurement, right: impl Measurement, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        let left = left.to_pt();
        let right = right.to_pt();
        self.state.bounds_mut().add_left_padding(left);
        self.state.bounds_mut().add_right_padding(right);
        f(self);
        self.state.bounds_mut().subtract_left_padding(left);
        self.state.bounds_mut().subtract_right_padding(right);
        self
    }

    /// Adds vertical padding before and after the block
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    pub fn pad<F>(&mut self, amount: impl Measurement, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        let amount = amount.to_pt();
        self.move_down(amount);
        f(self);
        self.move_down(amount);
        self
    }

    /// Adds vertical padding before the block
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    pub fn pad_top<F>(&mut self, amount: impl Measurement, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        self.move_down(amount);
        f(self);
        self
    }

    /// Adds vertical padding after the block
    ///
    /// Accepts any measurement unit: `f64` (points), `Mm`, `Cm`, `Inch`, `Pt`
    pub fn pad_bottom<F>(&mut self, amount: impl Measurement, f: F) -> &mut Self
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
    /// Follows Prawn coordinate convention: point specifies the top-left corner
    /// position relative to the current bounding box, with Y increasing upward.
    ///
    /// # Arguments
    ///
    /// * `point` - Position of the box's top-left corner:
    ///   - `point[0]`: x offset from current bounds left edge
    ///   - `point[1]`: y position from current bounds bottom (Y increases upward)
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
    /// let cursor_y = layout.cursor();  // Get current position
    ///
    /// // Box at current cursor position
    /// layout.bounding_box([0.0, cursor_y], 200.0, Some(100.0), |doc| {
    ///     doc.text("Inside the box");
    ///     doc.stroke_bounds();
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
        let bounds = self.state.bounds();

        // Prawn-style: point is relative to current bounds
        // point[0] is x offset from bounds.left
        // point[1] is y position from bounds.bottom (Y increases upward)
        let abs_x = bounds.absolute_left() + point[0];
        let abs_y = bounds.absolute_bottom() + point[1];

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

        // Prawn behavior: if user didn't modify cursor, restore original position
        // (i.e., the bounding box was not used for flowing content)
        if (self.state.cursor_y - abs_y).abs() < 0.01 {
            self.state.cursor_y = old_cursor;
        } else if height.is_some() {
            // Fixed height: cursor moves to below the fixed-height box
            self.state.cursor_y = abs_y - height.unwrap();
        } else {
            // Stretchy: cursor is at the bottom of content
            self.state.cursor_y = abs_y - finished_bbox.height();
        }

        self
    }

    // === Column layout methods ===

    /// Creates a multi-column layout region
    ///
    /// Content rendered inside the closure flows within columns. When the
    /// cursor reaches the bottom of a column, it moves to the next column.
    /// When the last column overflows, a new page is created and columns
    /// restart from the first column.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, ColumnBoxOptions};
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    /// layout.column_box(ColumnBoxOptions::new(3).spacer(12.0), |doc| {
    ///     for i in 0..20 {
    ///         doc.text(&format!("Item {}", i + 1));
    ///     }
    /// });
    /// ```
    pub fn column_box<F>(&mut self, options: ColumnBoxOptions, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        let bounds = self.state.bounds();
        let total_width = bounds.width();
        let origin_x = bounds.absolute_left();
        let origin_y = self.state.cursor_y;
        let page_bottom = bounds.absolute_bottom();
        let bottom_y = options
            .height
            .map(|h| (origin_y - h).max(page_bottom))
            .unwrap_or(page_bottom);

        let spacer = options.spacer.unwrap_or(self.inner.current_font_size);
        let columns = options.columns.max(1);
        let column_width = (total_width - spacer * (columns as f64 - 1.0)) / columns as f64;

        // Set up column state
        self.state.column_state = Some(ColumnState {
            columns,
            spacer,
            current_column: 0,
            column_width,
            origin_x,
            origin_y,
            bottom_y,
            max_depth_y: origin_y,
        });

        // Push a bounding box for the first column
        let bbox = BoundingBox::new(origin_x, origin_y, column_width, Some(origin_y - bottom_y));
        self.state.bounds_stack.push(bbox);
        self.state.cursor_y = origin_y;

        // Execute user content
        f(self);

        // Record final cursor depth before cleanup
        if let Some(ref mut cs) = self.state.column_state {
            if self.state.cursor_y < cs.max_depth_y {
                cs.max_depth_y = self.state.cursor_y;
            }
        }
        let final_cursor = self
            .state
            .column_state
            .as_ref()
            .map(|c| c.max_depth_y)
            .unwrap_or(self.state.cursor_y);

        // Pop the column bounding box
        self.state.bounds_stack.pop();

        // Move cursor below the actual content depth
        self.state.cursor_y = final_cursor;

        // Clear column state
        self.state.column_state = None;

        self
    }

    /// Begin column layout (for FFI / Python bindings).
    ///
    /// Must be paired with [`column_box_end`]. Prefer [`column_box`] in Rust.
    pub fn column_box_begin(&mut self, options: ColumnBoxOptions) {
        let bounds = self.state.bounds();
        let total_width = bounds.width();
        let origin_x = bounds.absolute_left();
        let origin_y = self.state.cursor_y;
        let page_bottom = bounds.absolute_bottom();
        let bottom_y = options
            .height
            .map(|h| (origin_y - h).max(page_bottom))
            .unwrap_or(page_bottom);

        let spacer = options.spacer.unwrap_or(self.inner.current_font_size);
        let columns = options.columns.max(1);
        let column_width = (total_width - spacer * (columns as f64 - 1.0)) / columns as f64;

        self.state.column_state = Some(ColumnState {
            columns,
            spacer,
            current_column: 0,
            column_width,
            origin_x,
            origin_y,
            bottom_y,
            max_depth_y: origin_y,
        });

        let bbox = BoundingBox::new(origin_x, origin_y, column_width, Some(origin_y - bottom_y));
        self.state.bounds_stack.push(bbox);
        self.state.cursor_y = origin_y;
    }

    /// End column layout (for FFI / Python bindings).
    ///
    /// Must be paired with [`column_box_begin`]. Prefer [`column_box`] in Rust.
    pub fn column_box_end(&mut self) {
        // Record final cursor depth
        if let Some(ref mut cs) = self.state.column_state {
            if self.state.cursor_y < cs.max_depth_y {
                cs.max_depth_y = self.state.cursor_y;
            }
        }
        let final_cursor = self
            .state
            .column_state
            .as_ref()
            .map(|c| c.max_depth_y)
            .unwrap_or(self.state.cursor_y);

        self.state.bounds_stack.pop();
        self.state.cursor_y = final_cursor;
        self.state.column_state = None;
    }

    /// Checks if cursor has overflowed the current column and handles it
    ///
    /// Called internally after each text rendering operation when column_state
    /// is active. If cursor is below the column bottom:
    /// - Advances to next column if available
    /// - Creates a new page and restarts at first column if at last column
    fn check_column_overflow(&mut self) {
        let col = match self.state.column_state.as_ref() {
            Some(c) => c.clone(),
            None => return,
        };

        // Check if cursor has gone below the column bottom
        if self.state.cursor_y >= col.bottom_y {
            return; // no overflow
        }

        if col.current_column + 1 < col.columns {
            // Record depth before moving to next column
            if let Some(ref mut cs) = self.state.column_state {
                if self.state.cursor_y < cs.max_depth_y {
                    cs.max_depth_y = self.state.cursor_y;
                }
            }

            // Move to next column
            let next_col = col.current_column + 1;
            let next_x = col.origin_x + (col.column_width + col.spacer) * next_col as f64;

            // Pop old column bbox, push new one
            self.state.bounds_stack.pop();
            let bbox = BoundingBox::new(
                next_x,
                col.origin_y,
                col.column_width,
                Some(col.origin_y - col.bottom_y),
            );
            self.state.bounds_stack.push(bbox);

            // Reset cursor to top of column
            self.state.cursor_y = col.origin_y;

            // Update column index
            if let Some(ref mut cs) = self.state.column_state {
                cs.current_column = next_col;
            }
        } else {
            // Record depth before new page
            if let Some(ref mut cs) = self.state.column_state {
                if self.state.cursor_y < cs.max_depth_y {
                    cs.max_depth_y = self.state.cursor_y;
                }
            }

            // Last column overflowed — new page, restart at first column
            self.state.bounds_stack.pop();

            self.start_new_page();

            let (_, page_height) = self.inner.page_size.dimensions(self.inner.page_layout);
            let new_origin_y = page_height - self.state.margin.top;
            let new_bottom_y = self.state.margin.bottom;

            // Push bbox for first column on new page
            let bbox = BoundingBox::new(
                col.origin_x,
                new_origin_y,
                col.column_width,
                Some(new_origin_y - new_bottom_y),
            );
            self.state.bounds_stack.push(bbox);
            self.state.cursor_y = new_origin_y;

            // Reset column state for new page
            if let Some(ref mut cs) = self.state.column_state {
                cs.current_column = 0;
                cs.origin_y = new_origin_y;
                cs.bottom_y = new_bottom_y;
            }
        }
    }

    // === Grid methods ===

    /// Defines a grid system for the current page
    ///
    /// The grid divides the current bounds into rows and columns with optional
    /// gutters. Use `grid()` and `grid_span()` to access cells.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, GridOptions};
    ///
    /// let doc = Document::new();
    /// let mut layout = LayoutDocument::new(doc);
    ///
    /// // Create a 12-column, 10-row grid with 10pt gutters
    /// layout.define_grid(GridOptions::new(10, 12).gutter(10.0));
    ///
    /// // Access a single cell
    /// let cell = layout.grid(2, 3).unwrap();
    /// layout.bounding_box(cell.top_left(), cell.width, Some(cell.height), |doc| {
    ///     doc.text("Content");
    /// });
    /// ```
    pub fn define_grid(&mut self, options: GridOptions) -> &mut Self {
        let bounds = self.state.bounds();
        let width = bounds.width;
        let height = bounds.height();

        self.grid = Some(Grid::new(&options, width, height));
        self
    }

    /// Returns a reference to the current grid (if defined)
    pub fn current_grid(&self) -> Option<&Grid> {
        self.grid.as_ref()
    }

    /// Returns a GridBox for the specified row and column
    ///
    /// Row and column are 0-indexed. Returns None if no grid is defined
    /// or if the indices are out of bounds.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, GridOptions};
    ///
    /// let doc = Document::new();
    /// let mut layout = LayoutDocument::new(doc);
    /// layout.define_grid(GridOptions::new(10, 12).gutter(10.0));
    ///
    /// if let Some(cell) = layout.grid(2, 5) {
    ///     layout.bounding_box(cell.top_left(), cell.width, Some(cell.height), |doc| {
    ///         doc.text("Row 2, Column 5");
    ///     });
    /// }
    /// ```
    pub fn grid(&self, row: usize, column: usize) -> Option<GridBox> {
        self.grid.as_ref().and_then(|g| {
            if row < g.rows && column < g.columns {
                Some(g.cell(row, column))
            } else {
                None
            }
        })
    }

    /// Returns a MultiBox spanning from one cell to another
    ///
    /// The span includes both the start and end cells and everything in between.
    /// Returns None if no grid is defined or if indices are out of bounds.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, GridOptions};
    ///
    /// let doc = Document::new();
    /// let mut layout = LayoutDocument::new(doc);
    /// layout.define_grid(GridOptions::new(10, 12).gutter(10.0));
    ///
    /// // Span from (2,3) to (5,8)
    /// if let Some(span) = layout.grid_span((2, 3), (5, 8)) {
    ///     layout.bounding_box(span.top_left(), span.width, Some(span.height), |doc| {
    ///         doc.text("Spanning multiple cells");
    ///     });
    /// }
    /// ```
    pub fn grid_span(&self, start: (usize, usize), end: (usize, usize)) -> Option<MultiBox> {
        self.grid.as_ref().and_then(|g| {
            if start.0 < g.rows && start.1 < g.columns && end.0 < g.rows && end.1 < g.columns {
                Some(g.span(start, end))
            } else {
                None
            }
        })
    }

    /// Creates a bounding box at the specified grid cell position
    ///
    /// This method places content at the absolute grid cell position,
    /// regardless of current cursor position. The cursor is NOT moved
    /// after this operation.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, GridOptions};
    ///
    /// let doc = Document::new();
    /// let mut layout = LayoutDocument::new(doc);
    /// layout.define_grid(GridOptions::new(6, 4).gutter(10.0));
    ///
    /// layout.grid_bounding_box(0, 0, |doc| {
    ///     doc.text("Cell (0,0)");
    /// });
    /// ```
    pub fn grid_bounding_box<F>(&mut self, row: usize, column: usize, f: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        let (abs_x, abs_y, width, height) = {
            let grid = match &self.grid {
                Some(g) => g,
                None => return self,
            };

            if row >= grid.rows || column >= grid.columns {
                return self;
            }

            let bounds = self.state.bounds();
            let origin_x = bounds.absolute_left();
            let origin_y = bounds.absolute_top();

            let cell = grid.cell(row, column);
            let abs_x = origin_x + cell.left;
            let abs_y = origin_y - (grid.total_height - cell.top);

            (abs_x, abs_y, cell.width, cell.height)
        };

        // Create bounding box at absolute position
        let bbox = BoundingBox::new(abs_x, abs_y, width, Some(height));
        self.state.bounds_stack.push(bbox);

        // Save and set cursor to bbox top
        let old_cursor = self.state.cursor_y;
        self.state.cursor_y = abs_y;

        // Execute closure
        f(self);

        // Pop the bounding box
        self.state.bounds_stack.pop();

        // Restore cursor (grid boxes don't affect flow)
        self.state.cursor_y = old_cursor;

        self
    }

    /// Creates a bounding box spanning multiple grid cells
    ///
    /// This method places content spanning from start cell to end cell,
    /// regardless of current cursor position. The cursor is NOT moved
    /// after this operation.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, GridOptions};
    ///
    /// let doc = Document::new();
    /// let mut layout = LayoutDocument::new(doc);
    /// layout.define_grid(GridOptions::new(6, 4).gutter(10.0));
    ///
    /// // Span from (1,1) to (2,3)
    /// layout.grid_span_bounding_box((1, 1), (2, 3), |doc| {
    ///     doc.text("Spanning content");
    /// });
    /// ```
    pub fn grid_span_bounding_box<F>(
        &mut self,
        start: (usize, usize),
        end: (usize, usize),
        f: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        let (abs_x, abs_y, width, height) = {
            let grid = match &self.grid {
                Some(g) => g,
                None => return self,
            };

            if start.0 >= grid.rows
                || start.1 >= grid.columns
                || end.0 >= grid.rows
                || end.1 >= grid.columns
            {
                return self;
            }

            let bounds = self.state.bounds();
            let origin_x = bounds.absolute_left();
            let origin_y = bounds.absolute_top();

            let span = grid.span(start, end);
            let abs_x = origin_x + span.left;
            let abs_y = origin_y - (grid.total_height - span.top);

            (abs_x, abs_y, span.width, span.height)
        };

        // Create bounding box at absolute position
        let bbox = BoundingBox::new(abs_x, abs_y, width, Some(height));
        self.state.bounds_stack.push(bbox);

        // Save and set cursor to bbox top
        let old_cursor = self.state.cursor_y;
        self.state.cursor_y = abs_y;

        // Execute closure
        f(self);

        // Pop the bounding box
        self.state.bounds_stack.pop();

        // Restore cursor (grid boxes don't affect flow)
        self.state.cursor_y = old_cursor;

        self
    }

    /// Shows all grid cells with a stroke (diagnostic method)
    ///
    /// Draws the outline of each cell with the specified color.
    /// Useful for debugging layout.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdfcrate::api::{Document, LayoutDocument, GridOptions, Color};
    ///
    /// let doc = Document::new();
    /// let mut layout = LayoutDocument::new(doc);
    /// layout.define_grid(GridOptions::new(10, 12).gutter(10.0));
    /// layout.show_grid(Color::parse("#CCCCCC"));
    /// ```
    pub fn show_grid(&mut self, color: Color) -> &mut Self {
        let grid = match &self.grid {
            Some(g) => g.clone(),
            None => return self,
        };

        let bounds = self.state.bounds();
        let origin_x = bounds.absolute_left();
        let origin_y = bounds.absolute_top();

        for row in 0..grid.rows {
            for col in 0..grid.columns {
                let cell = grid.cell(row, col);
                let x = origin_x + cell.left;
                let y = origin_y - (grid.total_height - cell.top);

                // Draw cell outline
                self.inner.pages[self.inner.current_page]
                    .content
                    .save_state();
                self.inner.pages[self.inner.current_page]
                    .content
                    .set_stroke_color_rgb(color.r, color.g, color.b);
                self.inner.pages[self.inner.current_page]
                    .content
                    .rect(x, y - cell.height, cell.width, cell.height)
                    .stroke();

                // Draw cell name
                let font_size = 8.0;
                self.inner.ensure_font("Helvetica");
                self.inner.pages[self.inner.current_page]
                    .content
                    .set_fill_color_rgb(color.r, color.g, color.b);
                self.inner.pages[self.inner.current_page]
                    .content
                    .begin_text()
                    .set_font("Helvetica", font_size)
                    .move_text_pos(x + 2.0, y - font_size - 2.0)
                    .show_text(&cell.name())
                    .end_text();

                self.inner.pages[self.inner.current_page]
                    .content
                    .restore_state();
            }
        }

        self
    }

    /// Sets the cursor to a specific y position (relative to bounds.bottom, Prawn-style)
    ///
    /// This is an alias for `move_cursor_to`. The y value is measured from the
    /// bottom of the current bounds, with Y increasing upward (same as `cursor()`).
    pub fn set_cursor(&mut self, y: f64) -> &mut Self {
        let bottom = self.state.bounds().absolute_bottom();
        self.state.cursor_y = bottom + y;
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

    // === Link methods ===

    /// Sets the fill color for subsequent text and shape operations
    ///
    /// This follows Prawn's API where color is set persistently until changed.
    /// Accepts a hex color string (e.g., "ff0000") or a Color object.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdfcrate::prelude::*;
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    /// layout.fill_color("808080"); // Set gray color using hex
    /// layout.text("This text will be gray");
    ///
    /// layout.fill_color("ff0000"); // Change to red
    /// layout.text("This text will be red");
    /// ```
    pub fn fill_color(&mut self, color: impl Into<ColorInput>) -> &mut Self {
        let color: ColorInput = color.into();
        let c = color.to_color();
        self.inner.pages[self.inner.current_page]
            .content
            .set_fill_color_rgb(c.r, c.g, c.b);
        self
    }

    /// Sets the stroke color for subsequent line and shape operations
    ///
    /// This follows Prawn's API where color is set persistently until changed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdfcrate::prelude::*;
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    /// layout.stroke_color("0000ff"); // Set blue stroke color
    /// ```
    pub fn stroke_color(&mut self, color: impl Into<ColorInput>) -> &mut Self {
        let color: ColorInput = color.into();
        let c = color.to_color();
        self.inner.pages[self.inner.current_page]
            .content
            .set_stroke_color_rgb(c.r, c.g, c.b);
        self
    }

    /// Adds a link annotation to the current page
    ///
    /// Creates a clickable region that performs the specified action when clicked.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdfcrate::prelude::*;
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    /// layout.text("Click here for more info");
    ///
    /// // Add a URL link (coordinates in page space)
    /// layout.link_annotation(LinkAnnotation::url([72.0, 700.0, 200.0, 720.0], "https://example.com"));
    /// ```
    pub fn link_annotation(&mut self, annotation: super::link::LinkAnnotation) -> &mut Self {
        self.inner.link_annotation(annotation);
        self
    }

    /// Adds a URL link annotation
    ///
    /// Convenience method for adding a clickable URL link.
    ///
    /// # Arguments
    ///
    /// * `rect` - The clickable rectangle [x1, y1, x2, y2] in page coordinates
    /// * `url` - The URL to open when clicked
    ///
    /// # Example
    ///
    /// ```ignore
    /// layout.link_url([72.0, 700.0, 200.0, 720.0], "https://example.com");
    /// ```
    pub fn link_url(&mut self, rect: [f64; 4], url: impl Into<String>) -> &mut Self {
        self.inner.link_url(rect, url);
        self
    }

    /// Creates a text with an embedded link
    ///
    /// Draws text at the current cursor position and adds a clickable link over it.
    /// Returns `&mut Self` for method chaining.
    ///
    /// # Example
    ///
    /// ```ignore
    /// layout.text_link("Visit our website", "https://example.com");
    /// ```
    pub fn text_link(&mut self, text: &str, url: impl Into<String>) -> &mut Self {
        // Get text dimensions before drawing
        let text_width = self.measure_text_width_with_spacing(text);
        let line_height = self.line_height();

        // Get current position and calculate x based on alignment
        let bounds = self.state.bounds();
        let left = bounds.absolute_left();
        let right = bounds.absolute_right();
        let width = bounds.width();
        let y = self.state.cursor_y;

        // Calculate x position based on alignment (same logic as text())
        let x = match self.state.text_align {
            TextAlign::Left => left,
            TextAlign::Center => left + (width - text_width) / 2.0,
            TextAlign::Right => right - text_width,
            TextAlign::Justify => left, // Justify not supported for single-line text
        };

        // Draw the text
        self.text(text);

        // Add link annotation over the text area
        // rect is [x1, y1, x2, y2] where y1 < y2
        let rect = [
            x,
            y - line_height, // bottom of text
            x + text_width,
            y, // top of text (baseline + ascender approximation)
        ];
        self.inner.link_url(rect, url);

        self
    }

    /// Creates a text with a link to a named destination
    ///
    /// Draws text at the current cursor position and adds a clickable link
    /// that navigates to the named destination within the document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // First create a destination
    /// layout.add_dest_here("chapter1");
    ///
    /// // Later, create a link to that destination
    /// layout.text_link_dest("Go to Chapter 1", "chapter1");
    /// ```
    pub fn text_link_dest(&mut self, text: &str, dest_name: impl Into<String>) -> &mut Self {
        // Get text dimensions before drawing
        let text_width = self.measure_text_width_with_spacing(text);
        let line_height = self.line_height();

        // Get current position and calculate x based on alignment
        let bounds = self.state.bounds();
        let left = bounds.absolute_left();
        let right = bounds.absolute_right();
        let width = bounds.width();
        let y = self.state.cursor_y;

        // Calculate x position based on alignment
        let x = match self.state.text_align {
            TextAlign::Left => left,
            TextAlign::Center => left + (width - text_width) / 2.0,
            TextAlign::Right => right - text_width,
            TextAlign::Justify => left,
        };

        // Draw the text
        self.text(text);

        // Add link annotation to named destination
        let rect = [x, y - line_height, x + text_width, y];
        let annotation = super::link::LinkAnnotation::named(rect, dest_name);
        self.inner.link_annotation(annotation);

        self
    }

    /// Adds a named destination at a specific page and position
    ///
    /// Named destinations allow creating bookmarks and cross-references
    /// within the document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Add a destination at page 0 with FitH fit type
    /// layout.add_dest("intro", 0, DestinationFit::FitH(Some(700.0)));
    /// ```
    pub fn add_dest(
        &mut self,
        name: impl Into<String>,
        page_index: usize,
        fit: super::link::DestinationFit,
    ) -> &mut Self {
        self.inner.add_dest(name, page_index, fit);
        self
    }

    /// Adds a named destination at the current cursor position
    ///
    /// Creates a destination that will navigate to the current page
    /// at the current cursor y position.
    ///
    /// # Example
    ///
    /// ```ignore
    /// layout.text("Chapter 1: Introduction");
    /// layout.add_dest_here("chapter1");
    /// ```
    pub fn add_dest_here(&mut self, name: impl Into<String>) -> &mut Self {
        let page_index = self.inner.page_count().saturating_sub(1);
        let y = self.state.cursor_y;
        self.inner
            .add_dest(name, page_index, super::link::DestinationFit::FitH(Some(y)));
        self
    }

    // === Outline (Bookmarks) methods ===

    /// Defines the document outline (bookmarks) using a closure-based DSL
    ///
    /// The outline appears in the PDF viewer's navigation panel and allows
    /// users to quickly jump to different sections of the document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// layout.outline(|o| {
    ///     o.section("Chapter 1", 0, |o| {
    ///         o.page("Introduction", 0);
    ///         o.page("Getting Started", 1);
    ///     });
    ///     o.section("Chapter 2", 2, |o| {
    ///         o.page("Advanced Topics", 2);
    ///     });
    /// });
    /// ```
    pub fn outline<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut super::outline::OutlineBuilder),
    {
        self.inner.outline(f);
        self
    }

    /// Adds a single outline item at the root level
    pub fn add_outline_item(&mut self, item: super::outline::OutlineItem) -> &mut Self {
        self.inner.add_outline_item(item);
        self
    }

    /// Sets the entire document outline
    pub fn set_outline(&mut self, outline: super::outline::Outline) -> &mut Self {
        self.inner.set_outline(outline);
        self
    }

    /// Returns whether the document has an outline
    pub fn has_outline(&self) -> bool {
        self.inner.has_outline()
    }

    // === Table methods ===

    /// Creates and draws a table at the current cursor position
    ///
    /// The table is drawn immediately and the cursor is moved below the table.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdfcrate::prelude::*;
    ///
    /// let mut layout = LayoutDocument::new(Document::new());
    ///
    /// layout.table(
    ///     &[
    ///         &["Name", "Age", "City"],
    ///         &["Alice", "30", "New York"],
    ///         &["Bob", "25", "Los Angeles"],
    ///     ],
    ///     TableOptions::default(),
    /// );
    /// ```
    pub fn table<R, C>(&mut self, data: &[R], options: super::table::TableOptions) -> &mut Self
    where
        R: AsRef<[C]>,
        C: super::table::IntoCell + Clone,
    {
        let bounds = self.state.bounds();
        let available_width = bounds.width();
        let origin_x = bounds.absolute_left();
        let origin_y = self.state.cursor_y;
        let use_page_breaks = options.page_breaks;

        // Create and layout the table
        let mut table = super::table::Table::new(data, options, available_width);
        table.calculate_layout(self);

        // Draw the table (with or without page breaks)
        if use_page_breaks {
            table.draw_with_page_breaks(self, [origin_x, origin_y]);
        } else {
            table.draw(self, [origin_x, origin_y]);
            // Move cursor below the table
            self.state.cursor_y -= table.height();
        }

        self
    }

    /// Creates and draws a table with a configuration closure
    ///
    /// This allows customizing the table after creation but before drawing.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdfcrate::prelude::*;
    ///
    /// layout.table_with(
    ///     &[
    ///         &["Header 1", "Header 2"],
    ///         &["Cell 1", "Cell 2"],
    ///     ],
    ///     TableOptions::default(),
    ///     |table| {
    ///         table.row_background(0, Color::gray(0.9));
    ///     },
    /// );
    /// ```
    pub fn table_with<R, C, F>(
        &mut self,
        data: &[R],
        options: super::table::TableOptions,
        configure: F,
    ) -> &mut Self
    where
        R: AsRef<[C]>,
        C: super::table::IntoCell + Clone,
        F: FnOnce(&mut super::table::Table),
    {
        let bounds = self.state.bounds();
        let available_width = bounds.width();
        let origin_x = bounds.absolute_left();
        let origin_y = self.state.cursor_y;
        let use_page_breaks = options.page_breaks;

        // Create the table
        let mut table = super::table::Table::new(data, options, available_width);

        // Allow customization BEFORE layout calculation
        // This allows configure to modify padding/font/column widths etc.
        configure(&mut table);

        // Calculate layout after configuration
        table.calculate_layout(self);

        // Draw the table (with or without page breaks)
        if use_page_breaks {
            table.draw_with_page_breaks(self, [origin_x, origin_y]);
        } else {
            table.draw(self, [origin_x, origin_y]);
            // Move cursor below the table
            self.state.cursor_y -= table.height();
        }

        self
    }

    // === Python binding support methods ===

    /// Pushes indentation - for Python bindings
    ///
    /// This is the low-level method used by Python context managers.
    /// For normal Rust usage, prefer the closure-based `indent()` method.
    #[doc(hidden)]
    pub fn push_indent(&mut self, left: f64, right: f64) {
        self.state.bounds_mut().add_left_padding(left);
        self.state.bounds_mut().add_right_padding(right);
    }

    /// Pops indentation - for Python bindings
    ///
    /// This is the low-level method used by Python context managers.
    /// For normal Rust usage, prefer the closure-based `indent()` method.
    #[doc(hidden)]
    pub fn pop_indent(&mut self, left: f64, right: f64) {
        self.state.bounds_mut().subtract_left_padding(left);
        self.state.bounds_mut().subtract_right_padding(right);
    }

    /// Pushes a bounding box onto the stack - for Python bindings
    ///
    /// Returns the saved cursor position for later restoration.
    /// This is the low-level method used by Python context managers.
    /// For normal Rust usage, prefer the closure-based `bounding_box()` method.
    #[doc(hidden)]
    pub fn push_bounding_box(&mut self, point: [f64; 2], width: f64, height: Option<f64>) -> f64 {
        let bounds = self.state.bounds();

        // Prawn-style: point is relative to current bounds
        // point[0] is x offset from bounds.left
        // point[1] is y position from bounds.bottom (Y increases upward)
        let abs_x = bounds.absolute_left() + point[0];
        let abs_y = bounds.absolute_bottom() + point[1];

        let bbox = BoundingBox::new(abs_x, abs_y, width, height);

        // Push to stack
        self.state.bounds_stack.push(bbox);

        // Save cursor and set new cursor to bbox top
        let old_cursor = self.state.cursor_y;
        self.state.cursor_y = abs_y;

        old_cursor
    }

    /// Pops a bounding box from the stack - for Python bindings
    ///
    /// Returns the actual height of the bounding box.
    /// This is the low-level method used by Python context managers.
    /// For normal Rust usage, prefer the closure-based `bounding_box()` method.
    #[doc(hidden)]
    pub fn pop_bounding_box(&mut self, _old_cursor: f64, fixed_height: Option<f64>) -> f64 {
        // Update stretched height before popping
        let cursor_y = self.state.cursor_y;
        if let Some(bbox) = self.state.bounds_stack.last_mut() {
            bbox.update_stretched_height(cursor_y);
        }

        // Pop and get the finished bbox
        let finished_bbox = self.state.bounds_stack.pop().unwrap();
        let abs_y = finished_bbox.absolute_top();

        // Update parent cursor position
        if let Some(height) = fixed_height {
            self.state.cursor_y = abs_y - height;
        } else {
            self.state.cursor_y = abs_y - finished_bbox.height();
        }

        finished_bbox.height()
    }

    /// Gets the current cursor y position
    #[doc(hidden)]
    pub fn cursor_y(&self) -> f64 {
        self.state.cursor_y
    }

    /// Sets the cursor y position directly
    #[doc(hidden)]
    pub fn set_cursor_y(&mut self, y: f64) {
        self.state.cursor_y = y;
    }

    /// Pushes a bounding box at absolute coordinates - for Python grid support
    ///
    /// Returns the saved cursor position for later restoration.
    #[doc(hidden)]
    pub fn push_bounding_box_absolute(
        &mut self,
        abs_x: f64,
        abs_y: f64,
        width: f64,
        height: Option<f64>,
    ) -> f64 {
        let bbox = BoundingBox::new(abs_x, abs_y, width, height);

        // Push to stack
        self.state.bounds_stack.push(bbox);

        // Save cursor and set new cursor to bbox top
        let old_cursor = self.state.cursor_y;
        self.state.cursor_y = abs_y;

        old_cursor
    }
}

// Relative coordinate drawing (Prawn-style)

impl LayoutDocument {
    fn relative_origin(&self) -> (f64, f64) {
        let bounds = self.state.bounds();
        (bounds.absolute_left(), bounds.absolute_bottom())
    }

    /// Sets up a relative stroke context for drawing
    ///
    /// Coordinates are relative to the current bounding box origin (bottom-left),
    /// matching Prawn's behavior for graphics primitives.
    pub fn stroke_relative<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut RelativeStrokeContext),
    {
        let (origin_x, origin_y) = self.relative_origin();
        self.inner.stroke(|ctx| {
            let mut rel = RelativeStrokeContext::new(ctx, origin_x, origin_y);
            f(&mut rel);
        });
        self
    }

    /// Sets up a relative fill context for drawing
    ///
    /// Coordinates are relative to the current bounding box origin (bottom-left),
    /// matching Prawn's behavior for graphics primitives.
    pub fn fill_relative<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut RelativeFillContext),
    {
        let (origin_x, origin_y) = self.relative_origin();
        self.inner.fill(|ctx| {
            let mut rel = RelativeFillContext::new(ctx, origin_x, origin_y);
            f(&mut rel);
        });
        self
    }

    /// Sets up a relative fill-and-stroke context for drawing
    ///
    /// Coordinates are relative to the current bounding box origin (bottom-left),
    /// matching Prawn's behavior for graphics primitives.
    pub fn fill_and_stroke_relative<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut RelativeFillAndStrokeContext),
    {
        let (origin_x, origin_y) = self.relative_origin();
        self.inner.fill_and_stroke(|ctx| {
            let mut rel = RelativeFillAndStrokeContext::new(ctx, origin_x, origin_y);
            f(&mut rel);
        });
        self
    }
}

/// Relative stroke context
///
/// Provides Prawn-style relative coordinates within the current bounding box.
pub struct RelativeStrokeContext<'a, 'ctx> {
    inner: &'a mut StrokeContext<'ctx>,
    origin_x: f64,
    origin_y: f64,
}

impl<'a, 'ctx> RelativeStrokeContext<'a, 'ctx> {
    fn new(inner: &'a mut StrokeContext<'ctx>, origin_x: f64, origin_y: f64) -> Self {
        Self {
            inner,
            origin_x,
            origin_y,
        }
    }

    fn map_point(&self, point: [f64; 2]) -> [f64; 2] {
        [self.origin_x + point[0], self.origin_y + point[1]]
    }

    fn map_xy(&self, x: f64, y: f64) -> (f64, f64) {
        (self.origin_x + x, self.origin_y + y)
    }

    /// Sets line width
    pub fn line_width(&mut self, width: f64) -> &mut Self {
        self.inner.line_width(width);
        self
    }

    /// Sets stroke color from various input types
    pub fn color(&mut self, color: impl Into<ColorInput>) -> &mut Self {
        self.inner.color(color);
        self
    }

    /// Sets stroke color (grayscale)
    pub fn gray(&mut self, gray: f64) -> &mut Self {
        self.inner.gray(gray);
        self
    }

    /// Sets stroke color (CMYK)
    pub fn cmyk(&mut self, c: f64, m: f64, y: f64, k: f64) -> &mut Self {
        self.inner.cmyk(c, m, y, k);
        self
    }

    /// Sets dash pattern
    pub fn dash(&mut self, pattern: &[f64]) -> &mut Self {
        self.inner.dash(pattern);
        self
    }

    /// Sets dash pattern with phase
    pub fn dash_with_phase(&mut self, pattern: &[f64], phase: f64) -> &mut Self {
        self.inner.dash_with_phase(pattern, phase);
        self
    }

    /// Clears dash pattern (solid line)
    pub fn undash(&mut self) -> &mut Self {
        self.inner.undash();
        self
    }

    /// Sets line cap style
    pub fn cap(&mut self, cap: crate::content::LineCap) -> &mut Self {
        self.inner.cap(cap);
        self
    }

    /// Sets line join style
    pub fn join(&mut self, join: crate::content::LineJoin) -> &mut Self {
        self.inner.join(join);
        self
    }

    /// Adds a line to the current path
    pub fn line(&mut self, from: [f64; 2], to: [f64; 2]) -> &mut Self {
        let from = self.map_point(from);
        let to = self.map_point(to);
        self.inner.line(from, to);
        self
    }

    /// Adds a rectangle to the current path (Prawn-compatible)
    pub fn rectangle(&mut self, point: [f64; 2], width: f64, height: f64) -> &mut Self {
        let point = self.map_point(point);
        self.inner.rectangle(point, width, height);
        self
    }

    /// Adds a rectangle with bottom-left origin to the current path (PDF native)
    pub fn rectangle_bl(&mut self, origin: [f64; 2], width: f64, height: f64) -> &mut Self {
        let origin = self.map_point(origin);
        self.inner.rectangle_bl(origin, width, height);
        self
    }

    /// Adds a rounded rectangle to the current path (Prawn-compatible)
    pub fn rounded_rectangle(
        &mut self,
        point: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        let point = self.map_point(point);
        self.inner.rounded_rectangle(point, width, height, radius);
        self
    }

    /// Adds a rounded rectangle with bottom-left origin to the current path (PDF native)
    pub fn rounded_rectangle_bl(
        &mut self,
        origin: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        let origin = self.map_point(origin);
        self.inner
            .rounded_rectangle_bl(origin, width, height, radius);
        self
    }

    /// Adds a circle to the current path
    pub fn circle(&mut self, center: [f64; 2], radius: f64) -> &mut Self {
        let center = self.map_point(center);
        self.inner.circle(center, radius);
        self
    }

    /// Adds an ellipse to the current path
    pub fn ellipse(&mut self, center: [f64; 2], rx: f64, ry: f64) -> &mut Self {
        let center = self.map_point(center);
        self.inner.ellipse(center, rx, ry);
        self
    }

    /// Moves to a point (starts a new subpath)
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        let (x, y) = self.map_xy(x, y);
        self.inner.move_to(x, y);
        self
    }

    /// Adds a line to a point
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        let (x, y) = self.map_xy(x, y);
        self.inner.line_to(x, y);
        self
    }

    /// Adds a cubic Bezier curve
    pub fn curve_to(&mut self, cp1: [f64; 2], cp2: [f64; 2], end: [f64; 2]) -> &mut Self {
        let cp1 = self.map_point(cp1);
        let cp2 = self.map_point(cp2);
        let end = self.map_point(end);
        self.inner.curve_to(cp1, cp2, end);
        self
    }

    /// Closes the current subpath
    pub fn close_path(&mut self) -> &mut Self {
        self.inner.close_path();
        self
    }

    /// Strokes the current path immediately and resets path state
    pub fn stroke(&mut self) -> &mut Self {
        self.inner.stroke();
        self
    }

    /// Adds a polygon to the current path
    pub fn polygon(&mut self, points: &[[f64; 2]]) -> &mut Self {
        if points.is_empty() {
            return self;
        }
        let mapped: Vec<[f64; 2]> = points.iter().copied().map(|p| self.map_point(p)).collect();
        self.inner.polygon(&mapped);
        self
    }

    /// Accesses the underlying absolute coordinate context
    pub fn absolute(&mut self) -> &mut StrokeContext<'ctx> {
        self.inner
    }
}

/// Relative fill context
///
/// Provides Prawn-style relative coordinates within the current bounding box.
pub struct RelativeFillContext<'a, 'ctx> {
    inner: &'a mut FillContext<'ctx>,
    origin_x: f64,
    origin_y: f64,
}

impl<'a, 'ctx> RelativeFillContext<'a, 'ctx> {
    fn new(inner: &'a mut FillContext<'ctx>, origin_x: f64, origin_y: f64) -> Self {
        Self {
            inner,
            origin_x,
            origin_y,
        }
    }

    fn map_point(&self, point: [f64; 2]) -> [f64; 2] {
        [self.origin_x + point[0], self.origin_y + point[1]]
    }

    fn map_xy(&self, x: f64, y: f64) -> (f64, f64) {
        (self.origin_x + x, self.origin_y + y)
    }

    /// Sets fill color from various input types
    pub fn color(&mut self, color: impl Into<ColorInput>) -> &mut Self {
        self.inner.color(color);
        self
    }

    /// Sets fill color (grayscale)
    pub fn gray(&mut self, gray: f64) -> &mut Self {
        self.inner.gray(gray);
        self
    }

    /// Sets fill color (CMYK)
    pub fn cmyk(&mut self, c: f64, m: f64, y: f64, k: f64) -> &mut Self {
        self.inner.cmyk(c, m, y, k);
        self
    }

    /// Adds a rectangle to the current path (Prawn-compatible)
    pub fn rectangle(&mut self, point: [f64; 2], width: f64, height: f64) -> &mut Self {
        let point = self.map_point(point);
        self.inner.rectangle(point, width, height);
        self
    }

    /// Adds a rectangle with bottom-left origin to the current path (PDF native)
    pub fn rectangle_bl(&mut self, origin: [f64; 2], width: f64, height: f64) -> &mut Self {
        let origin = self.map_point(origin);
        self.inner.rectangle_bl(origin, width, height);
        self
    }

    /// Adds a rounded rectangle to the current path (Prawn-compatible)
    pub fn rounded_rectangle(
        &mut self,
        point: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        let point = self.map_point(point);
        self.inner.rounded_rectangle(point, width, height, radius);
        self
    }

    /// Adds a rounded rectangle with bottom-left origin to the current path (PDF native)
    pub fn rounded_rectangle_bl(
        &mut self,
        origin: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        let origin = self.map_point(origin);
        self.inner
            .rounded_rectangle_bl(origin, width, height, radius);
        self
    }

    /// Adds a circle to the current path
    pub fn circle(&mut self, center: [f64; 2], radius: f64) -> &mut Self {
        let center = self.map_point(center);
        self.inner.circle(center, radius);
        self
    }

    /// Adds an ellipse to the current path
    pub fn ellipse(&mut self, center: [f64; 2], rx: f64, ry: f64) -> &mut Self {
        let center = self.map_point(center);
        self.inner.ellipse(center, rx, ry);
        self
    }

    /// Moves to a point (starts a new subpath)
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        let (x, y) = self.map_xy(x, y);
        self.inner.move_to(x, y);
        self
    }

    /// Adds a line to a point
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        let (x, y) = self.map_xy(x, y);
        self.inner.line_to(x, y);
        self
    }

    /// Adds a cubic Bezier curve
    pub fn curve_to(&mut self, cp1: [f64; 2], cp2: [f64; 2], end: [f64; 2]) -> &mut Self {
        let cp1 = self.map_point(cp1);
        let cp2 = self.map_point(cp2);
        let end = self.map_point(end);
        self.inner.curve_to(cp1, cp2, end);
        self
    }

    /// Closes the current subpath
    pub fn close_path(&mut self) -> &mut Self {
        self.inner.close_path();
        self
    }

    /// Fills the current path immediately and resets path state
    pub fn fill(&mut self) -> &mut Self {
        self.inner.fill();
        self
    }

    /// Adds a polygon to the current path
    pub fn polygon(&mut self, points: &[[f64; 2]]) -> &mut Self {
        if points.is_empty() {
            return self;
        }
        let mapped: Vec<[f64; 2]> = points.iter().copied().map(|p| self.map_point(p)).collect();
        self.inner.polygon(&mapped);
        self
    }

    /// Accesses the underlying absolute coordinate context
    pub fn absolute(&mut self) -> &mut FillContext<'ctx> {
        self.inner
    }
}

/// Relative fill-and-stroke context
///
/// Provides Prawn-style relative coordinates within the current bounding box.
pub struct RelativeFillAndStrokeContext<'a, 'ctx> {
    inner: &'a mut FillAndStrokeContext<'ctx>,
    origin_x: f64,
    origin_y: f64,
}

impl<'a, 'ctx> RelativeFillAndStrokeContext<'a, 'ctx> {
    fn new(inner: &'a mut FillAndStrokeContext<'ctx>, origin_x: f64, origin_y: f64) -> Self {
        Self {
            inner,
            origin_x,
            origin_y,
        }
    }

    fn map_point(&self, point: [f64; 2]) -> [f64; 2] {
        [self.origin_x + point[0], self.origin_y + point[1]]
    }

    fn map_xy(&self, x: f64, y: f64) -> (f64, f64) {
        (self.origin_x + x, self.origin_y + y)
    }

    /// Sets fill color from various input types
    pub fn fill_color(&mut self, color: impl Into<ColorInput>) -> &mut Self {
        self.inner.fill_color(color);
        self
    }

    /// Sets fill color (grayscale)
    pub fn fill_gray(&mut self, gray: f64) -> &mut Self {
        self.inner.fill_gray(gray);
        self
    }

    /// Sets fill color (CMYK)
    pub fn fill_cmyk(&mut self, c: f64, m: f64, y: f64, k: f64) -> &mut Self {
        self.inner.fill_cmyk(c, m, y, k);
        self
    }

    /// Sets stroke color from various input types
    pub fn stroke_color(&mut self, color: impl Into<ColorInput>) -> &mut Self {
        self.inner.stroke_color(color);
        self
    }

    /// Sets stroke color (grayscale)
    pub fn stroke_gray(&mut self, gray: f64) -> &mut Self {
        self.inner.stroke_gray(gray);
        self
    }

    /// Sets stroke color (CMYK)
    pub fn stroke_cmyk(&mut self, c: f64, m: f64, y: f64, k: f64) -> &mut Self {
        self.inner.stroke_cmyk(c, m, y, k);
        self
    }

    /// Sets line width
    pub fn line_width(&mut self, width: f64) -> &mut Self {
        self.inner.line_width(width);
        self
    }

    /// Sets dash pattern
    pub fn dash(&mut self, pattern: &[f64]) -> &mut Self {
        self.inner.dash(pattern);
        self
    }

    /// Sets dash pattern with phase
    pub fn dash_with_phase(&mut self, pattern: &[f64], phase: f64) -> &mut Self {
        self.inner.dash_with_phase(pattern, phase);
        self
    }

    /// Clears dash pattern (solid line)
    pub fn undash(&mut self) -> &mut Self {
        self.inner.undash();
        self
    }

    /// Sets line cap style
    pub fn cap(&mut self, cap: crate::content::LineCap) -> &mut Self {
        self.inner.cap(cap);
        self
    }

    /// Sets line join style
    pub fn join(&mut self, join: crate::content::LineJoin) -> &mut Self {
        self.inner.join(join);
        self
    }

    /// Adds a rectangle to the current path (Prawn-compatible)
    pub fn rectangle(&mut self, point: [f64; 2], width: f64, height: f64) -> &mut Self {
        let point = self.map_point(point);
        self.inner.rectangle(point, width, height);
        self
    }

    /// Adds a rectangle with bottom-left origin to the current path (PDF native)
    pub fn rectangle_bl(&mut self, origin: [f64; 2], width: f64, height: f64) -> &mut Self {
        let origin = self.map_point(origin);
        self.inner.rectangle_bl(origin, width, height);
        self
    }

    /// Adds a rounded rectangle to the current path (Prawn-compatible)
    pub fn rounded_rectangle(
        &mut self,
        point: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        let point = self.map_point(point);
        self.inner.rounded_rectangle(point, width, height, radius);
        self
    }

    /// Adds a rounded rectangle with bottom-left origin to the current path (PDF native)
    pub fn rounded_rectangle_bl(
        &mut self,
        origin: [f64; 2],
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        let origin = self.map_point(origin);
        self.inner
            .rounded_rectangle_bl(origin, width, height, radius);
        self
    }

    /// Adds a circle to the current path
    pub fn circle(&mut self, center: [f64; 2], radius: f64) -> &mut Self {
        let center = self.map_point(center);
        self.inner.circle(center, radius);
        self
    }

    /// Adds an ellipse to the current path
    pub fn ellipse(&mut self, center: [f64; 2], rx: f64, ry: f64) -> &mut Self {
        let center = self.map_point(center);
        self.inner.ellipse(center, rx, ry);
        self
    }

    /// Moves to a point (starts a new subpath)
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        let (x, y) = self.map_xy(x, y);
        self.inner.move_to(x, y);
        self
    }

    /// Adds a line to a point
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        let (x, y) = self.map_xy(x, y);
        self.inner.line_to(x, y);
        self
    }

    /// Adds a cubic Bezier curve
    pub fn curve_to(&mut self, cp1: [f64; 2], cp2: [f64; 2], end: [f64; 2]) -> &mut Self {
        let cp1 = self.map_point(cp1);
        let cp2 = self.map_point(cp2);
        let end = self.map_point(end);
        self.inner.curve_to(cp1, cp2, end);
        self
    }

    /// Closes the current subpath
    pub fn close_path(&mut self) -> &mut Self {
        self.inner.close_path();
        self
    }

    /// Fills and strokes the current path immediately and resets path state
    pub fn fill_and_stroke(&mut self) -> &mut Self {
        self.inner.fill_and_stroke();
        self
    }

    /// Adds a polygon to the current path
    pub fn polygon(&mut self, points: &[[f64; 2]]) -> &mut Self {
        if points.is_empty() {
            return self;
        }
        let mapped: Vec<[f64; 2]> = points.iter().copied().map(|p| self.map_point(p)).collect();
        self.inner.polygon(&mapped);
        self
    }

    /// Accesses the underlying absolute coordinate context
    pub fn absolute(&mut self) -> &mut FillAndStrokeContext<'ctx> {
        self.inner
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

        // Prawn-style: cursor() returns position relative to bounds.bottom
        // A4 size: 595.28 x 841.89, margin 36pt
        // bounds_top = 805.89, bounds_bottom = 36, bounds_height = 769.89
        // Initial cursor_y (absolute) = 805.89
        // cursor() = cursor_y - bounds_bottom = 805.89 - 36 = 769.89
        let initial_cursor = layout.cursor();
        assert!(
            (initial_cursor - 769.89).abs() < 0.01,
            "expected 769.89, got {}",
            initial_cursor
        );

        // Move down: cursor_y -= 100, cursor() decreases
        layout.move_down(100.0);
        assert!((layout.cursor() - 669.89).abs() < 0.01);

        // Move up: cursor_y += 50, cursor() increases
        layout.move_up(50.0);
        assert!((layout.cursor() - 719.89).abs() < 0.01);

        // move_cursor_to(y): cursor_y = bounds_bottom + y
        // move_cursor_to(200) = cursor at 200pt from bottom
        layout.move_cursor_to(200.0);
        assert!(
            (layout.cursor() - 200.0).abs() < 0.01,
            "expected 200, got {}",
            layout.cursor()
        );
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
        let expected_line_height = layout.line_height();

        layout.text("Hello, World!");

        // Cursor should have moved down by line height
        // Using actual font metrics: Helvetica (718 - -207) / 1000 * 12 * 1.0 = 11.1pt
        assert!((layout.cursor() - (cursor_before - expected_line_height)).abs() < 0.01);
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

        // Prawn-style: point[1] is Y position from parent's bottom (same as cursor())
        // To create a box at cursor position, pass cursor() as point[1]
        layout.bounding_box([50.0, initial_cursor], 200.0, Some(150.0), |doc| {
            // Inside the box, bounds should reflect the new box
            let inner_bounds = doc.bounds();
            assert_eq!(inner_bounds.absolute_left(), 36.0 + 50.0); // margin + offset
            assert!((inner_bounds.width() - 200.0).abs() < 0.01);
            assert!((inner_bounds.height() - 150.0).abs() < 0.01);

            doc.text("Inside box");
        });

        // After the box, cursor should be below the box
        // Box top was at initial_cursor, box height is 150
        // New cursor = initial_cursor - 150
        let expected_cursor = initial_cursor - 150.0;
        assert!((layout.cursor() - expected_cursor).abs() < 0.01);
    }

    #[test]
    fn test_bounding_box_method_stretchy() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.font("Helvetica").size(12.0);
        let line_height = layout.line_height();
        let initial_cursor = layout.cursor();

        // Prawn-style: pass cursor() as point[1] to create box at cursor position
        layout.bounding_box([0.0, initial_cursor], 200.0, None, |doc| {
            doc.text("Line 1");
            doc.text("Line 2");
            doc.text("Line 3");
        });

        // The box should have stretched to fit 3 lines
        // Using actual font metrics: Helvetica height = 11.1pt per line
        let expected_height = line_height * 3.0;
        assert!((initial_cursor - layout.cursor() - expected_height).abs() < 0.1);
    }

    #[test]
    fn test_bounding_box_method_nested() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        let outer_left = layout.bounds().absolute_left();
        let outer_cursor = layout.cursor();

        // Prawn-style: pass cursor() as point[1] to create box at cursor position
        layout.bounding_box([20.0, outer_cursor], 300.0, Some(200.0), |doc| {
            let inner_bounds = doc.bounds();
            assert_eq!(inner_bounds.absolute_left(), outer_left + 20.0);

            let inner_cursor = doc.cursor();
            // Nested bounding box at inner cursor position
            doc.bounding_box([30.0, inner_cursor], 150.0, Some(100.0), |doc| {
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

    // === Link and Named Destinations Tests ===

    #[test]
    fn test_text_link() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.text_link("Click here", "https://example.com");

        let bytes = layout.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        assert!(
            pdf_str.contains("/Subtype /Link"),
            "Should have Link annotation"
        );
        assert!(
            pdf_str.contains("https://example.com"),
            "Should contain URL"
        );
    }

    #[test]
    fn test_text_link_dest() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // First add destination, then link to it
        layout.text("Chapter 1");
        layout.add_dest_here("chapter1");

        layout.move_down(100.0);
        layout.text_link_dest("Go to Chapter 1", "chapter1");

        let bytes = layout.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        assert!(
            pdf_str.contains("/Dest (chapter1)"),
            "Link should reference named destination"
        );
        assert!(pdf_str.contains("/Names"), "Should have Names dictionary");
    }

    #[test]
    fn test_add_dest_here_layout() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.text("Introduction");
        layout.add_dest_here("intro");

        assert_eq!(layout.inner.destinations.len(), 1);
        assert!(layout.inner.destinations.contains_key("intro"));

        // Verify the destination has the current page index
        let (page_index, _) = layout.inner.destinations.get("intro").unwrap();
        assert_eq!(*page_index, 0);
    }

    #[test]
    fn test_add_dest_multi_page() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.text("Page 1 content");
        layout.add_dest_here("page1_top");

        layout.start_new_page();
        layout.text("Page 2 content");
        layout.add_dest_here("page2_top");

        assert_eq!(layout.inner.destinations.len(), 2);

        let (page1_idx, _) = layout.inner.destinations.get("page1_top").unwrap();
        let (page2_idx, _) = layout.inner.destinations.get("page2_top").unwrap();

        assert_eq!(*page1_idx, 0);
        assert_eq!(*page2_idx, 1);
    }

    #[test]
    fn test_link_url_layout() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.text("Some text");
        layout.link_url([72.0, 700.0, 200.0, 720.0], "https://rust-lang.org");

        let bytes = layout.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        assert!(
            pdf_str.contains("https://rust-lang.org"),
            "Should contain URL"
        );
    }

    #[test]
    fn test_link_annotation_layout() {
        use crate::api::link::LinkAnnotation;

        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        let link = LinkAnnotation::page([72.0, 700.0, 200.0, 720.0], 0);
        layout.link_annotation(link);

        let bytes = layout.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        assert!(
            pdf_str.contains("/Subtype /Link"),
            "Should have Link annotation"
        );
    }

    #[test]
    fn test_add_dest_with_fit() {
        use crate::api::link::DestinationFit;

        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        layout.add_dest("fit_dest", 0, DestinationFit::Fit);
        layout.add_dest(
            "xyz_dest",
            0,
            DestinationFit::XYZ {
                left: Some(72.0),
                top: Some(700.0),
                zoom: Some(1.0),
            },
        );

        let bytes = layout.render().unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);

        assert!(pdf_str.contains("/Fit]"), "Should have Fit destination");
        assert!(pdf_str.contains("/XYZ"), "Should have XYZ destination");
    }

    // =========================================================================
    // Fallback fonts tests
    // =========================================================================

    #[test]
    fn test_fallback_fonts_global_setting() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Initially no fallback fonts
        assert!(layout.state.fallback_fonts.is_empty());

        // Set fallback fonts
        layout.fallback_fonts(vec!["Courier".to_string()]);
        assert_eq!(layout.state.fallback_fonts.len(), 1);
        assert_eq!(layout.state.fallback_fonts[0], "Courier");

        // Clear fallback fonts
        layout.fallback_fonts(vec![]);
        assert!(layout.state.fallback_fonts.is_empty());
    }

    #[test]
    fn test_text_wrap_with_fallback_renders() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Set fallback fonts
        layout.fallback_fonts(vec!["Courier".to_string()]);

        // text_wrap should work with fallback fonts configured
        layout.font("Helvetica").size(12.0);
        layout.text_wrap("Hello World - this text should wrap correctly.");

        let bytes = layout.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_text_with_fallback_renders() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Set fallback fonts
        layout.fallback_fonts(vec!["Courier".to_string()]);

        // text should work with fallback fonts configured
        layout.font("Helvetica").size(12.0);
        layout.text("Hello World");

        let bytes = layout.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_measure_text_width_with_spacing_no_fallback() {
        let doc = Document::new();
        let layout = LayoutDocument::new(doc);

        // Without fallback fonts, should use simple measurement
        let width = layout.measure_text_width_with_spacing("Hello");
        assert!(width > 0.0);
    }

    #[test]
    fn test_measure_text_width_with_spacing_with_fallback() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Set fallback fonts
        layout.fallback_fonts(vec!["Courier".to_string()]);

        // With fallback fonts, should still measure correctly
        let width = layout.measure_text_width_with_spacing("Hello");
        assert!(width > 0.0);
    }

    #[test]
    fn test_measure_text_width_for_justify_no_fallback() {
        let doc = Document::new();
        let layout = LayoutDocument::new(doc);

        // Without fallback fonts, should use simple measurement
        let width = layout.measure_text_width_for_justify("Hello World");
        assert!(width > 0.0);
    }

    #[test]
    fn test_measure_text_width_for_justify_with_fallback() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Set fallback fonts
        layout.fallback_fonts(vec!["Courier".to_string()]);

        // With fallback fonts, should still measure correctly
        let width = layout.measure_text_width_for_justify("Hello World");
        assert!(width > 0.0);
    }

    #[test]
    fn test_text_justify_with_fallback() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Set fallback fonts and justify alignment
        layout.fallback_fonts(vec!["Courier".to_string()]);
        layout.align(TextAlign::Justify);

        // text_wrap with justify should work with fallback fonts
        layout.font("Helvetica").size(12.0);
        layout.text_wrap("This is a longer paragraph of text that should be justified across the full width of the text area with proper word spacing calculated.");

        let bytes = layout.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_character_spacing_with_fallback() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Set fallback fonts and character spacing
        layout.fallback_fonts(vec!["Courier".to_string()]);
        layout.character_spacing(1.0);

        // text should work with both fallback and character spacing
        layout.font("Helvetica").size(12.0);
        layout.text("Hello World");

        let bytes = layout.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_text_align_center_with_fallback() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Set fallback fonts and center alignment
        layout.fallback_fonts(vec!["Courier".to_string()]);
        layout.align(TextAlign::Center);

        // text should work with both fallback and center alignment
        layout.font("Helvetica").size(12.0);
        layout.text("Centered text with fallback");

        let bytes = layout.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_text_align_right_with_fallback() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);

        // Set fallback fonts and right alignment
        layout.fallback_fonts(vec!["Courier".to_string()]);
        layout.align(TextAlign::Right);

        // text should work with both fallback and right alignment
        layout.font("Helvetica").size(12.0);
        layout.text("Right-aligned text with fallback");

        let bytes = layout.render().unwrap();
        assert!(bytes.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_winansi_char_detection() {
        // ASCII characters should be supported
        assert!(LayoutDocument::is_winansi_char('A'));
        assert!(LayoutDocument::is_winansi_char('z'));
        assert!(LayoutDocument::is_winansi_char('0'));
        assert!(LayoutDocument::is_winansi_char(' '));

        // Latin-1 Supplement characters should be supported
        assert!(LayoutDocument::is_winansi_char('é')); // U+00E9
        assert!(LayoutDocument::is_winansi_char('ñ')); // U+00F1
        assert!(LayoutDocument::is_winansi_char('ü')); // U+00FC
        assert!(LayoutDocument::is_winansi_char('©')); // U+00A9
        assert!(LayoutDocument::is_winansi_char('®')); // U+00AE

        // Special WinAnsi characters from higher Unicode ranges
        assert!(LayoutDocument::is_winansi_char('€')); // U+20AC Euro
        assert!(LayoutDocument::is_winansi_char('–')); // U+2013 En dash
        assert!(LayoutDocument::is_winansi_char('—')); // U+2014 Em dash
        assert!(LayoutDocument::is_winansi_char('\u{201C}')); // U+201C Left double quote "
        assert!(LayoutDocument::is_winansi_char('\u{201D}')); // U+201D Right double quote "
        assert!(LayoutDocument::is_winansi_char('\u{2018}')); // U+2018 Left single quote '
        assert!(LayoutDocument::is_winansi_char('\u{2019}')); // U+2019 Right single quote '
        assert!(LayoutDocument::is_winansi_char('…')); // U+2026 Ellipsis
        assert!(LayoutDocument::is_winansi_char('•')); // U+2022 Bullet
        assert!(LayoutDocument::is_winansi_char('™')); // U+2122 Trademark
        assert!(LayoutDocument::is_winansi_char('‰')); // U+2030 Per mille
                                                       // Other WinAnsi special characters
        assert!(LayoutDocument::is_winansi_char('\u{0152}')); // U+0152 Œ
        assert!(LayoutDocument::is_winansi_char('\u{0153}')); // U+0153 œ
        assert!(LayoutDocument::is_winansi_char('\u{0160}')); // U+0160 Š
        assert!(LayoutDocument::is_winansi_char('\u{0161}')); // U+0161 š

        // Characters NOT in WinAnsiEncoding should return false
        assert!(!LayoutDocument::is_winansi_char('中')); // Chinese
        assert!(!LayoutDocument::is_winansi_char('日')); // Japanese
        assert!(!LayoutDocument::is_winansi_char('한')); // Korean
        assert!(!LayoutDocument::is_winansi_char('α')); // Greek alpha (not in WinAnsi)
        assert!(!LayoutDocument::is_winansi_char('→')); // Arrow

        // C1 control characters (U+0080-U+009F) are NOT in WinAnsiEncoding
        // These are distinct from the WinAnsi byte positions 0x80-0x9F
        assert!(!LayoutDocument::is_winansi_char('\u{0080}')); // C1 control, not Euro
        assert!(!LayoutDocument::is_winansi_char('\u{0081}')); // C1 control
        assert!(!LayoutDocument::is_winansi_char('\u{0082}')); // C1 control, not ‚
        assert!(!LayoutDocument::is_winansi_char('\u{008D}')); // C1 control
        assert!(!LayoutDocument::is_winansi_char('\u{008F}')); // C1 control
        assert!(!LayoutDocument::is_winansi_char('\u{0090}')); // C1 control
        assert!(!LayoutDocument::is_winansi_char('\u{0091}')); // C1 control, not '
        assert!(!LayoutDocument::is_winansi_char('\u{0093}')); // C1 control, not "
        assert!(!LayoutDocument::is_winansi_char('\u{009D}')); // C1 control
        assert!(!LayoutDocument::is_winansi_char('\u{009F}')); // C1 control, not Ÿ
    }

    // ====================================================================
    // Inline format integration tests
    // ====================================================================

    #[test]
    fn test_text_inline_basic() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);
        let cursor_before = layout.cursor();
        layout.text_inline("Hello <b>world</b>!");
        let cursor_after = layout.cursor();
        // Cursor should move down (at least one line)
        assert!(cursor_after < cursor_before);
    }

    #[test]
    fn test_text_inline_empty() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);
        let cursor_before = layout.cursor();
        layout.text_inline("");
        let cursor_after = layout.cursor();
        // Empty input should not move cursor
        assert!((cursor_after - cursor_before).abs() < 0.01);
    }

    #[test]
    fn test_text_inline_plain_text() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);
        let cursor_before = layout.cursor();
        layout.text_inline("Just plain text");
        let cursor_after = layout.cursor();
        assert!(cursor_after < cursor_before);
    }

    #[test]
    fn test_text_inline_nested_tags() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);
        // Should not panic with nested tags
        layout.text_inline("<b>bold <i>bold-italic</i> bold</b>");
    }

    #[test]
    fn test_text_wrap_inline_basic() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);
        let cursor_before = layout.cursor();
        layout.text_wrap_inline(
            "This is a <b>long</b> paragraph with <i>mixed</i> styles that should wrap.",
        );
        let cursor_after = layout.cursor();
        assert!(cursor_after < cursor_before);
    }

    #[test]
    fn test_text_wrap_inline_empty() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);
        let cursor_before = layout.cursor();
        layout.text_wrap_inline("");
        let cursor_after = layout.cursor();
        assert!((cursor_after - cursor_before).abs() < 0.01);
    }

    #[test]
    fn test_text_wrap_inline_long_text_wraps() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);

        // Single unwrapped line
        let cursor_start = layout.cursor();
        layout.text("Short");
        let single_line_drop = cursor_start - layout.cursor();

        // Reset cursor for next test
        let cursor_before_wrap = layout.cursor();
        // Long text with inline formatting that should wrap to multiple lines
        layout.text_wrap_inline(
            "This is a <b>very long piece of text</b> that contains <i>many words</i> \
             and should definitely <u>wrap across multiple lines</u> when rendered \
             within the default page margins of the layout document.",
        );
        let multi_line_drop = cursor_before_wrap - layout.cursor();

        // Multi-line should drop more than single line
        assert!(multi_line_drop > single_line_drop * 1.5);
    }

    #[test]
    fn test_formatted_text_wrap_preserves_styles() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);

        // Should not panic; styles should carry through wrapping
        let fragments = vec![
            TextFragment::new("Normal text "),
            TextFragment::new("bold text ").bold(),
            TextFragment::new("italic text ").italic(),
            TextFragment::new("more normal text that keeps going and going to force wrapping"),
        ];
        layout.formatted_text_wrap(&fragments);
    }

    #[test]
    fn test_formatted_text_wrap_empty() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);
        let cursor_before = layout.cursor();
        layout.formatted_text_wrap(&[]);
        let cursor_after = layout.cursor();
        assert!((cursor_after - cursor_before).abs() < 0.01);
    }

    #[test]
    fn test_formatted_text_wrap_with_newlines() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);
        // Newlines in fragments should create paragraph breaks
        let fragments = vec![
            TextFragment::new("Line 1"),
            TextFragment::new("\n"),
            TextFragment::new("Line 2"),
        ];
        let cursor_before = layout.cursor();
        layout.formatted_text_wrap(&fragments);
        let cursor_after = layout.cursor();
        let drop = cursor_before - cursor_after;
        // Should be at least 2 lines worth of drop
        let line_height = layout.line_height();
        assert!(drop >= line_height * 1.5);
    }

    #[test]
    fn test_text_inline_renders_pdf() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(12.0);
        layout.text_inline("Hello <b>bold</b> and <i>italic</i>");
        layout.text_wrap_inline("Wrapped <u>underline</u> text that goes on for a while.");
        let bytes = layout.into_inner().render().unwrap();
        // Should produce valid PDF bytes
        assert!(bytes.len() > 100);
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_column_box_overflow() {
        // Column box should overflow text to the next column
        let doc = Document::new();
        let mut layout = LayoutDocument::with_margin(doc, Margin::new(72.0, 72.0, 72.0, 72.0));
        layout.font("Helvetica").size(10.0);

        let cursor_before = layout.cursor();
        layout.column_box(ColumnBoxOptions::new(3), |col| {
            // Write enough lines to overflow at least one column
            for i in 0..100 {
                col.text(&format!("Line {}", i));
            }
        });
        let cursor_after = layout.cursor();

        // After column_box, cursor should have moved down
        assert!(cursor_after < cursor_before);

        // Verify the PDF renders without errors
        let bytes = layout.into_inner().render().unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_column_box_text_wrap_overflow() {
        // text_wrap inside column_box should flow across columns
        let doc = Document::new();
        let mut layout = LayoutDocument::with_margin(doc, Margin::new(72.0, 72.0, 72.0, 72.0));
        layout.font("Helvetica").size(10.0);

        let lorem = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                     Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ";
        let long_text = lorem.repeat(20);

        layout.column_box(ColumnBoxOptions::new(3), |col| {
            col.text_wrap(&long_text);
        });

        // Should produce a multi-page or valid PDF
        let bytes = layout.into_inner().render().unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_standard_font_height_uses_afm_metrics() {
        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.inner_mut().font("Helvetica").size(12.0);

        // Helvetica AFM: ascender=718, descender=-207, line_gap=231
        let fh = layout.font_height();
        let expected = (718.0 + 207.0 + 231.0) * 12.0 / 1000.0; // 13.872
        assert!(
            (fh - expected).abs() < 0.01,
            "Helvetica font_height: got {fh}, expected {expected}"
        );

        let ah = layout.ascender_height();
        let expected_asc = 718.0 * 12.0 / 1000.0; // 8.616
        assert!(
            (ah - expected_asc).abs() < 0.01,
            "Helvetica ascender_height: got {ah}, expected {expected_asc}"
        );
    }

    #[cfg(feature = "fonts")]
    #[test]
    fn test_embedded_font_height_uses_ttf_metrics() {
        let mut doc = Document::new();
        let roboto = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("fonts")
            .join("Roboto-Regular.ttf");
        if !roboto.exists() {
            eprintln!("Skipping: examples/fonts/Roboto-Regular.ttf not found");
            return;
        }
        let ps_name = doc.embed_font_file(roboto.to_str().unwrap()).unwrap();
        doc.font(&ps_name).size(12.0);

        let layout = LayoutDocument::new(doc);

        let fh = layout.font_height();
        // Embedded font should NOT fall back to font_size * 1.15
        let fallback = 12.0 * 1.15;
        assert!(
            (fh - fallback).abs() > 0.01,
            "font_height should use TTF metrics, not fallback {fallback}; got {fh}"
        );
        assert!(fh > 0.0, "font_height must be positive");

        let ah = layout.ascender_height();
        assert!(ah > 0.0, "ascender_height must be positive");
        assert!(
            ah < fh,
            "ascender_height ({ah}) should be less than font_height ({fh})"
        );
    }

    #[test]
    fn test_formatted_text_mixed_fonts_cursor_advancement() {
        use crate::api::{FontStyle, TextFragment};

        let doc = Document::new();
        let mut layout = LayoutDocument::new(doc);
        layout.font("Helvetica").size(10.0);

        // Record cursor before formatted_text
        let cursor_before = layout.cursor();

        // Mixed: Helvetica (normal) + Helvetica-Bold
        // Helvetica-Bold has line_gap=265 vs Helvetica's 231,
        // so font_height differs: 1190/1000*10=11.90 vs 1156/1000*10=11.56
        layout.formatted_text(&[
            TextFragment::new("Normal "),
            TextFragment::new("Bold").style(FontStyle::Bold),
            TextFragment::new(" text"),
        ]);

        let cursor_after = layout.cursor();
        let moved = cursor_before - cursor_after;

        // Should use Helvetica-Bold's font_height (11.90), not Helvetica's (11.56)
        let expected = 1190.0 / 1000.0 * 10.0; // 11.90
        assert!(
            (moved - expected).abs() < 0.01,
            "formatted_text should advance by max font_height across fragments; \
             expected {expected:.2}, got {moved:.2}"
        );
    }
}
