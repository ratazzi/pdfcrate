//! pdfcrate - A Rust library for creating and manipulating PDF documents
//!
//! # Example
//!
//! ```rust,no_run
//! use pdfcrate::prelude::*;
//!
//! fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//!     Document::generate("hello.pdf", |doc| {
//!         doc.text_at("Hello, World!", [72.0, 700.0]);
//!         Ok(())
//!     })?;
//!     Ok(())
//! }
//! ```
//!
//! # WASM Support
//!
//! This library supports WebAssembly targets. When compiled for WASM:
//! - Use `Document::new()` and `doc.render()` to get PDF bytes
//! - File I/O methods (`save`, `generate`) require the `std` feature
//! - The `WasmDocument` wrapper provides JavaScript-friendly bindings

pub mod api;
pub mod codec;
pub mod content;
pub mod document;
pub mod font;
pub mod forms;
pub mod objects;
pub mod parser;
pub mod writer;

pub mod image;
#[cfg(feature = "svg")]
pub mod svg;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

mod error;

pub use error::{Error, Result};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::api::image::{EmbeddedImage, ImageOptions, ImageSource, Position};
    pub use crate::api::layout::{
        BoundingBox, Color, LayoutDocument, Margin, Overflow, PageNumberConfig, PageNumberPosition,
        RepeaterPages, TextAlign, TextBoxResult,
    };
    pub use crate::api::link::{
        DestinationFit, HighlightMode, LinkAction, LinkAnnotation, LinkDestination,
    };
    pub use crate::api::measurements::{
        cm, inch, mm, pt2cm, pt2inch, pt2mm, Cm, Inch, Measurement, Mm, Pt,
    };
    pub use crate::api::page::{PageLayout, PageSize};
    pub use crate::api::table::{
        BorderLine, Cell, CellContent, CellSelection, CellStyle, ColumnWidths, ImageContent,
        ImageFit, IntoCell, RangeBoundsExt, SubtableData, Table, TableOptions, TablePosition,
        TextOverflow, VerticalAlign,
    };
    pub use crate::api::Document;
    pub use crate::content::{LineCap, LineJoin};
    pub use crate::document::{EmbeddedPage, LoadedDocument};
    pub use crate::error::{Error, Result};
    pub use crate::forms::TextAlign as FormTextAlign;
    pub use crate::forms::{AcroForm, FieldFlags, FieldType, FormField}; // Rename to avoid conflict

    #[cfg(feature = "fonts")]
    pub use crate::font::EmbeddedFont;
}
