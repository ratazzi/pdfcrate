//! PDF Parser
//!
//! This module handles parsing PDF documents from bytes.

mod lexer;
mod parser;

pub use lexer::Lexer;
pub use parser::Parser;

use crate::objects::PdfObject;
use crate::Result;

/// Parses a PDF object from bytes
pub fn parse_object(bytes: &[u8]) -> Result<PdfObject> {
    let mut lexer = Lexer::new(bytes);
    let mut parser = Parser::new(&mut lexer);
    parser.parse_object()
}
