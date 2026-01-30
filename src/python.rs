//! Python bindings for pdfcrate
//!
//! This module provides Python bindings using PyO3.

use pyo3::prelude::*;
use std::sync::Mutex;

use crate::api::layout::{
    Color as RustColor, ColumnBoxOptions as RustColumnBoxOptions, GridOptions as RustGridOptions,
    LayoutDocument, Margin as RustMargin, TextAlign as RustTextAlign,
    TextFragment as RustTextFragment,
};
use crate::api::page::{PageLayout, PageSize as RustPageSize};
use crate::api::table::{
    CellStyle as RustCellStyle, TableOptions as RustTableOptions,
    VerticalAlign as RustVerticalAlign,
};
use crate::api::Document as RustDocument;

// ============================================================================
// Color
// ============================================================================

/// Color for fills, strokes, and text
#[pyclass]
#[derive(Clone)]
pub struct Color {
    inner: RustColor,
}

#[pymethods]
impl Color {
    /// Create a color from RGB values (0.0-1.0)
    #[new]
    #[pyo3(signature = (r, g, b))]
    fn new(r: f64, g: f64, b: f64) -> Self {
        Self {
            inner: RustColor::rgb(r, g, b),
        }
    }

    /// Create a color from RGB values (0-255)
    #[staticmethod]
    fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            inner: RustColor::rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0),
        }
    }

    /// Create a grayscale color (0.0-1.0)
    #[staticmethod]
    fn gray(value: f64) -> Self {
        Self {
            inner: RustColor::gray(value),
        }
    }

    /// Create a color from hex string (e.g., "#FF0000" or "FF0000")
    #[staticmethod]
    fn hex(value: &str) -> Self {
        Self {
            inner: RustColor::hex(value),
        }
    }

    /// Create a color from CSS color name or any CSS color string
    #[staticmethod]
    fn parse(value: &str) -> Self {
        Self {
            inner: RustColor::parse(value),
        }
    }

    #[staticmethod]
    fn black() -> Self {
        Self {
            inner: RustColor::BLACK,
        }
    }
    #[staticmethod]
    fn white() -> Self {
        Self {
            inner: RustColor::WHITE,
        }
    }
    #[staticmethod]
    fn red() -> Self {
        Self {
            inner: RustColor::RED,
        }
    }
    #[staticmethod]
    fn green() -> Self {
        Self {
            inner: RustColor::GREEN,
        }
    }
    #[staticmethod]
    fn blue() -> Self {
        Self {
            inner: RustColor::BLUE,
        }
    }
}

// ============================================================================
// Margin
// ============================================================================

/// Page margins
#[pyclass]
#[derive(Clone)]
pub struct Margin {
    inner: RustMargin,
}

#[pymethods]
impl Margin {
    /// Create margins with individual values (top, right, bottom, left)
    #[new]
    #[pyo3(signature = (top, right=None, bottom=None, left=None))]
    fn new(top: f64, right: Option<f64>, bottom: Option<f64>, left: Option<f64>) -> Self {
        let inner = match (right, bottom, left) {
            (None, None, None) => RustMargin::all(top),
            (Some(r), None, None) => RustMargin::symmetric(top, r),
            (Some(r), Some(b), None) => RustMargin::new(top, r, b, r),
            (Some(r), Some(b), Some(l)) => RustMargin::new(top, r, b, l),
            _ => RustMargin::all(top),
        };
        Self { inner }
    }

    /// Create uniform margins on all sides
    #[staticmethod]
    fn all(value: f64) -> Self {
        Self {
            inner: RustMargin::all(value),
        }
    }

    /// Create symmetric margins (vertical, horizontal)
    #[staticmethod]
    fn symmetric(vertical: f64, horizontal: f64) -> Self {
        Self {
            inner: RustMargin::symmetric(vertical, horizontal),
        }
    }

    #[getter]
    fn top(&self) -> f64 {
        self.inner.top
    }
    #[getter]
    fn right(&self) -> f64 {
        self.inner.right
    }
    #[getter]
    fn bottom(&self) -> f64 {
        self.inner.bottom
    }
    #[getter]
    fn left(&self) -> f64 {
        self.inner.left
    }
}

// ============================================================================
// PageSize
// ============================================================================

/// Page size presets and custom sizes
#[pyclass]
#[derive(Clone)]
pub struct PageSize {
    inner: RustPageSize,
}

#[pymethods]
impl PageSize {
    /// Create a custom page size (width, height in points)
    #[new]
    fn new(width: f64, height: f64) -> Self {
        Self {
            inner: RustPageSize::Custom(width, height),
        }
    }

    /// Create page size from millimeters
    #[staticmethod]
    fn from_mm(width: f64, height: f64) -> Self {
        let w_pt = width * 72.0 / 25.4;
        let h_pt = height * 72.0 / 25.4;
        Self {
            inner: RustPageSize::Custom(w_pt, h_pt),
        }
    }

    /// Create page size from centimeters
    #[staticmethod]
    fn from_cm(width: f64, height: f64) -> Self {
        Self::from_mm(width * 10.0, height * 10.0)
    }

    /// Create page size from inches
    #[staticmethod]
    fn from_inches(width: f64, height: f64) -> Self {
        Self {
            inner: RustPageSize::Custom(width * 72.0, height * 72.0),
        }
    }

    fn dimensions(&self) -> (f64, f64) {
        self.inner.dimensions(PageLayout::Portrait)
    }

    #[staticmethod]
    fn a4() -> Self {
        Self {
            inner: RustPageSize::A4,
        }
    }
    #[staticmethod]
    fn a3() -> Self {
        Self {
            inner: RustPageSize::A3,
        }
    }
    #[staticmethod]
    fn a5() -> Self {
        Self {
            inner: RustPageSize::A5,
        }
    }
    #[staticmethod]
    fn letter() -> Self {
        Self {
            inner: RustPageSize::Letter,
        }
    }
    #[staticmethod]
    fn legal() -> Self {
        Self {
            inner: RustPageSize::Legal,
        }
    }
}

// ============================================================================
// TextAlign / VerticalAlign
// ============================================================================

/// Text alignment options
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

impl From<TextAlign> for RustTextAlign {
    fn from(align: TextAlign) -> Self {
        match align {
            TextAlign::Left => RustTextAlign::Left,
            TextAlign::Center => RustTextAlign::Center,
            TextAlign::Right => RustTextAlign::Right,
            TextAlign::Justify => RustTextAlign::Justify,
        }
    }
}

/// Vertical alignment options
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}

impl From<VerticalAlign> for RustVerticalAlign {
    fn from(align: VerticalAlign) -> Self {
        match align {
            VerticalAlign::Top => RustVerticalAlign::Top,
            VerticalAlign::Center => RustVerticalAlign::Center,
            VerticalAlign::Bottom => RustVerticalAlign::Bottom,
        }
    }
}

// ============================================================================
// Overflow / TextBoxResult
// ============================================================================

/// Text overflow behavior for text boxes
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    /// Truncate text that exceeds the box height
    Truncate,
    /// Expand the box height to fit all content
    Expand,
}

/// Result information from text_box rendering
#[pyclass]
#[derive(Clone)]
pub struct TextBoxResult {
    /// Actual height used by the text box
    #[pyo3(get)]
    pub height: f64,
    /// Whether text was truncated due to overflow
    #[pyo3(get)]
    pub truncated: bool,
    /// Actual font size used for rendering
    #[pyo3(get)]
    pub font_size: f64,
    /// Number of lines actually rendered
    #[pyo3(get)]
    pub lines_rendered: usize,
    /// Total number of lines in the wrapped text
    #[pyo3(get)]
    pub total_lines: usize,
}

#[pymethods]
impl TextBoxResult {
    fn __repr__(&self) -> String {
        format!(
            "TextBoxResult(height={:.1}, truncated={}, font_size={:.1}, lines_rendered={}, total_lines={})",
            self.height, self.truncated, self.font_size, self.lines_rendered, self.total_lines
        )
    }
}

// ============================================================================
// EmbeddedImage / EmbeddedFont
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct EmbeddedImage {
    #[allow(dead_code)]
    name: String,
    width: u32,
    height: u32,
}

#[pymethods]
impl EmbeddedImage {
    #[getter]
    fn width(&self) -> u32 {
        self.width
    }
    #[getter]
    fn height(&self) -> u32 {
        self.height
    }
}

#[pyclass]
#[derive(Clone)]
pub struct EmbeddedFont {
    name: String,
}

#[pymethods]
impl EmbeddedFont {
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    fn __repr__(&self) -> String {
        format!("EmbeddedFont('{}')", self.name)
    }
}

// ============================================================================
// TextFragment for formatted_text
// ============================================================================

/// A text fragment with optional styling for formatted_text
#[pyclass]
#[derive(Clone)]
pub struct TextFragment {
    text: String,
    bold: bool,
    italic: bool,
    color: Option<(f64, f64, f64)>,
    size: Option<f64>,
    font: Option<String>,
    underline: bool,
    strikethrough: bool,
    superscript: bool,
    subscript: bool,
    link: Option<String>,
}

