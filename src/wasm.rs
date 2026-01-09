//! WASM bindings for pdf_rs
//!
//! This module provides JavaScript-friendly bindings for use in WASM environments.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// WASM-friendly PDF document wrapper
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmDocument {
    inner: crate::api::Document,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmDocument {
    /// Creates a new empty PDF document
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmDocument {
        WasmDocument {
            inner: crate::api::Document::new(),
        }
    }

    /// Sets the document title
    pub fn title(&mut self, title: &str) {
        self.inner.title(title);
    }

    /// Sets the document author
    pub fn author(&mut self, author: &str) {
        self.inner.author(author);
    }

    /// Adds text to the document
    pub fn text(&mut self, text: &str) {
        self.inner.text(text);
    }

    /// Adds text at a specific position
    #[wasm_bindgen(js_name = textAt)]
    pub fn text_at(&mut self, text: &str, x: f64, y: f64) {
        self.inner.text_at(text, [x, y]);
    }

    /// Sets the current font
    pub fn font(&mut self, name: &str, size: f64) {
        self.inner.font(name).size(size);
    }

    /// Starts a new page
    #[wasm_bindgen(js_name = newPage)]
    pub fn new_page(&mut self) {
        self.inner.start_new_page();
    }

    /// Sets page size to A4
    #[wasm_bindgen(js_name = pageSizeA4)]
    pub fn page_size_a4(&mut self) {
        self.inner.page_size(crate::api::PageSize::A4);
    }

    /// Sets page size to Letter
    #[wasm_bindgen(js_name = pageSizeLetter)]
    pub fn page_size_letter(&mut self) {
        self.inner.page_size(crate::api::PageSize::Letter);
    }

    /// Renders the document to bytes
    pub fn render(&mut self) -> Result<Vec<u8>, JsError> {
        self.inner
            .render()
            .map_err(|e| JsError::new(&e.to_string()))
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for WasmDocument {
    fn default() -> Self {
        WasmDocument::new()
    }
}
