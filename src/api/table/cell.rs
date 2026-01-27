//! Table cell types and styling

use crate::api::layout::TextAlign;
use crate::api::Color;

/// Vertical alignment options for cell content
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum VerticalAlign {
    #[default]
    Top,
    Center,
    Bottom,
}

/// Border line styles
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BorderLine {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

/// Cell styling configuration
#[derive(Debug, Clone)]
pub struct CellStyle {
    /// Padding [top, right, bottom, left] in points
    pub padding: [f64; 4],
    /// Which borders to draw [top, right, bottom, left]
    pub borders: [bool; 4],
    /// Border widths [top, right, bottom, left]
    pub border_widths: [f64; 4],
    /// Border colors [top, right, bottom, left]
    pub border_colors: [Color; 4],
    /// Border line styles [top, right, bottom, left]
    pub border_lines: [BorderLine; 4],
    /// Background color (None = transparent)
    pub background_color: Option<Color>,
    /// Text color
    pub text_color: Color,
    /// Horizontal text alignment
    pub align: TextAlign,
    /// Vertical alignment
    pub valign: VerticalAlign,
    /// Font name override
    pub font: Option<String>,
    /// Font size override
    pub font_size: Option<f64>,
    /// Text overflow behavior
    pub overflow: TextOverflow,
    /// Minimum width constraint (for auto column width)
    pub min_width: Option<f64>,
    /// Maximum width constraint (for auto column width)
    pub max_width: Option<f64>,
    /// Force single line (no wrapping)
    pub single_line: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            padding: [5.0, 5.0, 5.0, 5.0],
            borders: [true, true, true, true],
            border_widths: [0.5, 0.5, 0.5, 0.5],
            border_colors: [Color::BLACK, Color::BLACK, Color::BLACK, Color::BLACK],
            border_lines: [BorderLine::Solid; 4],
            background_color: None,
            text_color: Color::BLACK,
            align: TextAlign::Left,
            valign: VerticalAlign::Top,
            font: None,
            font_size: None,
            overflow: TextOverflow::Truncate,
            min_width: None,
            max_width: None,
            single_line: false,
        }
    }
}

impl CellStyle {
    /// Create a new cell style with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set uniform padding on all sides
    pub fn padding(mut self, value: f64) -> Self {
        self.padding = [value, value, value, value];
        self
    }

    /// Set padding with [vertical, horizontal] values
    pub fn padding_vh(mut self, vertical: f64, horizontal: f64) -> Self {
        self.padding = [vertical, horizontal, vertical, horizontal];
        self
    }

    /// Set padding with [top, right, bottom, left] values
    pub fn padding_trbl(mut self, top: f64, right: f64, bottom: f64, left: f64) -> Self {
        self.padding = [top, right, bottom, left];
        self
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Set text color
    pub fn color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set text alignment
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set vertical alignment
    pub fn valign(mut self, valign: VerticalAlign) -> Self {
        self.valign = valign;
        self
    }

    /// Set uniform border width on all sides
    pub fn border_width(mut self, width: f64) -> Self {
        self.border_widths = [width, width, width, width];
        self
    }

    /// Set uniform border color on all sides
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_colors = [color, color, color, color];
        self
    }

    /// Disable all borders
    pub fn no_borders(mut self) -> Self {
        self.borders = [false, false, false, false];
        self
    }

    /// Enable all borders
    pub fn all_borders(mut self) -> Self {
        self.borders = [true, true, true, true];
        self
    }

    /// Enable/disable top border
    pub fn top_border(mut self, enabled: bool) -> Self {
        self.borders[0] = enabled;
        self
    }

    /// Enable/disable right border
    pub fn right_border(mut self, enabled: bool) -> Self {
        self.borders[1] = enabled;
        self
    }

    /// Enable/disable bottom border
    pub fn bottom_border(mut self, enabled: bool) -> Self {
        self.borders[2] = enabled;
        self
    }

    /// Enable/disable left border
    pub fn left_border(mut self, enabled: bool) -> Self {
        self.borders[3] = enabled;
        self
    }

    /// Set border line style for all sides
    pub fn border_line(mut self, line: BorderLine) -> Self {
        self.border_lines = [line, line, line, line];
        self
    }