#[pymethods]
impl TextFragment {
    #[new]
    #[pyo3(signature = (text, bold=false, italic=false, color=None, size=None, font=None, underline=false, strikethrough=false, superscript=false, subscript=false, link=None))]
    fn new(
        text: &str,
        bold: bool,
        italic: bool,
        color: Option<&Color>,
        size: Option<f64>,
        font: Option<&str>,
        underline: bool,
        strikethrough: bool,
        superscript: bool,
        subscript: bool,
        link: Option<&str>,
    ) -> Self {
        Self {
            text: text.to_string(),
            bold,
            italic,
            color: color.map(|c| (c.inner.r, c.inner.g, c.inner.b)),
            size,
            font: font.map(|s| s.to_string()),
            underline,
            strikethrough,
            superscript,
            subscript,
            link: link.map(|s| s.to_string()),
        }
    }
}

impl From<&TextFragment> for RustTextFragment {
    fn from(frag: &TextFragment) -> Self {
        use crate::api::layout::FontStyle;
        let style = match (frag.bold, frag.italic) {
            (true, true) => FontStyle::BoldItalic,
            (true, false) => FontStyle::Bold,
            (false, true) => FontStyle::Italic,
            (false, false) => FontStyle::Normal,
        };
        let mut rust_frag = RustTextFragment::new(&frag.text).style(style);
        if let Some(size) = frag.size {
            rust_frag = rust_frag.size(size);
        }
        if let Some(ref font) = frag.font {
            rust_frag = rust_frag.font(font);
        }
        if let Some((r, g, b)) = frag.color {
            rust_frag = rust_frag.color(RustColor::rgb(r, g, b));
        }
        if frag.underline {
            rust_frag = rust_frag.underline();
        }
        if frag.strikethrough {
            rust_frag = rust_frag.strikethrough();
        }
        if frag.superscript {
            rust_frag = rust_frag.superscript();
        }
        if frag.subscript {
            rust_frag = rust_frag.subscript();
        }
        if let Some(ref link) = frag.link {
            rust_frag = rust_frag.link(link);
        }
        rust_frag
    }
}

// ============================================================================
// SpanBuilder - for fluent rich text building: span().bold().italic().end()
// ============================================================================

/// A builder for creating TextFragment with fluent API
#[pyclass]
#[derive(Clone)]
pub struct SpanBuilder {
    text: String,
    bold: bool,
    italic: bool,
    color: Option<(f64, f64, f64)>,
    size: Option<f64>,
    font: Option<String>,
    underline: bool,
    strikethrough: bool,
    superscript: bool,
    subscript: bool,
    link: Option<String>,
}

#[pymethods]
impl SpanBuilder {
    #[new]
    fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            bold: false,
            italic: false,
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

    /// Make the text bold
    fn bold(slf: Py<Self>, py: Python<'_>) -> Py<Self> {
        slf.borrow_mut(py).bold = true;
        slf
    }

    /// Make the text italic
    fn italic(slf: Py<Self>, py: Python<'_>) -> Py<Self> {
        slf.borrow_mut(py).italic = true;
        slf
    }

    /// Set the text color
    fn color(slf: Py<Self>, py: Python<'_>, color: &Color) -> Py<Self> {
        slf.borrow_mut(py).color = Some((color.inner.r, color.inner.g, color.inner.b));
        slf
    }

    /// Set the font size
    fn size(slf: Py<Self>, py: Python<'_>, size: f64) -> Py<Self> {
        slf.borrow_mut(py).size = Some(size);
        slf
    }

    /// Set the font name
    fn font(slf: Py<Self>, py: Python<'_>, font: &str) -> Py<Self> {
        slf.borrow_mut(py).font = Some(font.to_string());
        slf
    }

    /// Enable underline
    fn underline(slf: Py<Self>, py: Python<'_>) -> Py<Self> {
        slf.borrow_mut(py).underline = true;
        slf
    }

    /// Enable strikethrough
    fn strikethrough(slf: Py<Self>, py: Python<'_>) -> Py<Self> {
        slf.borrow_mut(py).strikethrough = true;
        slf
    }

    /// Enable superscript
    fn superscript(slf: Py<Self>, py: Python<'_>) -> Py<Self> {
        slf.borrow_mut(py).superscript = true;
        slf
    }

    /// Enable subscript
    fn subscript(slf: Py<Self>, py: Python<'_>) -> Py<Self> {
        slf.borrow_mut(py).subscript = true;
        slf
    }

    /// Set hyperlink URL
    fn link(slf: Py<Self>, py: Python<'_>, url: &str) -> Py<Self> {
        slf.borrow_mut(py).link = Some(url.to_string());
        slf
    }

    /// Finish building and return a TextFragment
    fn end(&self) -> TextFragment {
        TextFragment {
            text: self.text.clone(),
            bold: self.bold,
            italic: self.italic,
            color: self.color,
            size: self.size,
            font: self.font.clone(),
            underline: self.underline,
            strikethrough: self.strikethrough,
            superscript: self.superscript,
            subscript: self.subscript,
            link: self.link.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "SpanBuilder({:?}, bold={}, italic={})",
            self.text, self.bold, self.italic
        )
    }
}

// ============================================================================
// GridBox - returned by grid() method
// ============================================================================

/// A grid cell reference with position and dimensions
#[pyclass]
#[derive(Clone)]
pub struct GridBox {
    #[pyo3(get)]
    row: usize,
    #[pyo3(get)]
    column: usize,
    #[pyo3(get)]
    width: f64,
    #[pyo3(get)]
    height: f64,
    #[pyo3(get)]
    left: f64,
    #[pyo3(get)]
    top: f64,
}

#[pymethods]
impl GridBox {
    fn __repr__(&self) -> String {
        format!(
            "GridBox(row={}, col={}, {}x{} at ({}, {}))",
            self.row, self.column, self.width, self.height, self.left, self.top
        )
    }
}

// ============================================================================
// OutlineItem for bookmarks
// ============================================================================

/// An outline/bookmark item
#[pyclass]
#[derive(Clone)]
pub struct OutlineItem {
    #[pyo3(get, set)]
    title: String,
    #[pyo3(get, set)]
    page: usize,
    #[pyo3(get, set)]
    children: Vec<OutlineItem>,
}

#[pymethods]
impl OutlineItem {
    #[new]
    #[pyo3(signature = (title, page, children=None))]
    fn new(title: &str, page: usize, children: Option<Vec<OutlineItem>>) -> Self {
        Self {
            title: title.to_string(),
            page,
            children: children.unwrap_or_default(),
        }
    }

    fn __repr__(&self) -> String {
        if self.children.is_empty() {
            format!("OutlineItem({:?}, page={})", self.title, self.page)
        } else {
            format!(
                "OutlineItem({:?}, page={}, children={})",
                self.title,
                self.page,
                self.children.len()
            )
        }
    }
}

// ============================================================================
// CellStyle for table
// ============================================================================

/// Cell styling options for tables
#[pyclass]
#[derive(Clone)]
pub struct CellStyle {
    background_color: Option<(f64, f64, f64)>,
    text_color: (f64, f64, f64),
    font: Option<String>,
    font_size: Option<f64>,
    align: TextAlign,
    valign: VerticalAlign,
    padding: [f64; 4],
}

#[pymethods]
impl CellStyle {
    #[new]
    #[pyo3(signature = (background_color=None, text_color=None, font=None, font_size=None, align=None, valign=None, padding=None))]
    fn new(
        background_color: Option<&Color>,
        text_color: Option<&Color>,
        font: Option<&str>,
        font_size: Option<f64>,
        align: Option<TextAlign>,
        valign: Option<VerticalAlign>,
        padding: Option<f64>,
    ) -> Self {
        Self {
            background_color: background_color.map(|c| (c.inner.r, c.inner.g, c.inner.b)),
            text_color: text_color
                .map(|c| (c.inner.r, c.inner.g, c.inner.b))
                .unwrap_or((0.0, 0.0, 0.0)),
            font: font.map(|s| s.to_string()),
            font_size,
            align: align.unwrap_or(TextAlign::Left),
            valign: valign.unwrap_or(VerticalAlign::Top),
            padding: padding
                .map(|p| [p, p, p, p])
                .unwrap_or([5.0, 5.0, 5.0, 5.0]),
        }
    }
}

impl From<&CellStyle> for RustCellStyle {
    fn from(style: &CellStyle) -> Self {
        let mut rust_style = RustCellStyle::default();
        if let Some((r, g, b)) = style.background_color {
            rust_style.background_color = Some(RustColor::rgb(r, g, b));
        }
        rust_style.text_color =
            RustColor::rgb(style.text_color.0, style.text_color.1, style.text_color.2);
        rust_style.font = style.font.clone();
        rust_style.font_size = style.font_size;
        rust_style.align = style.align.into();
        rust_style.valign = style.valign.into();
        rust_style.padding = style.padding;
        rust_style
    }
}

// ============================================================================
// Cell for table
// ============================================================================

/// A table cell with content and optional styling
#[pyclass]
#[derive(Clone)]
#[allow(dead_code)]
pub struct Cell {
    content: String,
    colspan: usize,
    rowspan: usize,
    style: Option<CellStyle>,
}

#[pymethods]
impl Cell {
    #[new]
    #[pyo3(signature = (content, colspan=1, rowspan=1, style=None))]
    fn new(content: &str, colspan: usize, rowspan: usize, style: Option<CellStyle>) -> Self {
        Self {
            content: content.to_string(),
            colspan,
            rowspan,
            style,
        }
    }

    /// Create a cell spanning multiple columns
    #[staticmethod]
    fn span(content: &str, colspan: usize) -> Self {
        Self {
            content: content.to_string(),
            colspan,
            rowspan: 1,
            style: None,
        }
    }
}

