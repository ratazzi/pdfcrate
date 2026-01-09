//! PDF Document structure
//!
//! This module handles the high-level PDF document structure.

mod context;
mod loader;

pub use context::PdfContext;
pub use loader::LoadedDocument;

use crate::objects::{PdfDict, PdfName, PdfObject, PdfRef};

/// Creates a new Catalog dictionary
pub fn create_catalog(pages_ref: PdfRef) -> PdfDict {
    let mut dict = PdfDict::new();
    dict.set("Type", PdfObject::Name(PdfName::catalog()));
    dict.set("Pages", PdfObject::Reference(pages_ref));
    dict
}

/// Creates a new Pages dictionary (page tree root)
pub fn create_pages(kids: Vec<PdfRef>, count: i64) -> PdfDict {
    let mut dict = PdfDict::new();
    dict.set("Type", PdfObject::Name(PdfName::pages()));

    let kids_array: Vec<PdfObject> = kids.into_iter().map(PdfObject::Reference).collect();
    dict.set("Kids", PdfObject::Array(kids_array.into()));
    dict.set("Count", PdfObject::Integer(count));

    dict
}

/// Creates a new Page dictionary
pub fn create_page(
    parent_ref: PdfRef,
    media_box: [f64; 4],
    resources: Option<PdfRef>,
    contents: Option<PdfRef>,
) -> PdfDict {
    let mut dict = PdfDict::new();
    dict.set("Type", PdfObject::Name(PdfName::page()));
    dict.set("Parent", PdfObject::Reference(parent_ref));

    // MediaBox: [left, bottom, right, top]
    let media_box_array: Vec<PdfObject> = media_box.iter().map(|&v| PdfObject::Real(v)).collect();
    dict.set("MediaBox", PdfObject::Array(media_box_array.into()));

    if let Some(res_ref) = resources {
        dict.set("Resources", PdfObject::Reference(res_ref));
    } else {
        // Empty resources dict
        dict.set("Resources", PdfObject::Dict(PdfDict::new()));
    }

    if let Some(content_ref) = contents {
        dict.set("Contents", PdfObject::Reference(content_ref));
    }

    dict
}