    /// Set border line styles per side [top, right, bottom, left]
    pub fn border_lines_trbl(
        mut self,
        top: BorderLine,
        right: BorderLine,
        bottom: BorderLine,
        left: BorderLine,
    ) -> Self {
        self.border_lines = [top, right, bottom, left];
        self
    }

    /// Set border widths per side [top, right, bottom, left]
    pub fn border_widths_trbl(mut self, top: f64, right: f64, bottom: f64, left: f64) -> Self {
        self.border_widths = [top, right, bottom, left];
        self
    }

    /// Set font name
    pub fn font(mut self, name: impl Into<String>) -> Self {
        self.font = Some(name.into());
        self
    }

    /// Set font size
    pub fn font_size(mut self, size: f64) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Get total horizontal padding (left + right)
    pub fn horizontal_padding(&self) -> f64 {
        self.padding[1] + self.padding[3]
    }

    /// Get total vertical padding (top + bottom)
    pub fn vertical_padding(&self) -> f64 {
        self.padding[0] + self.padding[2]
    }

    /// Set text overflow behavior
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Set minimum width constraint
    pub fn min_width(mut self, width: f64) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Set maximum width constraint
    pub fn max_width(mut self, width: f64) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Force single line (no wrapping)
    pub fn single_line(mut self, single: bool) -> Self {
        self.single_line = single;
        self
    }
}

/// Text overflow behavior
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TextOverflow {
    /// Truncate text that doesn't fit (default)
    #[default]
    Truncate,
    /// Shrink font to fit content, with minimum font size
    ShrinkToFit(f64),
    /// Expand cell height to fit all content
    Expand,
}

/// Image fit mode for image cells
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ImageFit {
    /// Scale image to fit within cell, maintaining aspect ratio
    #[default]
    Contain,
    /// Scale image to cover cell, maintaining aspect ratio (may crop)
    Cover,
    /// Stretch image to fill cell exactly (may distort)
    Fill,
    /// Use original image size (may overflow cell)
    None,
}

/// Image cell content
#[derive(Debug, Clone)]
pub struct ImageContent {
    /// Image data (bytes)
    pub data: Vec<u8>,
    /// Original width (if known)
    pub width: Option<f64>,
    /// Original height (if known)
    pub height: Option<f64>,
    /// How to fit the image in the cell
    pub fit: ImageFit,
    /// Scale factor (1.0 = original size)
    pub scale: f64,
}

impl ImageContent {
    /// Create new image content from bytes
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            width: None,
            height: None,
            fit: ImageFit::Contain,
            scale: 1.0,
        }
    }

    /// Set fit mode
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    /// Set scale
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    /// Set explicit dimensions
    pub fn dimensions(mut self, width: f64, height: f64) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }
}

/// Content type for table cells
#[derive(Debug, Clone, Default)]
pub enum CellContent {
    /// Text content
    Text(String),
    /// Image content
    Image(ImageContent),
    /// Subtable (nested table) - stores the raw data for later rendering
    Subtable(SubtableData),
    /// Empty cell
    #[default]
    Empty,
}

/// Data for a subtable cell
#[derive(Debug, Clone)]
pub struct SubtableData {
    /// Row data as strings (simplified for now)
    pub rows: Vec<Vec<String>>,
    /// Column widths (optional)
    pub column_widths: Option<Vec<f64>>,
}

impl From<&str> for CellContent {
    fn from(s: &str) -> Self {
        CellContent::Text(s.to_string())
    }
}

impl From<String> for CellContent {
    fn from(s: String) -> Self {
        CellContent::Text(s)
    }
}