// ============================================================================
// TableOptions
// ============================================================================

/// Options for table creation
#[pyclass]
#[derive(Clone)]
pub struct TableOptions {
    width: Option<f64>,
    header: usize,
    row_colors: Option<Vec<(f64, f64, f64)>>,
    cell_style: Option<CellStyle>,
    column_widths: Option<Vec<f64>>,
    page_breaks: bool,
}

#[pymethods]
impl TableOptions {
    #[new]
    #[pyo3(signature = (width=None, header=0, row_colors=None, cell_style=None, column_widths=None, page_breaks=false))]
    fn new(
        width: Option<f64>,
        header: usize,
        row_colors: Option<Vec<Color>>,
        cell_style: Option<CellStyle>,
        column_widths: Option<Vec<f64>>,
        page_breaks: bool,
    ) -> Self {
        Self {
            width,
            header,
            row_colors: row_colors.map(|colors| {
                colors
                    .iter()
                    .map(|c| (c.inner.r, c.inner.g, c.inner.b))
                    .collect()
            }),
            cell_style,
            column_widths,
            page_breaks,
        }
    }
}

impl From<&TableOptions> for RustTableOptions {
    fn from(opts: &TableOptions) -> Self {
        let mut rust_opts = RustTableOptions::new();
        if let Some(w) = opts.width {
            rust_opts = rust_opts.width(w);
        }
        rust_opts = rust_opts.header(opts.header);
        if let Some(ref colors) = opts.row_colors {
            rust_opts = rust_opts.row_colors(
                colors
                    .iter()
                    .map(|(r, g, b)| RustColor::rgb(*r, *g, *b))
                    .collect(),
            );
        }
        if let Some(ref style) = opts.cell_style {
            rust_opts = rust_opts.cell_style(style.into());
        }
        if let Some(ref widths) = opts.column_widths {
            rust_opts = rust_opts.column_widths(widths);
        }
        rust_opts = rust_opts.page_breaks(opts.page_breaks);
        rust_opts
    }
}

// ============================================================================
// Table (placeholder for table builder)
// ============================================================================

#[pyclass]
pub struct Table {}

#[pymethods]
impl Table {
    #[new]
    fn new() -> Self {
        Self {}
    }
}

// ============================================================================
// Context managers
// ============================================================================

#[pyclass]
pub struct TransparentContext {
    doc: Py<Document>,
    original_fill_alpha: f64,
    original_stroke_alpha: f64,
}

#[pymethods]
impl TransparentContext {
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        py: Python<'_>,
        _exc_type: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_val: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_tb: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> bool {
        let borrowed = self.doc.borrow(py);
        *borrowed.current_fill_alpha.lock().unwrap() = self.original_fill_alpha;
        *borrowed.current_stroke_alpha.lock().unwrap() = self.original_stroke_alpha;
        false
    }
}

#[pyclass]
pub struct IndentContext {
    doc: Py<Document>,
    left: f64,
    right: f64,
}

#[pymethods]
impl IndentContext {
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        py: Python<'_>,
        _exc_type: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_val: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_tb: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> PyResult<bool> {
        // Pop indent using the new public method
        let borrowed = self.doc.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        if let DocumentInner::Layout(layout) = &mut *guard {
            layout.pop_indent(self.left, self.right);
        }
        Ok(false)
    }
}

#[pyclass]
pub struct FloatContext {
    doc: Py<Document>,
    saved_cursor: f64,
    saved_page: usize,
}

#[pymethods]
impl FloatContext {
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        py: Python<'_>,
        _exc_type: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_val: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_tb: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> PyResult<bool> {
        // Restore saved cursor position and page (matches Rust float behavior)
        let borrowed = self.doc.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        if let DocumentInner::Layout(layout) = &mut *guard {
            // Restore page if changed
            if layout.inner().page_number() != self.saved_page {
                layout.inner_mut().go_to_page(self.saved_page - 1); // go_to_page uses 0-based index
            }
            layout.set_cursor(self.saved_cursor);
        }
        Ok(false)
    }
}

#[pyclass]
pub struct FontContext {
    doc: Py<Document>,
    original_font: String,
    original_size: f64,
}

#[pymethods]
impl FontContext {
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        py: Python<'_>,
        _exc_type: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_val: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_tb: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> PyResult<bool> {
        // Restore original font
        let borrowed = self.doc.borrow(py);
        *borrowed.current_font.lock().unwrap() = self.original_font.clone();
        *borrowed.current_font_size.lock().unwrap() = self.original_size;

        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.font(&self.original_font).size(self.original_size);
            }
            DocumentInner::Layout(layout) => {
                layout
                    .inner_mut()
                    .font(&self.original_font)
                    .size(self.original_size);
            }
            DocumentInner::Consumed => {}
        }
        Ok(false)
    }
}

#[pyclass]
pub struct BoundingBoxContext {
    doc: Py<Document>,
    old_cursor: f64,
    fixed_height: Option<f64>,
}

#[pymethods]
impl BoundingBoxContext {
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        py: Python<'_>,
        _exc_type: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_val: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_tb: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> PyResult<bool> {
        // Pop bounding box using the new public method
        let borrowed = self.doc.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        if let DocumentInner::Layout(layout) = &mut *guard {
            layout.pop_bounding_box(self.old_cursor, self.fixed_height);
        }
        Ok(false)
    }

    /// Stroke the bounds of the bounding box
    fn stroke_bounds(&self, py: Python<'_>) -> PyResult<()> {
        let borrowed = self.doc.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        if let DocumentInner::Layout(layout) = &mut *guard {
            layout.stroke_bounds();
        }
        Ok(())
    }

    /// Draw text at cursor in bounding box
    fn text(&self, py: Python<'_>, text: &str) -> PyResult<()> {
        let borrowed = self.doc.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        if let DocumentInner::Layout(layout) = &mut *guard {
            layout.text(text);
        }
        Ok(())
    }
}

#[pyclass]
pub struct PageContext {
    doc: Py<Document>,
    original_page: usize,
}

#[pymethods]
impl PageContext {
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        py: Python<'_>,
        _exc_type: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_val: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_tb: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> PyResult<bool> {
        // Go back to original page
        let borrowed = self.doc.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.go_to_page(self.original_page);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().go_to_page(self.original_page);
            }
            DocumentInner::Consumed => {}
        }
        Ok(false)
    }

    /// Draw text at position
    fn text_at(&self, py: Python<'_>, text: &str, pos: (f64, f64)) -> PyResult<()> {
        let borrowed = self.doc.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.text_at(text, [pos.0, pos.1]);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().text_at(text, [pos.0, pos.1]);
            }
            DocumentInner::Consumed => {}
        }
        Ok(())
    }
}

// ============================================================================
// Document
// ============================================================================

enum DocumentInner {
    Basic(RustDocument),
    Layout(LayoutDocument),
    Consumed,
}

/// PDF Document
#[pyclass(subclass)]
pub struct Document {
    inner: Mutex<DocumentInner>,
    output_path: Option<String>,
    fill_color: Mutex<(f64, f64, f64)>,
    stroke_color: Mutex<(f64, f64, f64)>,
    line_width: Mutex<f64>,
    current_font: Mutex<String>,
    current_font_size: Mutex<f64>,
    current_fill_alpha: Mutex<f64>,
    current_stroke_alpha: Mutex<f64>,
    dash_pattern: Mutex<Option<(Vec<f64>, f64)>>, // (pattern, phase)
}

#[pymethods]
impl Document {
    #[new]
    #[pyo3(signature = (path=None, page_size=None, margin=None))]
    fn new(path: Option<String>, page_size: Option<PageSize>, margin: Option<Margin>) -> Self {
        let size = page_size.map(|p| p.inner).unwrap_or(RustPageSize::A4);

        let inner = if let Some(m) = margin {
            let mut doc = RustDocument::new();
            doc.page_size(size);
            DocumentInner::Layout(LayoutDocument::with_margin(doc, m.inner))
        } else {
            let mut doc = RustDocument::new();
            doc.page_size(size);
            DocumentInner::Basic(doc)
        };

        Self {
            inner: Mutex::new(inner),
            output_path: path,
            fill_color: Mutex::new((0.0, 0.0, 0.0)),
            stroke_color: Mutex::new((0.0, 0.0, 0.0)),
            line_width: Mutex::new(1.0),
            current_font: Mutex::new("Helvetica".to_string()),
            current_font_size: Mutex::new(12.0),
            current_fill_alpha: Mutex::new(1.0),
            current_stroke_alpha: Mutex::new(1.0),
            dash_pattern: Mutex::new(None),
        }
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_val: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_tb: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> PyResult<bool> {
        if let Some(ref path) = self.output_path {
            self.save(path)?;
        }
        Ok(false)
    }

    /// Set document title
    fn title(slf: Py<Self>, py: Python<'_>, title: &str) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.title(title);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().title(title);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Set document author
    fn author(slf: Py<Self>, py: Python<'_>, author: &str) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.author(author);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().author(author);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Set current font and size, returns context manager for with statement
    #[pyo3(signature = (name, size=None))]
    fn font(slf: Py<Self>, py: Python<'_>, name: &str, size: Option<f64>) -> PyResult<FontContext> {
        let borrowed = slf.borrow(py);

        // Save original for context manager
        let original_font = borrowed.current_font.lock().unwrap().clone();
        let original_size = *borrowed.current_font_size.lock().unwrap();

        // Update current state
        *borrowed.current_font.lock().unwrap() = name.to_string();
        if let Some(sz) = size {
            *borrowed.current_font_size.lock().unwrap() = sz;
        }

        let font_size = *borrowed.current_font_size.lock().unwrap();
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.font(name).size(font_size);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().font(name).size(font_size);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);

        Ok(FontContext {
            doc: slf,
            original_font,
            original_size,
        })
    }

    /// Set font size only
    fn size(slf: Py<Self>, py: Python<'_>, size: f64) -> Py<Self> {
        let borrowed = slf.borrow(py);

        *borrowed.current_font_size.lock().unwrap() = size;
        let font_name = borrowed.current_font.lock().unwrap().clone();

        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.font(&font_name).size(size);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().font(&font_name).size(size);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Embed a TrueType font file
    fn embed_font(&self, path: &str) -> PyResult<EmbeddedFont> {
        let mut guard = self.inner.lock().unwrap();
        let name = match &mut *guard {
            DocumentInner::Basic(doc) => doc
                .embed_font_file(path)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?,
            DocumentInner::Layout(layout) => layout
                .inner_mut()
                .embed_font_file(path)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?,
            DocumentInner::Consumed => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Document already consumed",
                ));
            }
        };
        Ok(EmbeddedFont { name })
    }

