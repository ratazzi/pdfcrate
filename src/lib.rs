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

pub mod objects;
pub mod parser;
pub mod writer;
pub mod codec;
pub mod document;
pub mod content;
pub mod font;
pub mod api;

#[cfg(feature = "png")]
pub mod image;

mod error;

pub use error::{Error, Result};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::api::Document;
    pub use crate::api::page::{PageSize, PageLayout};
    pub use crate::error::{Error, Result};
}
