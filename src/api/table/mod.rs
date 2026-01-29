//! Table layout and rendering
//!
//! Provides table functionality similar to prawn-table for Ruby's Prawn PDF library.
//!
//! # Example
//!
//! ```ignore
//! use pdfcrate::prelude::*;
//!
//! let mut layout = LayoutDocument::new(Document::new());
//!
//! layout.table(
//!     &[
//!         &["Name", "Age", "City"],
//!         &["Alice", "30", "New York"],
//!         &["Bob", "25", "Los Angeles"],
//!     ],
//!     TableOptions::default(),
//! );
//! ```

mod cell;

pub use cell::{
    BorderLine, Cell, CellContent, CellSelection, CellStyle, ImageContent, ImageFit, IntoCell,
    RangeBoundsExt, SubtableData, TextOverflow, VerticalAlign,
};

use crate::api::layout::{LayoutDocument, TextAlign};
use crate::api::measurements::Measurement;
use crate::api::Color;

/// Table positioning options
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TablePosition {
    /// Align table to the left of the bounding box
    #[default]
    Left,
    /// Center table horizontally
    Center,
    /// Align table to the right
    Right,
}

/// Column width specification
#[derive(Debug, Clone, Default)]
pub enum ColumnWidths {
    /// Divide width equally among columns
    #[default]
    Equal,
    /// Explicit widths for each column
    Fixed(Vec<f64>),
    /// Auto-calculate based on content (Phase 2)
    Auto,
}

/// Options for table creation
#[derive(Debug, Clone)]
pub struct TableOptions {
    /// Total table width (None = use bounding box width)
    pub width: Option<f64>,
    /// Table position within bounding box
    pub position: TablePosition,
    /// Number of header rows to repeat on page breaks
    pub header: usize,
    /// Alternating row background colors
    pub row_colors: Option<Vec<Color>>,
    /// Default cell style
    pub cell_style: CellStyle,
    /// Column width specification
    pub column_widths: ColumnWidths,
    /// Enable automatic page breaks for long tables
    pub page_breaks: bool,
}

impl Default for TableOptions {
    fn default() -> Self {
        Self {
            width: None,
            position: TablePosition::Left,
            header: 0,
            row_colors: None,
            cell_style: CellStyle::default(),
            column_widths: ColumnWidths::Equal,
            page_breaks: false,
        }
    }
}

impl TableOptions {
    /// Create new table options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set table width
    pub fn width(mut self, width: impl Measurement) -> Self {
        self.width = Some(width.to_pt());
        self
    }

    /// Set table position
    pub fn position(mut self, position: TablePosition) -> Self {
        self.position = position;
        self
    }

    /// Set number of header rows
    pub fn header(mut self, rows: usize) -> Self {
        self.header = rows;
        self
    }

    /// Set alternating row colors
    pub fn row_colors(mut self, colors: Vec<Color>) -> Self {
        self.row_colors = Some(colors);
        self
    }

    /// Set default cell style
    pub fn cell_style(mut self, style: CellStyle) -> Self {
        self.cell_style = style;
        self
    }

    /// Set fixed column widths
    pub fn column_widths(mut self, widths: &[f64]) -> Self {
        self.column_widths = ColumnWidths::Fixed(widths.to_vec());
        self
    }

    /// Enable automatic page breaks for long tables
    ///
    /// When enabled, the table will automatically break across pages
    /// and repeat header rows (if configured) on each new page.
    pub fn page_breaks(mut self, enable: bool) -> Self {
        self.page_breaks = enable;
        self
    }
}

/// Table structure for layout and rendering
pub struct Table {
    /// All cells in the table
    cells: Vec<Cell>,
    /// Number of rows
    row_count: usize,
    /// Number of columns
    column_count: usize,
    /// Calculated column widths
    column_widths: Vec<f64>,
    /// Calculated row heights
    row_heights: Vec<f64>,
    /// Table options
    options: TableOptions,
    /// Total table width
    width: f64,
    /// Total table height (calculated)
    height: f64,
}