    /// Measure the width of text with the current font
    #[cfg(feature = "fonts")]
    fn measure_text(&self, text: &str) -> f64 {
        let guard = self.inner.lock().unwrap();
        match &*guard {
            DocumentInner::Basic(doc) => doc.measure_text(text),
            DocumentInner::Layout(layout) => layout.measure_text(text),
            DocumentInner::Consumed => 0.0,
        }
    }

    /// Set fill color
    fn fill(slf: Py<Self>, py: Python<'_>, color: &Color) -> Py<Self> {
        let borrowed = slf.borrow(py);
        *borrowed.fill_color.lock().unwrap() = (color.inner.r, color.inner.g, color.inner.b);
        drop(borrowed);
        slf
    }

    /// Set stroke color and optional line width
    #[pyo3(signature = (color, width=None))]
    fn stroke(slf: Py<Self>, py: Python<'_>, color: &Color, width: Option<f64>) -> Py<Self> {
        let borrowed = slf.borrow(py);
        *borrowed.stroke_color.lock().unwrap() = (color.inner.r, color.inner.g, color.inner.b);
        if let Some(w) = width {
            *borrowed.line_width.lock().unwrap() = w;
        }
        drop(borrowed);
        slf
    }

    /// Set transparency (returns context manager)
    #[pyo3(signature = (fill_opacity, stroke_opacity=None))]
    fn transparent(
        slf: Py<Self>,
        py: Python<'_>,
        fill_opacity: f64,
        stroke_opacity: Option<f64>,
    ) -> PyResult<TransparentContext> {
        let stroke_opacity = stroke_opacity.unwrap_or(fill_opacity);
        let borrowed = slf.borrow(py);
        let original_fill_alpha = *borrowed.current_fill_alpha.lock().unwrap();
        let original_stroke_alpha = *borrowed.current_stroke_alpha.lock().unwrap();
        *borrowed.current_fill_alpha.lock().unwrap() = fill_opacity;
        *borrowed.current_stroke_alpha.lock().unwrap() = stroke_opacity;
        drop(borrowed);

        Ok(TransparentContext {
            doc: slf,
            original_fill_alpha,
            original_stroke_alpha,
        })
    }

    /// Set indent (returns context manager)
    #[pyo3(signature = (left, right=None))]
    fn indent(
        slf: Py<Self>,
        py: Python<'_>,
        left: f64,
        right: Option<f64>,
    ) -> PyResult<IndentContext> {
        let right = right.unwrap_or(0.0);
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        if let DocumentInner::Layout(layout) = &mut *guard {
            layout.push_indent(left, right);
        }
        drop(guard);
        drop(borrowed);

        Ok(IndentContext {
            doc: slf,
            left,
            right,
        })
    }

