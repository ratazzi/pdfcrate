//! pdf_rs - A Rust library for creating and manipulating PDF documents
//!
//! # Example
//!
//! ```rust,no_run
//! use pdf_rs::prelude::*;
//!
//! fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//!     Document::generate("hello.pdf", |doc| {
//!         doc.text("Hello, World!");
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
pub mod objects;
pub mod parser;
pub mod writer;

#[cfg(feature = "png")]
pub mod image;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

mod error;

pub use error::{Error, Result};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::api::image::{EmbeddedImage, ImageOptions, Position};
    pub use crate::api::page::{PageLayout, PageSize};
    pub use crate::api::Document;
    pub use crate::content::{LineCap, LineJoin};
    pub use crate::document::LoadedDocument;
    pub use crate::error::{Error, Result};

    #[cfg(feature = "fonts")]
    pub use crate::font::EmbeddedFont;
}