impl Table {
    /// Create a new table from 2D data
    ///
    /// Handles colspan/rowspan by tracking occupied positions.
    pub fn new<R, C>(data: &[R], options: TableOptions, available_width: f64) -> Self
    where
        R: AsRef<[C]>,
        C: IntoCell + Clone,
    {
        use std::collections::HashSet;

        // Track occupied positions (row, col) due to rowspan from previous rows
        let mut occupied: HashSet<(usize, usize)> = HashSet::new();

        let mut cells = Vec::new();
        let mut max_row = 0usize;
        let mut max_col = 0usize;

        for (row_idx, row) in data.iter().enumerate() {
            let mut col_idx = 0usize;

            for cell_data in row.as_ref().iter() {
                // Skip occupied positions
                while occupied.contains(&(row_idx, col_idx)) {
                    col_idx += 1;
                }

                let mut cell = cell_data.clone().into_cell();
                cell.row = row_idx;
                cell.column = col_idx;

                // Apply default style from options (merge with existing cell style)
                let default_style = &options.cell_style;

                // Only override if cell doesn't have a specific value
                if cell.style.font.is_none() {
                    cell.style.font = default_style.font.clone();
                }
                if cell.style.font_size.is_none() {
                    cell.style.font_size = default_style.font_size;
                }
                // Apply default padding (check if still at default CellStyle values)
                if cell.style.padding == [5.0, 5.0, 5.0, 5.0] {
                    cell.style.padding = default_style.padding;
                }
                // Apply default border settings if at default
                if cell.style.border_widths == [0.5, 0.5, 0.5, 0.5] {
                    cell.style.border_widths = default_style.border_widths;
                }
                if cell.style.borders == [true, true, true, true] {
                    cell.style.borders = default_style.borders;
                }
                if cell.style.border_colors
                    == [Color::BLACK, Color::BLACK, Color::BLACK, Color::BLACK]
                {
                    cell.style.border_colors = default_style.border_colors;
                }
                if cell.style.border_lines == [BorderLine::Solid; 4] {
                    cell.style.border_lines = default_style.border_lines;
                }
                // Apply other defaults
                if cell.style.background_color.is_none() {
                    cell.style.background_color = default_style.background_color;
                }
                if cell.style.text_color == Color::BLACK {
                    cell.style.text_color = default_style.text_color;
                }
                if cell.style.align == TextAlign::Left {
                    cell.style.align = default_style.align;
                }
                if cell.style.valign == VerticalAlign::Top {
                    cell.style.valign = default_style.valign;
                }

                // Mark positions occupied by this cell's span
                for r in 0..cell.rowspan {
                    for c in 0..cell.colspan {
                        if r > 0 || c > 0 {
                            // Don't mark the cell's own position, only spanned positions
                            occupied.insert((row_idx + r, col_idx + c));
                        }
                    }
                }

                // Track max dimensions
                max_row = max_row.max(row_idx + cell.rowspan);
                max_col = max_col.max(col_idx + cell.colspan);

                cells.push(cell);
                col_idx += 1;
            }
        }

        let row_count = max_row.max(data.len());
        let column_count = max_col;

        // Determine table width
        let width = options.width.unwrap_or(available_width);

        // Calculate column widths (Auto is deferred to calculate_layout)
        let column_widths = match &options.column_widths {
            ColumnWidths::Auto => vec![0.0; column_count], // Placeholder, calculated later
            _ => Self::calculate_column_widths(column_count, width, &options.column_widths),
        };

        // Create table (row heights calculated later with font info)
        Table {
            cells,
            row_count,
            column_count,
            column_widths,
            row_heights: vec![0.0; row_count],
            options,
            width,
            height: 0.0,
        }
    }

    /// Calculate column widths based on specification
    fn calculate_column_widths(
        column_count: usize,
        table_width: f64,
        spec: &ColumnWidths,
    ) -> Vec<f64> {
        match spec {
            ColumnWidths::Equal => {
                if column_count == 0 {
                    vec![]
                } else {
                    let width = table_width / column_count as f64;
                    vec![width; column_count]
                }
            }
            ColumnWidths::Fixed(widths) => {
                let mut result = widths.clone();
                // Extend with equal distribution if not enough widths specified
                if result.len() < column_count {
                    let remaining_width: f64 = table_width - result.iter().sum::<f64>();
                    let remaining_cols = column_count - result.len();
                    let width_per_col = if remaining_cols > 0 {
                        remaining_width / remaining_cols as f64
                    } else {
                        0.0
                    };
                    result.resize(column_count, width_per_col.max(0.0));
                }
                result
            }
            ColumnWidths::Auto => {
                // Phase 2: auto-calculate based on content
                // For now, fall back to equal
                if column_count == 0 {
                    vec![]
                } else {
                    let width = table_width / column_count as f64;
                    vec![width; column_count]
                }
            }
        }
    }

    /// Calculate row heights based on content and font metrics
    pub fn calculate_layout(&mut self, doc: &mut LayoutDocument) {
        // Calculate auto column widths if needed
        if matches!(self.options.column_widths, ColumnWidths::Auto) {
            self.column_widths = self.calculate_auto_column_widths(doc);
        }

        // First pass: calculate row heights for cells with rowspan=1
        for row in 0..self.row_count {
            let mut max_height = 0.0f64;

            // Collect minimal cell info to avoid borrow issues (no ImageContent cloning)
            let cell_info: Vec<_> = self
                .cells
                .iter()
                .filter(|c| c.row == row && c.rowspan == 1)
                .map(|c| {
                    let col_width: f64 = (c.column..c.column + c.colspan)
                        .filter_map(|col| self.column_widths.get(col))
                        .sum();
                    (
                        col_width,
                        c.style.horizontal_padding(),
                        c.style.vertical_padding(),
                        c.style.font.clone(),
                        c.style.font_size,
                        // Extract only what's needed from content
                        match &c.content {
                            CellContent::Text(t) => (Some(t.clone()), None, None),
                            CellContent::Image(img) => (None, img.height, None),
                            CellContent::Subtable(sub) => (None, None, Some(sub.rows.len())),
                            CellContent::Empty => (None, None, None),
                        },
                    )
                })
                .collect();

            for (col_width, h_pad, v_pad, font, font_size, content_info) in cell_info {
                let cell_height = Self::calculate_content_height(
                    doc,
                    col_width,
                    h_pad,
                    v_pad,
                    font.as_deref(),
                    font_size,
                    content_info,
                );
                max_height = max_height.max(cell_height);
            }

            // Ensure minimum height
            self.row_heights[row] = max_height.max(20.0);
        }

        // Second pass: handle rowspan cells - distribute their height across spanned rows
        let rowspan_info: Vec<_> = self
            .cells
            .iter()
            .filter(|c| c.rowspan > 1)
            .map(|c| {
                let col_width: f64 = (c.column..c.column + c.colspan)
                    .filter_map(|col| self.column_widths.get(col))
                    .sum();
                (
                    c.row,
                    c.rowspan,
                    col_width,
                    c.style.horizontal_padding(),
                    c.style.vertical_padding(),
                    c.style.font.clone(),
                    c.style.font_size,
                    match &c.content {
                        CellContent::Text(t) => (Some(t.clone()), None, None),
                        CellContent::Image(img) => (None, img.height, None),
                        CellContent::Subtable(sub) => (None, None, Some(sub.rows.len())),
                        CellContent::Empty => (None, None, None),
                    },
                )
            })
            .collect();

        for (row, rowspan, col_width, h_pad, v_pad, font, font_size, content_info) in rowspan_info {
            let cell_height = Self::calculate_content_height(
                doc,
                col_width,
                h_pad,
                v_pad,
                font.as_deref(),
                font_size,
                content_info,
            );

            // Sum of current row heights for spanned rows
            let current_sum: f64 = (row..row + rowspan)
                .filter_map(|r| self.row_heights.get(r))
                .sum();

            // If cell needs more height, distribute extra across spanned rows
            if cell_height > current_sum {
                let extra_per_row = (cell_height - current_sum) / rowspan as f64;
                for r in row..row + rowspan {
                    if r < self.row_heights.len() {
                        self.row_heights[r] += extra_per_row;
                    }
                }
            }
        }

        // Calculate total height
        self.height = self.row_heights.iter().sum();

        // Position all cells
        self.position_cells();
    }