/// A single table cell
#[derive(Debug, Clone)]
pub struct Cell {
    /// Cell content
    pub content: CellContent,
    /// Row index (0-based)
    pub row: usize,
    /// Column index (0-based)
    pub column: usize,
    /// Number of columns to span
    pub colspan: usize,
    /// Number of rows to span
    pub rowspan: usize,
    /// Cell styling
    pub style: CellStyle,
    // Calculated layout values (set during table layout)
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl Cell {
    /// Create a new text cell
    pub fn new(content: impl Into<CellContent>) -> Self {
        Self {
            content: content.into(),
            row: 0,
            column: 0,
            colspan: 1,
            rowspan: 1,
            style: CellStyle::default(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    /// Create an empty cell
    pub fn empty() -> Self {
        Self::new(CellContent::Empty)
    }

    /// Create an image cell from bytes
    pub fn image(data: Vec<u8>) -> Self {
        Self {
            content: CellContent::Image(ImageContent::new(data)),
            row: 0,
            column: 0,
            colspan: 1,
            rowspan: 1,
            style: CellStyle::default(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    /// Create an image cell with custom options
    pub fn image_with(content: ImageContent) -> Self {
        Self {
            content: CellContent::Image(content),
            row: 0,
            column: 0,
            colspan: 1,
            rowspan: 1,
            style: CellStyle::default(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    /// Create a subtable cell
    pub fn subtable(rows: Vec<Vec<String>>) -> Self {
        Self {
            content: CellContent::Subtable(SubtableData {
                rows,
                column_widths: None,
            }),
            row: 0,
            column: 0,
            colspan: 1,
            rowspan: 1,
            style: CellStyle::default().padding(0.0), // No padding by default for subtables
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    /// Create a subtable cell with column widths
    pub fn subtable_with_widths(rows: Vec<Vec<String>>, column_widths: Vec<f64>) -> Self {
        Self {
            content: CellContent::Subtable(SubtableData {
                rows,
                column_widths: Some(column_widths),
            }),
            row: 0,
            column: 0,
            colspan: 1,
            rowspan: 1,
            style: CellStyle::default().padding(0.0),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    /// Set colspan
    pub fn colspan(mut self, span: usize) -> Self {
        self.colspan = span.max(1);
        self
    }

    /// Set rowspan
    pub fn rowspan(mut self, span: usize) -> Self {
        self.rowspan = span.max(1);
        self
    }

    /// Set cell style
    pub fn style(mut self, style: CellStyle) -> Self {
        self.style = style;
        self
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.style.background_color = Some(color);
        self
    }

    /// Set text color
    pub fn color(mut self, color: Color) -> Self {
        self.style.text_color = color;
        self
    }

    /// Set text alignment
    pub fn align(mut self, align: TextAlign) -> Self {
        self.style.align = align;
        self
    }

    /// Get content width (excluding padding)
    pub fn content_width(&self) -> f64 {
        (self.width - self.style.horizontal_padding()).max(0.0)
    }

    /// Get content height (excluding padding)
    pub fn content_height(&self) -> f64 {
        (self.height - self.style.vertical_padding()).max(0.0)
    }

    /// Get content area top-left position (accounting for padding)
    pub fn content_origin(&self) -> [f64; 2] {
        [
            self.x + self.style.padding[3], // left padding
            self.y - self.style.padding[0], // top padding (y decreases downward)
        ]
    }

    /// Check if this cell spans multiple columns or rows
    pub fn is_spanning(&self) -> bool {
        self.colspan > 1 || self.rowspan > 1
    }

    /// Get text content if this is a text cell
    pub fn text(&self) -> Option<&str> {
        match &self.content {
            CellContent::Text(s) => Some(s),
            CellContent::Empty | CellContent::Image(_) | CellContent::Subtable(_) => None,
        }
    }
}

/// Trait for types that can be converted to a Cell
pub trait IntoCell {
    fn into_cell(self) -> Cell;
}

impl IntoCell for Cell {
    fn into_cell(self) -> Cell {
        self
    }
}

impl IntoCell for &str {
    fn into_cell(self) -> Cell {
        Cell::new(self)
    }
}

impl IntoCell for String {
    fn into_cell(self) -> Cell {
        Cell::new(self)
    }
}

impl IntoCell for &String {
    fn into_cell(self) -> Cell {
        Cell::new(self.clone())
    }
}

// Implement for numeric types
macro_rules! impl_into_cell_for_numeric {
    ($($t:ty),*) => {
        $(
            impl IntoCell for $t {
                fn into_cell(self) -> Cell {
                    Cell::new(self.to_string())
                }
            }
        )*
    };
}

impl_into_cell_for_numeric!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

/// A selection of cells for batch styling operations
///
/// This provides a prawn-table-like API for styling multiple cells at once:
/// ```ignore
/// table.rows(0..2).background_color(Color::gray(0.8));
/// table.columns(1..3).align(TextAlign::Right);
/// table.rows(0).columns(0..2).style(CellStyle::default().no_borders());
/// ```
pub struct CellSelection<'a> {
    /// Indices of selected cells in the parent table's cells vector
    indices: Vec<usize>,
    /// Reference to the parent table's cells
    cells: &'a mut [Cell],
}

impl<'a> CellSelection<'a> {
    /// Create a new cell selection
    pub fn new(cells: &'a mut [Cell], indices: Vec<usize>) -> Self {
        Self { indices, cells }
    }

    /// Get the maximum row index in the selection (for negative index resolution)
    fn max_row(&self) -> usize {
        self.cells.iter().map(|c| c.row + 1).max().unwrap_or(0)
    }

    /// Get the maximum column index in the selection (for negative index resolution)
    fn max_column(&self) -> usize {
        self.cells.iter().map(|c| c.column + 1).max().unwrap_or(0)
    }

    /// Filter to only cells in the given rows
    ///
    /// Supports negative indices: -1 = last row, -2 = second-to-last, etc.
    pub fn rows(self, rows: impl RangeBoundsExt) -> Self {
        let max_row = self.max_row();
        let rows_range = rows.to_range(max_row);
        let indices = self
            .indices
            .into_iter()
            .filter(|&i| rows_range.contains(&self.cells[i].row))
            .collect();
        Self {
            indices,
            cells: self.cells,
        }
    }

    /// Filter to only cells in the given columns
    ///
    /// Supports negative indices: -1 = last column, -2 = second-to-last, etc.
    pub fn columns(self, cols: impl RangeBoundsExt) -> Self {
        let max_col = self.max_column();
        let cols_range = cols.to_range(max_col);
        let indices = self
            .indices
            .into_iter()
            .filter(|&i| cols_range.contains(&self.cells[i].column))
            .collect();
        Self {
            indices,
            cells: self.cells,
        }
    }

    /// Apply a style to all selected cells
    pub fn style(self, style: CellStyle) -> Self {
        for &i in &self.indices {
            self.cells[i].style = style.clone();
        }
        self
    }

    /// Set background color for all selected cells
    pub fn background_color(self, color: Color) -> Self {
        for &i in &self.indices {
            self.cells[i].style.background_color = Some(color);
        }
        self
    }

    /// Set text color for all selected cells
    pub fn text_color(self, color: Color) -> Self {
        for &i in &self.indices {
            self.cells[i].style.text_color = color;
        }
        self
    }

    /// Set horizontal alignment for all selected cells
    pub fn align(self, align: TextAlign) -> Self {
        for &i in &self.indices {
            self.cells[i].style.align = align;
        }
        self
    }

    /// Set vertical alignment for all selected cells
    pub fn valign(self, valign: VerticalAlign) -> Self {
        for &i in &self.indices {
            self.cells[i].style.valign = valign;
        }
        self
    }

    /// Set font for all selected cells
    pub fn font(self, font: &str, size: Option<f64>) -> Self {
        for &i in &self.indices {
            self.cells[i].style.font = Some(font.to_string());
            if let Some(s) = size {
                self.cells[i].style.font_size = Some(s);
            }
        }
        self
    }

    /// Set padding for all selected cells
    pub fn padding(self, padding: f64) -> Self {
        for &i in &self.indices {
            self.cells[i].style.padding = [padding; 4];
        }
        self
    }

    /// Remove borders from all selected cells
    pub fn no_borders(self) -> Self {
        for &i in &self.indices {
            self.cells[i].style.borders = [false; 4];
        }
        self
    }

    /// Set border width for all selected cells
    pub fn border_width(self, width: f64) -> Self {
        for &i in &self.indices {
            self.cells[i].style.border_widths = [width; 4];
        }
        self
    }

    /// Set border color for all selected cells
    pub fn border_color(self, color: Color) -> Self {
        for &i in &self.indices {
            self.cells[i].style.border_colors = [color; 4];
        }
        self
    }

    /// Apply a function to each selected cell
    pub fn each<F>(self, mut f: F) -> Self
    where
        F: FnMut(&mut Cell),
    {
        for &i in &self.indices {
            f(&mut self.cells[i]);
        }
        self
    }
}

/// Helper trait for range bounds
///
/// Supports both positive indices (0, 1, 2, ...) and negative indices (-1, -2, ...).
/// Negative indices count from the end: -1 is the last element, -2 is second-to-last, etc.
///
/// # Examples
/// ```ignore
/// // Using positive indices
/// table.select_rows(0);       // First row
/// table.select_rows(0..3);    // First three rows
///
/// // Using negative indices
/// table.select_rows(-1);      // Last row
/// table.select_rows(-2);      // Second-to-last row
/// table.select_rows(-3..-1);  // Third-to-last to second-to-last (exclusive)
/// ```
pub trait RangeBoundsExt {
    fn to_range(self, max: usize) -> std::ops::Range<usize>;
}

/// Convert a potentially negative index to a positive index
///
/// Negative indices count from the end: -1 = last, -2 = second-to-last, etc.
fn resolve_index(idx: isize, max: usize) -> usize {
    if idx >= 0 {
        (idx as usize).min(max)
    } else {
        // Negative index: -1 = max-1, -2 = max-2, etc.
        let positive = max as isize + idx;
        if positive < 0 {
            0
        } else {
            positive as usize
        }
    }
}

impl RangeBoundsExt for usize {
    fn to_range(self, _max: usize) -> std::ops::Range<usize> {
        self..self + 1
    }
}

// Support for signed integers (negative indices)
impl RangeBoundsExt for isize {
    fn to_range(self, max: usize) -> std::ops::Range<usize> {
        let idx = resolve_index(self, max);
        idx..idx + 1
    }
}

impl RangeBoundsExt for i32 {
    fn to_range(self, max: usize) -> std::ops::Range<usize> {
        let idx = resolve_index(self as isize, max);
        idx..idx + 1
    }
}

impl RangeBoundsExt for std::ops::Range<usize> {
    fn to_range(self, _max: usize) -> std::ops::Range<usize> {
        self
    }
}

impl RangeBoundsExt for std::ops::Range<isize> {
    fn to_range(self, max: usize) -> std::ops::Range<usize> {
        let start = resolve_index(self.start, max);
        let end = resolve_index(self.end, max);
        start..end
    }
}

impl RangeBoundsExt for std::ops::Range<i32> {
    fn to_range(self, max: usize) -> std::ops::Range<usize> {
        let start = resolve_index(self.start as isize, max);
        let end = resolve_index(self.end as isize, max);
        start..end
    }
}

impl RangeBoundsExt for std::ops::RangeFrom<usize> {
    fn to_range(self, max: usize) -> std::ops::Range<usize> {
        self.start..max
    }
}

impl RangeBoundsExt for std::ops::RangeFrom<isize> {
    fn to_range(self, max: usize) -> std::ops::Range<usize> {
        let start = resolve_index(self.start, max);
        start..max
    }
}

impl RangeBoundsExt for std::ops::RangeTo<usize> {
    fn to_range(self, _max: usize) -> std::ops::Range<usize> {
        0..self.end
    }
}

impl RangeBoundsExt for std::ops::RangeTo<isize> {
    fn to_range(self, max: usize) -> std::ops::Range<usize> {
        let end = resolve_index(self.end, max);
        0..end
    }
}

impl RangeBoundsExt for std::ops::RangeFull {
    fn to_range(self, max: usize) -> std::ops::Range<usize> {
        0..max
    }
}

impl RangeBoundsExt for std::ops::RangeInclusive<usize> {
    fn to_range(self, _max: usize) -> std::ops::Range<usize> {
        *self.start()..*self.end() + 1
    }
}

impl RangeBoundsExt for std::ops::RangeInclusive<isize> {
    fn to_range(self, max: usize) -> std::ops::Range<usize> {
        let start = resolve_index(*self.start(), max);
        let end = resolve_index(*self.end(), max);
        start..end + 1
    }
}