    /// Push indent manually (use with indent_pop)
    #[pyo3(signature = (left, right=None))]
    fn indent_push(slf: Py<Self>, py: Python<'_>, left: f64, right: Option<f64>) -> Py<Self> {
        let right = right.unwrap_or(0.0);
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        if let DocumentInner::Layout(layout) = &mut *guard {
            layout.push_indent(left, right);
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Pop indent manually
    #[pyo3(signature = (left, right=None))]
    fn indent_pop(slf: Py<Self>, py: Python<'_>, left: f64, right: Option<f64>) -> Py<Self> {
        let right = right.unwrap_or(0.0);
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        if let DocumentInner::Layout(layout) = &mut *guard {
            layout.pop_indent(left, right);
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Float context - executes block without affecting cursor position or page
    ///
    /// After the block executes, the cursor returns to its original position.
    /// If the block creates new pages, float will return to the original page.
    /// This matches Prawn/Rust float() behavior.
    fn float(slf: Py<Self>, py: Python<'_>) -> PyResult<FloatContext> {
        let borrowed = slf.borrow(py);
        let guard = borrowed.inner.lock().unwrap();
        let (saved_cursor, saved_page) = match &*guard {
            DocumentInner::Layout(layout) => (layout.cursor(), layout.inner().page_number()),
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "float() requires margin (LayoutDocument)",
                ));
            }
        };
        drop(guard);
        drop(borrowed);

        Ok(FloatContext {
            doc: slf,
            saved_cursor,
            saved_page,
        })
    }

    /// Create a bounding box (returns context manager or executes callback)
    #[pyo3(signature = (width, height=None, x=0.0, y=0.0, do_=None))]
    fn bounding_box(
        slf: Py<Self>,
        py: Python<'_>,
        width: f64,
        height: Option<f64>,
        x: f64,
        y: f64,
        do_: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> PyResult<Option<BoundingBoxContext>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let old_cursor = if let DocumentInner::Layout(layout) = &mut *guard {
            layout.push_bounding_box([x, y], width, height)
        } else {
            0.0
        };
        drop(guard);
        drop(borrowed);

        // If callback provided, execute it and pop bounds
        if let Some(callback) = do_ {
            let ctx = BoundingBoxContext {
                doc: slf.clone_ref(py),
                old_cursor,
                fixed_height: height,
            };
            // Call the callback with the context, ensuring pop_bounding_box is called even on exception
            let result = callback.call1((ctx,));
            // Always pop the bounding box, regardless of callback success/failure
            let borrowed = slf.borrow(py);
            let mut guard = borrowed.inner.lock().unwrap();
            if let DocumentInner::Layout(layout) = &mut *guard {
                layout.pop_bounding_box(old_cursor, height);
            }
            drop(guard);
            drop(borrowed);
            // Now propagate any error from the callback
            result?;
            Ok(None)
        } else {
            Ok(Some(BoundingBoxContext {
                doc: slf,
                old_cursor,
                fixed_height: height,
            }))
        }
    }

    /// Push a bounding box manually (use with bounds_pop)
    #[pyo3(signature = (width, height=None, x=0.0, y=0.0))]
    fn bounds_push(
        slf: Py<Self>,
        py: Python<'_>,
        width: f64,
        height: Option<f64>,
        x: f64,
        y: f64,
    ) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        if let DocumentInner::Layout(layout) = &mut *guard {
            layout.push_bounding_box([x, y], width, height);
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "bounds_push() requires margin.",
            ));
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Pop a bounding box manually
    fn bounds_pop(slf: Py<Self>, py: Python<'_>) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        if let DocumentInner::Layout(layout) = &mut *guard {
            // Use 0.0 for old_cursor since we don't track it in manual mode
            // The cursor will stay where it is after content
            layout.pop_bounding_box(0.0, None);
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "bounds_pop() requires margin.",
            ));
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Draw a rectangle
    #[pyo3(signature = (pos, width, height, do_fill=true, do_stroke=false))]
    fn rect(
        slf: Py<Self>,
        py: Python<'_>,
        pos: (f64, f64),
        width: f64,
        height: f64,
        do_fill: bool,
        do_stroke: bool,
    ) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let fill_c = *borrowed.fill_color.lock().unwrap();
        let stroke_c = *borrowed.stroke_color.lock().unwrap();
        let lw = *borrowed.line_width.lock().unwrap();
        let fill_alpha = *borrowed.current_fill_alpha.lock().unwrap();
        let stroke_alpha = *borrowed.current_stroke_alpha.lock().unwrap();

        match &mut *guard {
            DocumentInner::Basic(doc) => {
                let draw = |doc: &mut RustDocument| {
                    if do_fill {
                        doc.fill(|ctx| {
                            ctx.color(fill_c).rectangle([pos.0, pos.1], width, height);
                        });
                    }
                    if do_stroke {
                        doc.stroke(|ctx| {
                            ctx.color(stroke_c).line_width(lw).rectangle(
                                [pos.0, pos.1],
                                width,
                                height,
                            );
                        });
                    }
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    doc.transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(doc);
                }
            }
            DocumentInner::Layout(layout) => {
                let draw = |doc: &mut RustDocument| {
                    if do_fill {
                        doc.fill(|ctx| {
                            ctx.color(fill_c).rectangle([pos.0, pos.1], width, height);
                        });
                    }
                    if do_stroke {
                        doc.stroke(|ctx| {
                            ctx.color(stroke_c).line_width(lw).rectangle(
                                [pos.0, pos.1],
                                width,
                                height,
                            );
                        });
                    }
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    layout
                        .inner_mut()
                        .transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(layout.inner_mut());
                }
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Draw a circle
    #[pyo3(signature = (center, radius, do_fill=true, do_stroke=false))]
    fn circle(
        slf: Py<Self>,
        py: Python<'_>,
        center: (f64, f64),
        radius: f64,
        do_fill: bool,
        do_stroke: bool,
    ) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let fill_c = *borrowed.fill_color.lock().unwrap();
        let stroke_c = *borrowed.stroke_color.lock().unwrap();
        let lw = *borrowed.line_width.lock().unwrap();
        let fill_alpha = *borrowed.current_fill_alpha.lock().unwrap();
        let stroke_alpha = *borrowed.current_stroke_alpha.lock().unwrap();

        match &mut *guard {
            DocumentInner::Basic(doc) => {
                let draw = |doc: &mut RustDocument| {
                    if do_fill {
                        doc.fill(|ctx| {
                            ctx.color(fill_c).circle([center.0, center.1], radius);
                        });
                    }
                    if do_stroke {
                        doc.stroke(|ctx| {
                            ctx.color(stroke_c)
                                .line_width(lw)
                                .circle([center.0, center.1], radius);
                        });
                    }
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    doc.transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(doc);
                }
            }
            DocumentInner::Layout(layout) => {
                let draw = |doc: &mut RustDocument| {
                    if do_fill {
                        doc.fill(|ctx| {
                            ctx.color(fill_c).circle([center.0, center.1], radius);
                        });
                    }
                    if do_stroke {
                        doc.stroke(|ctx| {
                            ctx.color(stroke_c)
                                .line_width(lw)
                                .circle([center.0, center.1], radius);
                        });
                    }
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    layout
                        .inner_mut()
                        .transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(layout.inner_mut());
                }
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Draw an ellipse
    #[pyo3(signature = (center, rx, ry, do_fill=true, do_stroke=false))]
    fn ellipse(
        slf: Py<Self>,
        py: Python<'_>,
        center: (f64, f64),
        rx: f64,
        ry: f64,
        do_fill: bool,
        do_stroke: bool,
    ) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let fill_c = *borrowed.fill_color.lock().unwrap();
        let stroke_c = *borrowed.stroke_color.lock().unwrap();
        let lw = *borrowed.line_width.lock().unwrap();
        let fill_alpha = *borrowed.current_fill_alpha.lock().unwrap();
        let stroke_alpha = *borrowed.current_stroke_alpha.lock().unwrap();

        match &mut *guard {
            DocumentInner::Basic(doc) => {
                let draw = |doc: &mut RustDocument| {
                    if do_fill {
                        doc.fill(|ctx| {
                            ctx.color(fill_c).ellipse([center.0, center.1], rx, ry);
                        });
                    }
                    if do_stroke {
                        doc.stroke(|ctx| {
                            ctx.color(stroke_c).line_width(lw).ellipse(
                                [center.0, center.1],
                                rx,
                                ry,
                            );
                        });
                    }
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    doc.transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(doc);
                }
            }
            DocumentInner::Layout(layout) => {
                let draw = |doc: &mut RustDocument| {
                    if do_fill {
                        doc.fill(|ctx| {
                            ctx.color(fill_c).ellipse([center.0, center.1], rx, ry);
                        });
                    }
                    if do_stroke {
                        doc.stroke(|ctx| {
                            ctx.color(stroke_c).line_width(lw).ellipse(
                                [center.0, center.1],
                                rx,
                                ry,
                            );
                        });
                    }
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    layout
                        .inner_mut()
                        .transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(layout.inner_mut());
                }
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Draw a rounded rectangle (top-left origin)
    #[pyo3(signature = (pos, width, height, radius, do_fill=true, do_stroke=false))]
    fn rounded_rect(
        slf: Py<Self>,
        py: Python<'_>,
        pos: (f64, f64),
        width: f64,
        height: f64,
        radius: f64,
        do_fill: bool,
        do_stroke: bool,
    ) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let fill_c = *borrowed.fill_color.lock().unwrap();
        let stroke_c = *borrowed.stroke_color.lock().unwrap();
        let lw = *borrowed.line_width.lock().unwrap();
        let fill_alpha = *borrowed.current_fill_alpha.lock().unwrap();
        let stroke_alpha = *borrowed.current_stroke_alpha.lock().unwrap();

        match &mut *guard {
            DocumentInner::Basic(doc) => {
                let draw = |doc: &mut RustDocument| {
                    if do_fill {
                        doc.fill(|ctx| {
                            ctx.color(fill_c).rounded_rectangle(
                                [pos.0, pos.1],
                                width,
                                height,
                                radius,
                            );
                        });
                    }
                    if do_stroke {
                        doc.stroke(|ctx| {
                            ctx.color(stroke_c).line_width(lw).rounded_rectangle(
                                [pos.0, pos.1],
                                width,
                                height,
                                radius,
                            );
                        });
                    }
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    doc.transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(doc);
                }
            }
            DocumentInner::Layout(layout) => {
                let draw = |doc: &mut RustDocument| {
                    if do_fill {
                        doc.fill(|ctx| {
                            ctx.color(fill_c).rounded_rectangle(
                                [pos.0, pos.1],
                                width,
                                height,
                                radius,
                            );
                        });
                    }
                    if do_stroke {
                        doc.stroke(|ctx| {
                            ctx.color(stroke_c).line_width(lw).rounded_rectangle(
                                [pos.0, pos.1],
                                width,
                                height,
                                radius,
                            );
                        });
                    }
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    layout
                        .inner_mut()
                        .transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(layout.inner_mut());
                }
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Draw a polygon
    #[pyo3(signature = (points, do_fill=true, do_stroke=false))]
    fn polygon(
        slf: Py<Self>,
        py: Python<'_>,
        points: Vec<(f64, f64)>,
        do_fill: bool,
        do_stroke: bool,
    ) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let fill_c = *borrowed.fill_color.lock().unwrap();
        let stroke_c = *borrowed.stroke_color.lock().unwrap();
        let lw = *borrowed.line_width.lock().unwrap();
        let fill_alpha = *borrowed.current_fill_alpha.lock().unwrap();
        let stroke_alpha = *borrowed.current_stroke_alpha.lock().unwrap();

        let pts: Vec<[f64; 2]> = points.iter().map(|(x, y)| [*x, *y]).collect();

        match &mut *guard {
            DocumentInner::Basic(doc) => {
                let draw = |doc: &mut RustDocument| {
                    if do_fill {
                        doc.fill(|ctx| {
                            ctx.color(fill_c).polygon(&pts);
                        });
                    }
                    if do_stroke {
                        doc.stroke(|ctx| {
                            ctx.color(stroke_c).line_width(lw).polygon(&pts);
                        });
                    }
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    doc.transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(doc);
                }
            }
            DocumentInner::Layout(layout) => {
                let draw = |doc: &mut RustDocument| {
                    if do_fill {
                        doc.fill(|ctx| {
                            ctx.color(fill_c).polygon(&pts);
                        });
                    }
                    if do_stroke {
                        doc.stroke(|ctx| {
                            ctx.color(stroke_c).line_width(lw).polygon(&pts);
                        });
                    }
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    layout
                        .inner_mut()
                        .transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(layout.inner_mut());
                }
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Set dash pattern for subsequent strokes
    #[pyo3(signature = (pattern, phase=0.0))]
    fn dash(slf: Py<Self>, py: Python<'_>, pattern: Vec<f64>, phase: f64) -> Py<Self> {
        let borrowed = slf.borrow(py);
        *borrowed.dash_pattern.lock().unwrap() = Some((pattern, phase));
        drop(borrowed);
        slf
    }

    /// Clear dash pattern (solid lines)
    fn undash(slf: Py<Self>, py: Python<'_>) -> Py<Self> {
        let borrowed = slf.borrow(py);
        *borrowed.dash_pattern.lock().unwrap() = None;
        drop(borrowed);
        slf
    }

    /// Draw a line
    fn line(slf: Py<Self>, py: Python<'_>, start: (f64, f64), end: (f64, f64)) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let stroke_c = *borrowed.stroke_color.lock().unwrap();
        let lw = *borrowed.line_width.lock().unwrap();
        let dash = borrowed.dash_pattern.lock().unwrap().clone();
        let fill_alpha = *borrowed.current_fill_alpha.lock().unwrap();
        let stroke_alpha = *borrowed.current_stroke_alpha.lock().unwrap();

        match &mut *guard {
            DocumentInner::Basic(doc) => {
                let draw = |doc: &mut RustDocument| {
                    doc.stroke(|ctx| {
                        ctx.color(stroke_c).line_width(lw);
                        if let Some((ref pattern, phase)) = dash {
                            ctx.dash_with_phase(pattern, phase);
                        }
                        ctx.line([start.0, start.1], [end.0, end.1]);
                        // Note: undash() must NOT be called here - stroke happens on drop,
                        // so undash would clear the dash pattern before the stroke executes
                    });
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    doc.transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(doc);
                }
            }
            DocumentInner::Layout(layout) => {
                let draw = |doc: &mut RustDocument| {
                    doc.stroke(|ctx| {
                        ctx.color(stroke_c).line_width(lw);
                        if let Some((ref pattern, phase)) = dash {
                            ctx.dash_with_phase(pattern, phase);
                        }
                        ctx.line([start.0, start.1], [end.0, end.1]);
                        // Note: undash() must NOT be called here - stroke happens on drop,
                        // so undash would clear the dash pattern before the stroke executes
                    });
                };
                if fill_alpha < 1.0 || stroke_alpha < 1.0 {
                    layout
                        .inner_mut()
                        .transparent(fill_alpha, stroke_alpha, draw);
                } else {
                    draw(layout.inner_mut());
                }
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Draw coordinate axes with tick marks and labels (debug helper)
    ///
    /// Args:
    ///     at: Origin point (x, y) for the axes (default: (20, 20))
    ///     color: RGB color tuple (default: (0.6, 0.6, 0.6) gray)
    ///     step: Distance between tick marks (default: 100)
    #[pyo3(signature = (at=(20.0, 20.0), color=(0.6, 0.6, 0.6), step=100.0))]
    fn stroke_axis(
        slf: Py<Self>,
        py: Python<'_>,
        at: (f64, f64),
        color: (f64, f64, f64),
        step: f64,
    ) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let doc = match &mut *guard {
            DocumentInner::Basic(doc) => doc,
            DocumentInner::Layout(layout) => layout.inner_mut(),
            DocumentInner::Consumed => {
                drop(guard);
                drop(borrowed);
                return slf;
            }
        };

        doc.stroke_axis(
            crate::api::AxisOptions::new()
                .at(at.0, at.1)
                .color(color)
                .step_length(step),
        );

        drop(guard);
        drop(borrowed);
        slf
    }

    /// Draw text at absolute position
    fn text_at(slf: Py<Self>, py: Python<'_>, text: &str, pos: (f64, f64)) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.text_at(text, [pos.0, pos.1]);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().text_at(text, [pos.0, pos.1]);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Draw text at absolute position without kerning (for comparison demos)
    #[cfg(feature = "fonts")]
    fn text_at_no_kerning(slf: Py<Self>, py: Python<'_>, text: &str, pos: (f64, f64)) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.text_at_no_kerning(text, [pos.0, pos.1]);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().text_at_no_kerning(text, [pos.0, pos.1]);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Draw text at cursor position (requires margin)
    /// Draw text at cursor position
    ///
    /// Args:
    ///     text: The text to draw
    ///     indent: Optional one-time indent (doesn't affect subsequent lines)
    #[pyo3(signature = (text, indent=None))]
    fn text(slf: Py<Self>, py: Python<'_>, text: &str, indent: Option<f64>) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "text() requires margin. Use text_at() for absolute positioning.",
                ));
            }
            DocumentInner::Layout(layout) => {
                if let Some(indent_val) = indent {
                    // One-time indent: push, draw, pop
                    layout.push_indent(indent_val, 0.0);
                    layout.text(text);
                    layout.pop_indent(indent_val, 0.0);
                } else {
                    layout.text(text);
                }
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Draw text with word wrapping
    fn text_wrap(slf: Py<Self>, py: Python<'_>, text: &str) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "text_wrap() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                layout.text_wrap(text);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Draw formatted text with mixed styles
    fn formatted_text(
        slf: Py<Self>,
        py: Python<'_>,
        fragments: Vec<TextFragment>,
    ) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "formatted_text() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                let rust_fragments: Vec<RustTextFragment> =
                    fragments.iter().map(|f| f.into()).collect();
                layout.formatted_text(&rust_fragments);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Draw inline-formatted text (HTML-like tags) on a single line
    fn text_inline(slf: Py<Self>, py: Python<'_>, text: &str) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "text_inline() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                layout.text_inline(text);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Draw inline-formatted text with automatic word wrapping
    fn text_wrap_inline(slf: Py<Self>, py: Python<'_>, text: &str) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "text_wrap_inline() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                layout.text_wrap_inline(text);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Multi-column layout with automatic overflow handling
    ///
    /// Divides the current bounding box into columns. Text flows from
    /// one column to the next, and to new pages when all columns are full.
    ///
    /// Args:
    ///     do_: Callback that receives the document to add content
    ///     columns: Number of columns (default: 3)
    ///     spacer: Gap between columns in points (default: current font size)
    #[pyo3(signature = (do_, columns=3, spacer=None))]
    fn column_box(
        slf: Py<Self>,
        py: Python<'_>,
        do_: &Bound<'_, pyo3::types::PyAny>,
        columns: usize,
        spacer: Option<f64>,
    ) -> PyResult<Py<Self>> {
        // Set up column state
        {
            let borrowed = slf.borrow(py);
            let mut guard = borrowed.inner.lock().unwrap();
            match &mut *guard {
                DocumentInner::Basic(_) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        "column_box() requires margin.",
                    ));
                }
                DocumentInner::Layout(layout) => {
                    let opts = RustColumnBoxOptions {
                        columns,
                        spacer,
                        reflow_margins: false,
                    };
                    layout.column_box_begin(opts);
                }
                DocumentInner::Consumed => {}
            }
        }

        // Execute callback
        let result = do_.call1((slf.clone_ref(py),));

        // Always clean up column state
        {
            let borrowed = slf.borrow(py);
            let mut guard = borrowed.inner.lock().unwrap();
            if let DocumentInner::Layout(layout) = &mut *guard {
                layout.column_box_end();
            }
        }

        // Propagate any error
        result?;
        Ok(slf)
    }

    /// Create a SpanBuilder for fluent rich text creation
    /// Example: doc.span("Hello").bold().end()
    #[staticmethod]
    fn span(text: &str) -> SpanBuilder {
        SpanBuilder::new(text)
    }

    /// Set text alignment
    fn align(slf: Py<Self>, py: Python<'_>, align: TextAlign) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {}
            DocumentInner::Layout(layout) => {
                layout.align(align.into());
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Move cursor down by specified amount
    fn move_down(slf: Py<Self>, py: Python<'_>, amount: f64) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "move_down() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                layout.move_down(amount);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Move cursor up by specified amount
    fn move_up(slf: Py<Self>, py: Python<'_>, amount: f64) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "move_up() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                layout.move_up(amount);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Move cursor to position (y from bounds bottom, Prawn-style)
    fn move_cursor_to(slf: Py<Self>, py: Python<'_>, y: f64) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "move_cursor_to() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                layout.move_cursor_to(y);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Get cursor position (Y relative to bounds.bottom, Prawn-style)
    fn cursor(&self) -> PyResult<f64> {
        let guard = self.inner.lock().unwrap();
        match &*guard {
            DocumentInner::Basic(_) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "cursor() requires margin.",
            )),
            DocumentInner::Layout(layout) => Ok(layout.cursor()),
            DocumentInner::Consumed => Ok(0.0),
        }
    }

    /// Get bounds absolute bottom (for converting cursor to absolute coordinates)
    fn bounds_bottom(&self) -> PyResult<f64> {
        let guard = self.inner.lock().unwrap();
        match &*guard {
            DocumentInner::Basic(_) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "bounds_bottom() requires margin.",
            )),
            DocumentInner::Layout(layout) => Ok(layout.bounds().absolute_bottom()),
            DocumentInner::Consumed => Ok(0.0),
        }
    }

    /// Get bounds absolute left
    fn bounds_left(&self) -> PyResult<f64> {
        let guard = self.inner.lock().unwrap();
        match &*guard {
            DocumentInner::Basic(_) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "bounds_left() requires margin.",
            )),
            DocumentInner::Layout(layout) => Ok(layout.bounds().absolute_left()),
            DocumentInner::Consumed => Ok(0.0),
        }
    }

    /// Get bounds width
    fn bounds_width(&self) -> PyResult<f64> {
        let guard = self.inner.lock().unwrap();
        match &*guard {
            DocumentInner::Basic(_) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "bounds_width() requires margin.",
            )),
            DocumentInner::Layout(layout) => Ok(layout.bounds().width()),
            DocumentInner::Consumed => Ok(0.0),
        }
    }

    /// Get bounds height
    fn bounds_height(&self) -> PyResult<f64> {
        let guard = self.inner.lock().unwrap();
        match &*guard {
            DocumentInner::Basic(_) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "bounds_height() requires margin.",
            )),
            DocumentInner::Layout(layout) => Ok(layout.bounds().height()),
            DocumentInner::Consumed => Ok(0.0),
        }
    }

    /// Set cursor position (y relative to bounds.bottom, Prawn-style)
    ///
    /// The y value is measured from the bottom of the current bounds,
    /// with Y increasing upward (same coordinate system as cursor()).
    fn set_cursor(slf: Py<Self>, py: Python<'_>, y: f64) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "set_cursor() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                layout.set_cursor(y);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Draw text in a box with overflow handling
    ///
    /// Args:
    ///     text: The text to draw
    ///     point: (x, y) offset from current position
    ///     width: Box width
    ///     height: Box height (minimum height for Expand mode)
    ///     overflow: Overflow mode (Truncate, Expand)
    ///     shrink_to_fit: If set, shrink font to fit with this minimum size
    #[pyo3(signature = (text, point, width, height, overflow=None, shrink_to_fit=None))]
    fn text_box(
        &self,
        text: &str,
        point: (f64, f64),
        width: f64,
        height: f64,
        overflow: Option<Overflow>,
        shrink_to_fit: Option<f64>,
    ) -> PyResult<TextBoxResult> {
        let mut guard = self.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "text_box() requires margin.",
            )),
            DocumentInner::Layout(layout) => {
                let rust_overflow = if let Some(min_size) = shrink_to_fit {
                    crate::api::layout::Overflow::ShrinkToFit(min_size)
                } else {
                    match overflow.unwrap_or(Overflow::Truncate) {
                        Overflow::Truncate => crate::api::layout::Overflow::Truncate,
                        Overflow::Expand => crate::api::layout::Overflow::Expand,
                    }
                };

                let result =
                    layout.text_box(text, [point.0, point.1], width, height, rust_overflow);

                Ok(TextBoxResult {
                    height: result.height,
                    truncated: result.truncated,
                    font_size: result.font_size,
                    lines_rendered: result.lines_rendered,
                    total_lines: result.total_lines,
                })
            }
            DocumentInner::Consumed => Ok(TextBoxResult {
                height: 0.0,
                truncated: false,
                font_size: 0.0,
                lines_rendered: 0,
                total_lines: 0,
            }),
        }
    }

    /// Stroke the bounds of current bounding box
    fn stroke_bounds(slf: Py<Self>, py: Python<'_>) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "stroke_bounds() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                layout.stroke_bounds();
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    // =========== Grid methods ===========

    /// Define a grid layout
    #[pyo3(signature = (rows, columns, gutter=None, row_gutter=None, column_gutter=None))]
    fn define_grid(
        slf: Py<Self>,
        py: Python<'_>,
        rows: usize,
        columns: usize,
        gutter: Option<f64>,
        row_gutter: Option<f64>,
        column_gutter: Option<f64>,
    ) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "define_grid() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                let mut opts = RustGridOptions::new(rows, columns);
                if let Some(g) = gutter {
                    opts = opts.gutter(g);
                }
                if let Some(rg) = row_gutter {
                    opts = opts.row_gutter(rg);
                }
                if let Some(cg) = column_gutter {
                    opts = opts.column_gutter(cg);
                }
                layout.define_grid(opts);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Get grid cell info
    fn grid(&self, row: usize, column: usize) -> PyResult<Option<GridBox>> {
        let guard = self.inner.lock().unwrap();
        match &*guard {
            DocumentInner::Layout(layout) => Ok(layout.grid(row, column).map(|gb| GridBox {
                row: gb.row,
                column: gb.column,
                width: gb.width,
                height: gb.height,
                left: gb.left,
                top: gb.top,
            })),
            _ => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "grid() requires margin and define_grid().",
            )),
        }
    }

    /// Create a bounding box at grid cell (returns context manager)
    fn grid_cell(
        slf: Py<Self>,
        py: Python<'_>,
        row: usize,
        column: usize,
    ) -> PyResult<BoundingBoxContext> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();

        let (old_cursor, fixed_height) = if let DocumentInner::Layout(layout) = &mut *guard {
            // Get grid cell info
            let grid_box = layout.grid(row, column).ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Grid cell not found")
            })?;

            // Push bounding box for this cell using absolute coordinates
            let bounds = layout.bounds();
            let abs_x = bounds.absolute_left() + grid_box.left;
            let abs_y = bounds.absolute_top() - (bounds.height() - grid_box.top);
            let height = Some(grid_box.height);
            let old_cursor =
                layout.push_bounding_box_absolute(abs_x, abs_y, grid_box.width, height);
            (old_cursor, height)
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "grid_cell() requires margin and define_grid().",
            ));
        };
        drop(guard);
        drop(borrowed);

        Ok(BoundingBoxContext {
            doc: slf,
            old_cursor,
            fixed_height,
        })
    }

    /// Create a bounding box spanning multiple grid cells (returns context manager)
    fn grid_span(
        slf: Py<Self>,
        py: Python<'_>,
        start: (usize, usize),
        end: (usize, usize),
    ) -> PyResult<BoundingBoxContext> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();

        let (old_cursor, fixed_height) = if let DocumentInner::Layout(layout) = &mut *guard {
            let multi_box = layout.grid_span(start, end).ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Grid span not found")
            })?;

            let bounds = layout.bounds();
            let abs_x = bounds.absolute_left() + multi_box.left;
            let abs_y = bounds.absolute_top() - (bounds.height() - multi_box.top);
            let height = Some(multi_box.height);
            let old_cursor =
                layout.push_bounding_box_absolute(abs_x, abs_y, multi_box.width, height);
            (old_cursor, height)
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "grid_span() requires margin and define_grid().",
            ));
        };
        drop(guard);
        drop(borrowed);

        Ok(BoundingBoxContext {
            doc: slf,
            old_cursor,
            fixed_height,
        })
    }

    // =========== Table methods ===========

    /// Create and draw a table
    #[pyo3(signature = (data, options=None))]
    fn table(
        slf: Py<Self>,
        py: Python<'_>,
        data: Vec<Vec<String>>,
        options: Option<TableOptions>,
    ) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "table() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                let rust_opts = options.as_ref().map(|o| o.into()).unwrap_or_default();
                // Convert Vec<Vec<String>> to &[&[&str]] compatible format
                let data_refs: Vec<Vec<&str>> = data
                    .iter()
                    .map(|row| row.iter().map(|s| s.as_str()).collect())
                    .collect();
                let data_slices: Vec<&[&str]> =
                    data_refs.iter().map(|row| row.as_slice()).collect();
                layout.table(&data_slices, rust_opts);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    // =========== Page methods ===========

    /// Start a new page
    #[pyo3(signature = (page_size=None))]
    fn new_page(slf: Py<Self>, py: Python<'_>, page_size: Option<PageSize>) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.start_new_page();
                if let Some(size) = page_size {
                    doc.page_size(size.inner);
                }
            }
            DocumentInner::Layout(layout) => {
                layout.start_new_page();
                if let Some(size) = page_size {
                    layout.inner_mut().page_size(size.inner);
                }
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Get current page number (1-based)
    fn page_number(&self) -> usize {
        let guard = self.inner.lock().unwrap();
        match &*guard {
            DocumentInner::Basic(doc) => doc.page_number(),
            DocumentInner::Layout(layout) => layout.inner().page_number(),
            DocumentInner::Consumed => 0,
        }
    }

    /// Go to page (manual, no context manager)
    fn go_to_page(slf: Py<Self>, py: Python<'_>, index: usize) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.go_to_page(index);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().go_to_page(index);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Go to page with context manager
    fn page(slf: Py<Self>, py: Python<'_>, index: usize) -> PyResult<PageContext> {
        let borrowed = slf.borrow(py);
        let original_page = {
            let guard = borrowed.inner.lock().unwrap();
            match &*guard {
                DocumentInner::Basic(doc) => doc.page_number().saturating_sub(1),
                DocumentInner::Layout(layout) => layout.inner().page_number().saturating_sub(1),
                DocumentInner::Consumed => 0,
            }
        };

        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.go_to_page(index);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().go_to_page(index);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);

        Ok(PageContext {
            doc: slf,
            original_page,
        })
    }

    // =========== Image methods ===========

    /// Draw image at absolute position
    ///
    /// Args:
    ///     path: Path to the image file
    ///     pos: Position (x, y) for bottom-left corner of image
    ///     width: Explicit width (optional)
    ///     height: Explicit height (optional)
    ///     fit: Tuple (max_width, max_height) to fit image within bounds preserving aspect ratio.
    ///          When using fit, pos is the bottom-left corner of the available area,
    ///          and the image will be centered within that area.
    #[pyo3(signature = (path, pos, width=None, height=None, fit=None))]
    fn image_at(
        slf: Py<Self>,
        py: Python<'_>,
        path: &str,
        pos: (f64, f64),
        width: Option<f64>,
        height: Option<f64>,
        fit: Option<(f64, f64)>,
    ) -> PyResult<Py<Self>> {
        let data = std::fs::read(path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();

        // Helper to get inner document
        let doc = match &mut *guard {
            DocumentInner::Basic(doc) => doc,
            DocumentInner::Layout(layout) => layout.inner_mut(),
            DocumentInner::Consumed => {
                drop(guard);
                drop(borrowed);
                return Ok(slf);
            }
        };

        if let Some((max_w, max_h)) = fit {
            // Use image_with for fit support - it calculates dimensions and centers the image
            let mut opts = crate::api::image::ImageOptions::default();
            opts.fit = Some((max_w, max_h));
            // The position is the bottom-left of the available area
            opts.at = Some([pos.0, pos.1]);

            doc.image_with(data.as_slice(), opts)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        } else {
            // Use explicit dimensions
            let w = width.unwrap_or(100.0);
            let h = height.unwrap_or(100.0);
            doc.image(data.as_slice(), [pos.0, pos.1], w, h)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        }

        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Draw image at cursor (requires margin)
    ///
    /// Args:
    ///     path: Path to the image file
    ///     width: Explicit width (overrides fit)
    ///     height: Explicit height (overrides fit)
    ///     fit: Tuple (max_width, max_height) to fit image within bounds preserving aspect ratio
    #[pyo3(signature = (path, width=None, height=None, fit=None))]
    fn image(
        slf: Py<Self>,
        py: Python<'_>,
        path: &str,
        width: Option<f64>,
        height: Option<f64>,
        fit: Option<(f64, f64)>,
    ) -> PyResult<Py<Self>> {
        let data = std::fs::read(path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "image() requires margin. Use image_at() for absolute positioning.",
                ));
            }
            DocumentInner::Layout(layout) => {
                // Get absolute cursor position and bounds
                // cursor() returns relative to bounds.bottom, need absolute for image_with
                let bounds = layout.bounds();
                let abs_cursor_y = bounds.absolute_bottom() + layout.cursor();
                let x = bounds.absolute_left();

                // Create options with absolute position at cursor
                let mut opts = crate::api::image::ImageOptions::default();
                opts.at = Some([x, abs_cursor_y]);

                // Handle fit vs explicit dimensions
                if let Some((max_w, max_h)) = fit {
                    opts.fit = Some((max_w, max_h));
                }
                if let Some(w) = width {
                    opts.width = Some(w);
                }
                if let Some(h) = height {
                    opts.height = Some(h);
                }

                let embedded = layout
                    .inner_mut()
                    .image_with(data.as_slice(), opts)
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                    })?;

                // Move cursor down by actual rendered image height
                // Priority: explicit height > actual rendered height from embedded
                let move_height = height.unwrap_or(embedded.height as f64);
                layout.move_down(move_height);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    // =========== SVG methods ===========

    /// Draw SVG content at position
    ///
    /// Args:
    ///     svg: SVG content as string
    ///     pos: Position (x, y) for bottom-left corner
    ///     width: Target width
    ///     height: Target height
    #[cfg(feature = "svg")]
    fn draw_svg(
        slf: Py<Self>,
        py: Python<'_>,
        svg: &str,
        pos: (f64, f64),
        width: f64,
        height: f64,
    ) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.draw_svg(svg, [pos.0, pos.1], width, height)
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                    })?;
            }
            DocumentInner::Layout(layout) => {
                layout
                    .inner_mut()
                    .draw_svg(svg, [pos.0, pos.1], width, height)
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                    })?;
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    // =========== Link methods ===========

    /// Create a text link (URL, internal destination, or page)
    ///
    /// Args:
    ///     text: The link text to display
    ///     url: URL for external links
    ///     dest: Named destination for internal links
    ///     page: Page index (0-based) for page links
    #[pyo3(signature = (text, url=None, dest=None, page=None))]
    fn link(
        slf: Py<Self>,
        py: Python<'_>,
        text: &str,
        url: Option<&str>,
        dest: Option<&str>,
        page: Option<usize>,
    ) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "link() requires margin.",
                ));
            }
            DocumentInner::Layout(layout) => {
                if let Some(url) = url {
                    layout.text_link(text, url);
                } else if let Some(dest_name) = dest {
                    layout.text_link_dest(text, dest_name);
                } else if let Some(page_idx) = page {
                    // Create a temporary destination name for the page
                    let dest_name = format!("__page_{}", page_idx);
                    // Add destination at top of page if not exists
                    layout.add_dest(&dest_name, page_idx, crate::api::link::DestinationFit::Fit);
                    layout.text_link_dest(text, &dest_name);
                } else {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "link() requires url, dest, or page parameter",
                    ));
                }
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Add a named destination at the current cursor position
    ///
    /// Named destinations allow creating internal links within the document.
    /// Use `link(text, dest="name")` to create a clickable link to this destination.
    fn add_dest(slf: Py<Self>, py: Python<'_>, name: &str) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.add_dest_here(name, crate::api::link::DestinationFit::FitH(None));
            }
            DocumentInner::Layout(layout) => {
                layout.add_dest_here(name);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    /// Add a named destination at a specific page
    #[pyo3(signature = (name, page, y=None))]
    fn add_dest_at(
        slf: Py<Self>,
        py: Python<'_>,
        name: &str,
        page: usize,
        y: Option<f64>,
    ) -> Py<Self> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let fit = match y {
            Some(y_pos) => crate::api::link::DestinationFit::FitH(Some(y_pos)),
            None => crate::api::link::DestinationFit::Fit,
        };
        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.add_dest(name, page, fit);
            }
            DocumentInner::Layout(layout) => {
                layout.add_dest(name, page, fit);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        slf
    }

    // =========== Outline (Bookmarks) ===========

    /// Add document outline/bookmarks
    ///
    /// Takes a list of outline items. Each item is a dict with:
    /// - title: str - The bookmark title
    /// - page: int - The destination page index (0-based)
    /// - children: list (optional) - Nested outline items
    ///
    /// Example:
    /// ```python
    /// doc.outline([
    ///     {"title": "Chapter 1", "page": 0, "children": [
    ///         {"title": "Section 1.1", "page": 1},
    ///         {"title": "Section 1.2", "page": 2},
    ///     ]},
    ///     {"title": "Chapter 2", "page": 3},
    /// ])
    /// ```
    fn outline(slf: Py<Self>, py: Python<'_>, items: Vec<OutlineItem>) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();

        fn build_outline(builder: &mut crate::api::outline::OutlineBuilder, items: &[OutlineItem]) {
            for item in items {
                if item.children.is_empty() {
                    builder.page(&item.title, item.page);
                } else {
                    builder.section(&item.title, item.page, |b| {
                        build_outline(b, &item.children);
                    });
                }
            }
        }

        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.outline(|builder| {
                    build_outline(builder, &items);
                });
            }
            DocumentInner::Layout(layout) => {
                layout.outline(|builder| {
                    build_outline(builder, &items);
                });
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    // =========== Form fields ===========

    /// Add a text field
    #[pyo3(signature = (name, rect, multiline=false, value=None))]
    fn text_field(
        slf: Py<Self>,
        py: Python<'_>,
        name: &str,
        rect: (f64, f64, f64, f64),
        multiline: bool,
        value: Option<&str>,
    ) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let rect_arr = [rect.0, rect.1, rect.2, rect.3];

        match &mut *guard {
            DocumentInner::Basic(doc) => {
                if multiline {
                    doc.add_text_area(name, rect_arr);
                } else if let Some(v) = value {
                    doc.add_text_field_with(name, rect_arr, |f| f.with_value(v));
                } else {
                    doc.add_text_field(name, rect_arr);
                }
            }
            DocumentInner::Layout(layout) => {
                if multiline {
                    layout.inner_mut().add_text_area(name, rect_arr);
                } else if let Some(v) = value {
                    layout
                        .inner_mut()
                        .add_text_field_with(name, rect_arr, |f| f.with_value(v));
                } else {
                    layout.inner_mut().add_text_field(name, rect_arr);
                }
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Add a checkbox
    #[pyo3(signature = (name, rect, checked=false))]
    fn checkbox(
        slf: Py<Self>,
        py: Python<'_>,
        name: &str,
        rect: (f64, f64, f64, f64),
        checked: bool,
    ) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let rect_arr = [rect.0, rect.1, rect.2, rect.3];

        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.add_checkbox(name, rect_arr, checked);
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().add_checkbox(name, rect_arr, checked);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    /// Add a dropdown/combo box
    #[pyo3(signature = (name, rect, options, value=None))]
    fn dropdown(
        slf: Py<Self>,
        py: Python<'_>,
        name: &str,
        rect: (f64, f64, f64, f64),
        options: Vec<String>,
        value: Option<&str>,
    ) -> PyResult<Py<Self>> {
        let borrowed = slf.borrow(py);
        let mut guard = borrowed.inner.lock().unwrap();
        let rect_arr = [rect.0, rect.1, rect.2, rect.3];

        match &mut *guard {
            DocumentInner::Basic(doc) => {
                doc.add_dropdown(name, rect_arr, options);
                // Set value if provided (dropdown has first option selected by default)
                if value.is_some() {
                    // Note: We'd need to access the form to modify the value
                    // For simplicity, the first option is already selected
                }
            }
            DocumentInner::Layout(layout) => {
                layout.inner_mut().add_dropdown(name, rect_arr, options);
            }
            DocumentInner::Consumed => {}
        }
        drop(guard);
        drop(borrowed);
        Ok(slf)
    }

    // =========== Output methods ===========

    /// Render to bytes
    fn render(&self) -> PyResult<Vec<u8>> {
        let mut guard = self.inner.lock().unwrap();
        let inner = std::mem::replace(&mut *guard, DocumentInner::Consumed);
        match inner {
            DocumentInner::Basic(mut doc) => doc
                .render()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())),
            DocumentInner::Layout(layout) => {
                let mut doc = layout.into_inner();
                doc.render()
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
            }
            DocumentInner::Consumed => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Document already rendered",
            )),
        }
    }

    /// Save to file
    fn save(&self, path: &str) -> PyResult<()> {
        let bytes = self.render()?;
        std::fs::write(path, bytes)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
    }
}

// ============================================================================
// Module definition
// ============================================================================

#[pymodule]
fn pdfcrate(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Document>()?;
    m.add_class::<Color>()?;
    m.add_class::<Margin>()?;
    m.add_class::<PageSize>()?;
    m.add_class::<TextAlign>()?;
    m.add_class::<VerticalAlign>()?;
    m.add_class::<Overflow>()?;
    m.add_class::<TextBoxResult>()?;
    m.add_class::<TransparentContext>()?;
    m.add_class::<IndentContext>()?;
    m.add_class::<FloatContext>()?;
    m.add_class::<FontContext>()?;
    m.add_class::<BoundingBoxContext>()?;
    m.add_class::<PageContext>()?;
    m.add_class::<Table>()?;
    m.add_class::<TableOptions>()?;
    m.add_class::<CellStyle>()?;
    m.add_class::<Cell>()?;
    m.add_class::<TextFragment>()?;
    m.add_class::<SpanBuilder>()?;
    m.add_class::<GridBox>()?;
    m.add_class::<OutlineItem>()?;
    m.add_class::<EmbeddedImage>()?;
    m.add_class::<EmbeddedFont>()?;
    Ok(())
}