    /// Calculate natural column widths based on cell content
    fn calculate_auto_column_widths(&mut self, doc: &mut LayoutDocument) -> Vec<f64> {
        if self.column_count == 0 {
            return vec![];
        }

        // Calculate natural width for each column (max of all cells in column)
        let mut natural_widths = vec![0.0f64; self.column_count];
        let mut min_widths = vec![20.0f64; self.column_count]; // Default min
        let mut max_widths = vec![f64::MAX; self.column_count]; // Default max

        // Collect cell info first to avoid borrow issues
        // Only collect what we need to avoid cloning ImageContent data
        let cell_info: Vec<_> = self
            .cells
            .iter()
            .map(|c| {
                (
                    c.column,
                    c.colspan,
                    c.style.min_width,
                    c.style.max_width,
                    c.style.horizontal_padding(),
                    c.style.font.clone(),
                    c.style.font_size,
                    match &c.content {
                        CellContent::Text(t) => Some(t.clone()),
                        _ => None,
                    },
                    match &c.content {
                        CellContent::Image(img) => img.width,
                        _ => None,
                    },
                    match &c.content {
                        CellContent::Subtable(sub) => sub.column_widths.clone(),
                        _ => None,
                    },
                )
            })
            .collect();

        for (
            column,
            colspan,
            cell_min,
            cell_max,
            h_padding,
            font,
            font_size,
            text,
            img_width,
            sub_widths,
        ) in cell_info
        {
            // Calculate content width based on type
            let content_width = if let Some(ref text) = text {
                if text.is_empty() {
                    0.0
                } else {
                    // Apply cell's font settings for accurate measurement
                    let old_font = doc.current_font().to_string();
                    let old_size = doc.current_font_size();

                    if let Some(ref f) = font {
                        doc.inner_mut().font(f).size(font_size.unwrap_or(old_size));
                    } else if let Some(size) = font_size {
                        doc.inner_mut().font(&old_font).size(size);
                    }

                    let width = doc.measure_text(text);

                    doc.inner_mut().font(&old_font).size(old_size);
                    width
                }
            } else if let Some(w) = img_width {
                w
            } else if let Some(ref widths) = sub_widths {
                widths.iter().sum()
            } else {
                0.0
            };

            let cell_natural_width = content_width + h_padding;

            // For colspan cells, distribute width across spanned columns
            if colspan > 1 {
                let width_per_col = cell_natural_width / colspan as f64;
                for c in column..column + colspan {
                    if c < natural_widths.len() {
                        natural_widths[c] = natural_widths[c].max(width_per_col);
                    }
                }
            } else if column < natural_widths.len() {
                natural_widths[column] = natural_widths[column].max(cell_natural_width);
            }

            // Collect min/max constraints from cell styles (for all cells, not just colspan=1)
            if column < natural_widths.len() {
                if let Some(min) = cell_min {
                    min_widths[column] = min_widths[column].max(min);
                }
                if let Some(max) = cell_max {
                    max_widths[column] = max_widths[column].min(max);
                }
            }
        }

        // Apply min/max constraints to natural widths
        for i in 0..self.column_count {
            natural_widths[i] = natural_widths[i].max(min_widths[i]).min(max_widths[i]);
        }

        let natural_sum: f64 = natural_widths.iter().sum();

        // If table has a fixed width, scale columns to fit
        if (self.width - natural_sum).abs() > 0.01 {
            if self.width < natural_sum {
                // Shrink proportionally, respecting min_widths
                let shrinkable: f64 = natural_widths
                    .iter()
                    .zip(min_widths.iter())
                    .map(|(n, m)| n - m)
                    .sum();

                if shrinkable > 0.01 {
                    let needed_shrink = natural_sum - self.width;
                    let factor = (needed_shrink / shrinkable).min(1.0);

                    for i in 0..self.column_count {
                        let can_shrink = natural_widths[i] - min_widths[i];
                        natural_widths[i] -= can_shrink * factor;
                    }
                }
            } else {
                // Expand proportionally, respecting max_widths
                let expandable: f64 = natural_widths
                    .iter()
                    .zip(max_widths.iter())
                    .map(|(n, m)| m - n)
                    .filter(|x| x.is_finite())
                    .sum();

                if expandable > 0.01 {
                    let needed_expand = self.width - natural_sum;
                    let factor = (needed_expand / expandable).min(1.0);

                    for i in 0..self.column_count {
                        let can_expand = max_widths[i] - natural_widths[i];
                        if can_expand.is_finite() {
                            natural_widths[i] += can_expand * factor;
                        }
                    }
                } else {
                    // No max constraints, expand evenly
                    let factor = self.width / natural_sum;
                    for w in &mut natural_widths {
                        *w *= factor;
                    }
                }
            }
        }

        natural_widths
    }

    /// Truncate text to fit within max_width, adding ellipsis if needed
    fn truncate_text(text: &str, max_width: f64, doc: &LayoutDocument) -> String {
        // First check if text already fits
        let text_width = doc.measure_text(text);
        if text_width <= max_width {
            return text.to_string();
        }

        // Measure ellipsis width (use ASCII "..." for standard font compatibility)
        let ellipsis = "...";
        let ellipsis_width = doc.measure_text(ellipsis);

        // If even ellipsis doesn't fit, return empty or just ellipsis
        if ellipsis_width >= max_width {
            return ellipsis.to_string();
        }

        let available_width = max_width - ellipsis_width;

        // Binary search for the longest prefix that fits
        let chars: Vec<char> = text.chars().collect();
        let mut low = 0;
        let mut high = chars.len();

        while low < high {
            let mid = (low + high).div_ceil(2);
            let prefix: String = chars[..mid].iter().collect();
            let prefix_width = doc.measure_text(&prefix);

            if prefix_width <= available_width {
                low = mid;
            } else {
                high = mid - 1;
            }
        }

        // Build result with ellipsis
        if low == 0 {
            ellipsis.to_string()
        } else {
            let prefix: String = chars[..low].iter().collect();
            format!("{}{}", prefix.trim_end(), ellipsis)
        }
    }

    /// Simple text wrapping (will use doc's wrap method when exposed)
    fn simple_wrap(text: &str, max_width: f64, doc: &LayoutDocument) -> Vec<String> {
        let mut lines = Vec::new();

        for paragraph in text.split('\n') {
            if paragraph.is_empty() {
                lines.push(String::new());
                continue;
            }

            let mut current_line = String::new();

            for word in paragraph.split_whitespace() {
                let test_line = if current_line.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current_line, word)
                };

                let width = doc.measure_text(&test_line);

                if width <= max_width || current_line.is_empty() {
                    current_line = test_line;
                } else {
                    lines.push(current_line);
                    current_line = word.to_string();
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

    /// Calculate content height without needing full Cell reference
    ///
    /// This avoids cloning ImageContent.data by extracting only necessary info.
    /// content_info: (text, image_height, subtable_row_count)
    fn calculate_content_height(
        doc: &mut LayoutDocument,
        col_width: f64,
        h_pad: f64,
        v_pad: f64,
        font: Option<&str>,
        font_size: Option<f64>,
        content_info: (Option<String>, Option<f64>, Option<usize>),
    ) -> f64 {
        let content_width = col_width - h_pad;

        // Save and apply font settings
        let old_font = doc.current_font().to_string();
        let old_size = doc.current_font_size();

        if let Some(f) = font {
            doc.inner_mut().font(f).size(font_size.unwrap_or(old_size));
        } else if let Some(size) = font_size {
            doc.inner_mut().font(&old_font).size(size);
        }

        let (text, img_height, sub_rows) = content_info;

        let height = if let Some(ref text) = text {
            if text.is_empty() {
                v_pad + doc.line_height()
            } else {
                let lines = Self::simple_wrap(text, content_width, doc);
                let line_height = doc.line_height();
                let text_height = lines.len() as f64 * line_height;
                text_height + v_pad
            }
        } else if let Some(h) = img_height {
            h + v_pad
        } else if let Some(rows) = sub_rows {
            let row_count = rows.max(1) as f64;
            let estimated_row_height = doc.line_height() + 10.0;
            row_count * estimated_row_height + v_pad
        } else {
            // Empty or image without explicit height
            v_pad + doc.line_height()
        };

        // Restore font settings
        doc.inner_mut().font(&old_font).size(old_size);

        height
    }

    /// Position all cells based on calculated widths and heights
    fn position_cells(&mut self) {
        // Calculate cumulative positions
        let x_positions: Vec<f64> = std::iter::once(0.0)
            .chain(self.column_widths.iter().scan(0.0, |acc, &w| {
                *acc += w;
                Some(*acc)
            }))
            .collect();

        let y_positions: Vec<f64> = std::iter::once(0.0)
            .chain(self.row_heights.iter().scan(0.0, |acc, &h| {
                *acc += h;
                Some(*acc)
            }))
            .collect();

        // Assign positions to cells (handling colspan/rowspan)
        for cell in &mut self.cells {
            cell.x = x_positions.get(cell.column).copied().unwrap_or(0.0);
            cell.y = y_positions.get(cell.row).copied().unwrap_or(0.0);

            // Width = sum of spanned columns
            cell.width = (cell.column..cell.column + cell.colspan)
                .filter_map(|c| self.column_widths.get(c))
                .sum();

            // Height = sum of spanned rows
            cell.height = (cell.row..cell.row + cell.rowspan)
                .filter_map(|r| self.row_heights.get(r))
                .sum();
        }
    }

    /// Get number of rows
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Get number of columns
    pub fn column_count(&self) -> usize {
        self.column_count
    }

    /// Get total table width
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Get total table height
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Get cells iterator
    pub fn cells(&self) -> impl Iterator<Item = &Cell> {
        self.cells.iter()
    }

    /// Get mutable cells iterator
    pub fn cells_mut(&mut self) -> impl Iterator<Item = &mut Cell> {
        self.cells.iter_mut()
    }

    /// Get cells in a specific row
    pub fn row(&self, row: usize) -> impl Iterator<Item = &Cell> {
        self.cells.iter().filter(move |c| c.row == row)
    }

    /// Get mutable cells in a specific row
    pub fn row_mut(&mut self, row: usize) -> impl Iterator<Item = &mut Cell> {
        self.cells.iter_mut().filter(move |c| c.row == row)
    }

    /// Get cells in a specific column
    pub fn column(&self, col: usize) -> impl Iterator<Item = &Cell> {
        self.cells.iter().filter(move |c| c.column == col)
    }

    /// Get a selection of all cells for batch styling
    ///
    /// # Example
    /// ```ignore
    /// table.select_all().background_color(Color::WHITE);
    /// ```
    pub fn select_all(&mut self) -> CellSelection<'_> {
        let indices: Vec<usize> = (0..self.cells.len()).collect();
        CellSelection::new(&mut self.cells, indices)
    }

    /// Get a selection of cells in specific rows for batch styling
    ///
    /// # Example
    /// ```ignore
    /// table.select_rows(0).background_color(Color::gray(0.8)); // Header
    /// table.select_rows(1..5).align(TextAlign::Right);
    /// ```
    pub fn select_rows(&mut self, rows: impl RangeBoundsExt) -> CellSelection<'_> {
        let rows_range = rows.to_range(self.row_count);
        let indices: Vec<usize> = self
            .cells
            .iter()
            .enumerate()
            .filter(|(_, c)| rows_range.contains(&c.row))
            .map(|(i, _)| i)
            .collect();
        CellSelection::new(&mut self.cells, indices)
    }

    /// Get a selection of cells in specific columns for batch styling
    ///
    /// # Example
    /// ```ignore
    /// table.select_columns(1..3).align(TextAlign::Right);
    /// table.select_columns(0).font("Helvetica-Bold", Some(12.0));
    /// ```
    pub fn select_columns(&mut self, cols: impl RangeBoundsExt) -> CellSelection<'_> {
        let cols_range = cols.to_range(self.column_count);
        let indices: Vec<usize> = self
            .cells
            .iter()
            .enumerate()
            .filter(|(_, c)| cols_range.contains(&c.column))
            .map(|(i, _)| i)
            .collect();
        CellSelection::new(&mut self.cells, indices)
    }

    /// Apply a style to all cells in a row
    pub fn style_row(&mut self, row: usize, style: CellStyle) {
        for cell in self.cells.iter_mut().filter(|c| c.row == row) {
            cell.style = style.clone();
        }
    }

    /// Apply background color to a row
    pub fn row_background(&mut self, row: usize, color: Color) {
        for cell in self.cells.iter_mut().filter(|c| c.row == row) {
            cell.style.background_color = Some(color);
        }
    }

    /// Apply a style to multiple rows
    pub fn style_rows(&mut self, rows: std::ops::Range<usize>, style: CellStyle) {
        for cell in self.cells.iter_mut().filter(|c| rows.contains(&c.row)) {
            cell.style = style.clone();
        }
    }

    /// Apply background color to multiple rows
    pub fn rows_background(&mut self, rows: std::ops::Range<usize>, color: Color) {
        for cell in self.cells.iter_mut().filter(|c| rows.contains(&c.row)) {
            cell.style.background_color = Some(color);
        }
    }

    /// Apply a style to all cells in a column
    pub fn style_column(&mut self, col: usize, style: CellStyle) {
        for cell in self.cells.iter_mut().filter(|c| c.column == col) {
            cell.style = style.clone();
        }
    }

    /// Apply background color to a column
    pub fn column_background(&mut self, col: usize, color: Color) {
        for cell in self.cells.iter_mut().filter(|c| c.column == col) {
            cell.style.background_color = Some(color);
        }
    }

    /// Apply text alignment to a column
    pub fn column_align(&mut self, col: usize, align: crate::api::layout::TextAlign) {
        for cell in self.cells.iter_mut().filter(|c| c.column == col) {
            cell.style.align = align;
        }
    }

    /// Get a specific cell by row and column
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.cells.iter().find(|c| c.row == row && c.column == col)
    }

    /// Get a mutable reference to a specific cell
    pub fn cell_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        self.cells
            .iter_mut()
            .find(|c| c.row == row && c.column == col)
    }

    /// Set text color for all cells in a row
    pub fn row_text_color(&mut self, row: usize, color: Color) {
        for cell in self.cells.iter_mut().filter(|c| c.row == row) {
            cell.style.text_color = color;
        }
    }

    /// Set font for all cells in a row
    pub fn row_font(&mut self, row: usize, font: &str, size: Option<f64>) {
        for cell in self.cells.iter_mut().filter(|c| c.row == row) {
            cell.style.font = Some(font.to_string());
            if let Some(s) = size {
                cell.style.font_size = Some(s);
            }
        }
    }

    /// Disable borders between rows (keep outer borders)
    pub fn no_internal_horizontal_borders(&mut self) {
        for cell in &mut self.cells {
            if cell.row > 0 {
                cell.style.borders[0] = false; // top border
            }
            if cell.row < self.row_count - 1 {
                cell.style.borders[2] = false; // bottom border (will be covered by next row's top)
            }
        }
    }

    /// Disable borders between columns (keep outer borders)
    pub fn no_internal_vertical_borders(&mut self) {
        for cell in &mut self.cells {
            if cell.column > 0 {
                cell.style.borders[3] = false; // left border
            }
            if cell.column < self.column_count - 1 {
                cell.style.borders[1] = false; // right border
            }
        }
    }

    /// Disable all internal borders (keep outer borders only)
    pub fn no_internal_borders(&mut self) {
        self.no_internal_horizontal_borders();
        self.no_internal_vertical_borders();
    }

    /// Apply alternating row colors
    fn apply_row_colors(&mut self) {
        if let Some(ref colors) = self.options.row_colors {
            if colors.is_empty() {
                return;
            }
            let header_rows = self.options.header;
            for cell in &mut self.cells {
                if cell.row >= header_rows {
                    let color_idx = (cell.row - header_rows) % colors.len();
                    if cell.style.background_color.is_none() {
                        cell.style.background_color = Some(colors[color_idx]);
                    }
                }
            }
        }
    }

    /// Draw the table at the given position (no page breaks)
    pub fn draw(&mut self, doc: &mut LayoutDocument, origin: [f64; 2]) {
        // Apply row colors before drawing
        self.apply_row_colors();

        // Calculate x offset based on position
        let x_offset = match self.options.position {
            TablePosition::Left => 0.0,
            TablePosition::Center => (doc.bounds().width() - self.width) / 2.0,
            TablePosition::Right => doc.bounds().width() - self.width,
        };

        let table_x = origin[0] + x_offset;
        let table_y = origin[1];

        // Draw all backgrounds first
        for cell in &self.cells {
            if let Some(bg) = cell.style.background_color {
                let cell_x = table_x + cell.x;
                let cell_y = table_y - cell.y; // y decreases downward

                doc.inner_mut().fill(|ctx| {
                    ctx.color(bg.r, bg.g, bg.b);
                    ctx.rectangle([cell_x, cell_y], cell.width, cell.height);
                });
            }
        }

        // Draw all borders
        for cell in &self.cells {
            let cell_x = table_x + cell.x;
            let cell_y = table_y - cell.y;

            self.draw_cell_borders(doc, cell, cell_x, cell_y);
        }

        // Draw all content
        for cell in &self.cells {
            let cell_x = table_x + cell.x;
            let cell_y = table_y - cell.y;

            self.draw_cell_content(doc, cell, cell_x, cell_y);
        }
    }

    /// Draw the table with automatic page breaks
    ///
    /// When a row doesn't fit on the current page, a new page is started
    /// and header rows (if configured) are repeated.
    ///
    /// Returns the height consumed on the final page (useful for cursor positioning).
    pub fn draw_with_page_breaks(&mut self, doc: &mut LayoutDocument, origin: [f64; 2]) -> f64 {
        // Apply row colors before drawing
        self.apply_row_colors();

        // Calculate x offset based on position
        let x_offset = match self.options.position {
            TablePosition::Left => 0.0,
            TablePosition::Center => (doc.bounds().width() - self.width) / 2.0,
            TablePosition::Right => doc.bounds().width() - self.width,
        };

        let table_x = origin[0] + x_offset;
        let header_rows = self.options.header;
        let page_bottom = doc.bounds().absolute_bottom();

        // Calculate header height for repeating on new pages
        let _header_height: f64 = self.row_heights[..header_rows.min(self.row_count)]
            .iter()
            .sum();

        let mut current_y = origin[1];
        let mut current_page_height = 0.0; // Height on current page only
        let mut current_row = 0;

        while current_row < self.row_count {
            let row_height = self.row_heights[current_row];

            // Check if we need a page break (skip for header rows on first page)
            let needs_page_break = if current_row < header_rows {
                // Header rows: check if they fit
                current_y - row_height < page_bottom
            } else {
                // Data rows: check if row fits
                current_y - row_height < page_bottom
            };

            if needs_page_break && current_row >= header_rows {
                // Start a new page
                doc.start_new_page();
                // cursor() returns relative to bounds.bottom, convert to absolute Y
                current_y = doc.bounds().absolute_bottom() + doc.cursor();
                current_page_height = 0.0; // Reset for new page

                // Redraw header rows on new page
                if header_rows > 0 {
                    for header_row in 0..header_rows {
                        self.draw_row(doc, header_row, table_x, current_y);
                        current_y -= self.row_heights[header_row];
                        current_page_height += self.row_heights[header_row];
                    }
                }
            }

            // Draw the current row
            self.draw_row(doc, current_row, table_x, current_y);
            current_y -= row_height;
            current_page_height += row_height;
            current_row += 1;
        }

        // Update document cursor to the end of the table
        // set_cursor expects relative Y (from bounds.bottom), convert from absolute
        doc.set_cursor(current_y - doc.bounds().absolute_bottom());
        current_page_height
    }

    /// Draw a single row at the specified position
    fn draw_row(&self, doc: &mut LayoutDocument, row: usize, table_x: f64, row_y: f64) {
        // Get y offset for this row
        let row_y_offset: f64 = self.row_heights[..row].iter().sum();

        // Draw cells in this row (backgrounds first, then borders, then content)
        let cells_in_row: Vec<_> = self.cells.iter().filter(|c| c.row == row).collect();

        // Backgrounds
        for cell in &cells_in_row {
            if let Some(bg) = cell.style.background_color {
                // Cell's y is relative to table top, but we're drawing relative to row_y
                let cell_y_in_row = cell.y - row_y_offset;
                let cell_x = table_x + cell.x;
                let cell_y = row_y - cell_y_in_row;

                doc.inner_mut().fill(|ctx| {
                    ctx.color(bg.r, bg.g, bg.b);
                    ctx.rectangle([cell_x, cell_y], cell.width, cell.height);
                });
            }
        }

        // Borders
        for cell in &cells_in_row {
            let cell_y_in_row = cell.y - row_y_offset;
            let cell_x = table_x + cell.x;
            let cell_y = row_y - cell_y_in_row;

            self.draw_cell_borders(doc, cell, cell_x, cell_y);
        }

        // Content
        for cell in &cells_in_row {
            let cell_y_in_row = cell.y - row_y_offset;
            let cell_x = table_x + cell.x;
            let cell_y = row_y - cell_y_in_row;

            self.draw_cell_content(doc, cell, cell_x, cell_y);
        }
    }

    /// Draw borders for a cell
    fn draw_cell_borders(&self, doc: &mut LayoutDocument, cell: &Cell, x: f64, y: f64) {
        let w = cell.width;
        let h = cell.height;
        let style = &cell.style;

        doc.inner_mut().stroke(|ctx| {
            // Top border
            if style.borders[0] {
                ctx.line_width(style.border_widths[0]);
                ctx.color(
                    style.border_colors[0].r,
                    style.border_colors[0].g,
                    style.border_colors[0].b,
                );
                Self::apply_border_line(ctx, style.border_lines[0]);
                ctx.line([x, y], [x + w, y]);
            }

            // Right border
            if style.borders[1] {
                ctx.line_width(style.border_widths[1]);
                ctx.color(
                    style.border_colors[1].r,
                    style.border_colors[1].g,
                    style.border_colors[1].b,
                );
                Self::apply_border_line(ctx, style.border_lines[1]);
                ctx.line([x + w, y], [x + w, y - h]);
            }

            // Bottom border
            if style.borders[2] {
                ctx.line_width(style.border_widths[2]);
                ctx.color(
                    style.border_colors[2].r,
                    style.border_colors[2].g,
                    style.border_colors[2].b,
                );
                Self::apply_border_line(ctx, style.border_lines[2]);
                ctx.line([x, y - h], [x + w, y - h]);
            }

            // Left border
            if style.borders[3] {
                ctx.line_width(style.border_widths[3]);
                ctx.color(
                    style.border_colors[3].r,
                    style.border_colors[3].g,
                    style.border_colors[3].b,
                );
                Self::apply_border_line(ctx, style.border_lines[3]);
                ctx.line([x, y], [x, y - h]);
            }
        });
    }

    /// Apply border line style
    fn apply_border_line(ctx: &mut crate::api::StrokeContext, line: BorderLine) {
        match line {
            BorderLine::Solid => ctx.undash(),
            BorderLine::Dashed => ctx.dash(&[4.0, 2.0]),
            BorderLine::Dotted => ctx.dash(&[1.0, 1.0]),
        };
    }

    /// Draw content for a cell
    fn draw_cell_content(&self, doc: &mut LayoutDocument, cell: &Cell, x: f64, y: f64) {
        match &cell.content {
            CellContent::Text(text) => {
                self.draw_text_content(doc, cell, x, y, text);
            }
            CellContent::Image(image) => {
                self.draw_image_content(doc, cell, x, y, image);
            }
            CellContent::Subtable(subtable) => {
                self.draw_subtable_content(doc, cell, x, y, subtable);
            }
            CellContent::Empty => {}
        }
    }

    /// Draw text content for a cell
    fn draw_text_content(&self, doc: &mut LayoutDocument, cell: &Cell, x: f64, y: f64, text: &str) {
        if text.is_empty() {
            return;
        }

        let style = &cell.style;
        let content_x = x + style.padding[3];
        let content_y = y - style.padding[0];
        let content_width = cell.width - style.horizontal_padding();
        let content_height = cell.height - style.vertical_padding();

        // Save font state
        let old_font = doc.current_font().to_string();
        let old_size = doc.current_font_size();

        // Determine initial font settings
        let cell_font = style.font.clone().unwrap_or_else(|| old_font.clone());
        let mut font_size = style.font_size.unwrap_or(old_size);

        // Apply cell font settings
        doc.inner_mut().font(&cell_font).size(font_size);

        // Handle shrink_to_fit overflow
        let (lines, final_font_size) = match style.overflow {
            TextOverflow::ShrinkToFit(min_size) => self.calculate_shrink_to_fit_text(
                doc,
                text,
                &cell_font,
                font_size,
                min_size,
                content_width,
                content_height,
                style.single_line,
            ),
            TextOverflow::Truncate => {
                // Truncate text if single_line and too wide
                let lines = if style.single_line {
                    vec![Self::truncate_text(text, content_width, doc)]
                } else {
                    Self::simple_wrap(text, content_width, doc)
                };
                (lines, font_size)
            }
            TextOverflow::Expand => {
                // Expand: wrap text normally, cell height expands to fit
                // (single_line with Expand shows full text without truncation)
                let lines = if style.single_line {
                    vec![text.to_string()]
                } else {
                    Self::simple_wrap(text, content_width, doc)
                };
                (lines, font_size)
            }
        };

        // Apply the final font size (may have been shrunk)
        if (final_font_size - font_size).abs() > 0.001 {
            font_size = final_font_size;
            doc.inner_mut().font(&cell_font).size(font_size);
        }

        let line_height = doc.line_height();
        let total_text_height = lines.len() as f64 * line_height;

        // Calculate vertical offset based on valign
        let y_offset = match style.valign {
            VerticalAlign::Top => 0.0,
            VerticalAlign::Center => ((content_height - total_text_height) / 2.0).max(0.0),
            VerticalAlign::Bottom => (content_height - total_text_height).max(0.0),
        };

        // Set text color
        let color = style.text_color;
        let current_page = doc.inner().current_page;
        doc.inner_mut().pages[current_page]
            .content
            .set_fill_color_rgb(color.r, color.g, color.b);

        // Draw each line
        let ascender = doc.ascender_height();
        for (i, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }

            let line_y = content_y - y_offset - (i as f64 * line_height) - ascender;

            // Calculate x position based on alignment
            let line_width = doc.measure_text(line);
            let line_x = match style.align {
                TextAlign::Left => content_x,
                TextAlign::Center => content_x + (content_width - line_width) / 2.0,
                TextAlign::Right => content_x + content_width - line_width,
                TextAlign::Justify => content_x, // TODO: implement justify
            };

            doc.inner_mut().text_at(line, [line_x, line_y]);
        }

        // Restore font state
        doc.inner_mut().font(&old_font).size(old_size);
    }

    /// Calculate text with shrink-to-fit overflow handling
    ///
    /// Iteratively reduces font size until text fits within the cell,
    /// or reaches the minimum font size.
    #[allow(clippy::too_many_arguments)]
    fn calculate_shrink_to_fit_text(
        &self,
        doc: &mut LayoutDocument,
        text: &str,
        font: &str,
        initial_size: f64,
        min_size: f64,
        content_width: f64,
        content_height: f64,
        single_line: bool,
    ) -> (Vec<String>, f64) {
        let mut font_size = initial_size;
        let step = 0.5; // Reduce by 0.5pt each iteration

        loop {
            doc.inner_mut().font(font).size(font_size);

            let lines = if single_line {
                vec![text.to_string()]
            } else {
                Self::simple_wrap(text, content_width, doc)
            };

            let line_height = doc.line_height();
            let total_height = lines.len() as f64 * line_height;

            // Check if text fits
            let fits = if single_line {
                // For single line, check both width and height
                let text_width = doc.measure_text(text);
                text_width <= content_width && total_height <= content_height
            } else {
                // For wrapped text, just check height
                total_height <= content_height
            };

            if fits || font_size <= min_size {
                return (lines, font_size.max(min_size));
            }

            font_size -= step;
        }
    }

    /// Draw image content for a cell
    fn draw_image_content(
        &self,
        doc: &mut LayoutDocument,
        cell: &Cell,
        x: f64,
        y: f64,
        image: &ImageContent,
    ) {
        use crate::api::image::ImageOptions;

        let style = &cell.style;
        let content_x = x + style.padding[3];
        // PDF y-coordinate: content_y is top of content area, but image.at uses bottom-left
        let content_y_top = y - style.padding[0];
        let content_width = cell.width - style.horizontal_padding();
        let content_height = cell.height - style.vertical_padding();

        // Calculate image dimensions based on fit mode
        let options = match image.fit {
            ImageFit::Contain => ImageOptions::fit_at(
                content_x,
                content_y_top - content_height,
                content_width,
                content_height,
            ),
            ImageFit::Cover => {
                // For cover, we'd need to crop which isn't directly supported
                // Fall back to contain for now
                ImageOptions::fit_at(
                    content_x,
                    content_y_top - content_height,
                    content_width,
                    content_height,
                )
            }
            ImageFit::Fill => ImageOptions::new(
                content_x,
                content_y_top - content_height,
                content_width,
                content_height,
            ),
            ImageFit::None => {
                let mut opts = ImageOptions::at(content_x, content_y_top - content_height);
                if image.scale != 1.0 {
                    opts = opts.with_scale(image.scale);
                }
                opts
            }
        };

        // Draw the image (Vec<u8> implements ImageSource)
        let _ = doc.inner_mut().image_with(image.data.clone(), options);
    }

    /// Draw subtable content for a cell
    fn draw_subtable_content(
        &self,
        doc: &mut LayoutDocument,
        cell: &Cell,
        x: f64,
        y: f64,
        subtable: &SubtableData,
    ) {
        let style = &cell.style;
        let content_x = x + style.padding[3];
        let content_y = y - style.padding[0];
        let content_width = cell.width - style.horizontal_padding();

        // Create a mini-table for the subtable content
        let options = if let Some(ref widths) = subtable.column_widths {
            TableOptions::default().column_widths(widths)
        } else {
            TableOptions {
                width: Some(content_width),
                column_widths: ColumnWidths::Auto,
                ..Default::default()
            }
        };

        // Convert rows to the format expected by Table::new
        let data: Vec<Vec<&str>> = subtable
            .rows
            .iter()
            .map(|row| row.iter().map(|s| s.as_str()).collect())
            .collect();

        // Create and draw the subtable
        let mut sub = Table::new(&data, options, content_width);
        sub.calculate_layout(doc);
        sub.draw(doc, [content_x, content_y]);
    }
}
