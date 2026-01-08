//! PDF Writer
//!
//! Serializes PDF objects to bytes.

use std::io::{self, Write};

use crate::objects::{PdfDict, PdfObject, PdfRef, PdfStream};
use crate::Result;

/// PDF Writer
///
/// Writes PDF objects to an output stream.
pub struct PdfWriter<W: Write> {
    writer: W,
    /// Current byte offset
    offset: usize,
    /// Object offsets for xref table
    xref: Vec<(PdfRef, usize)>,
}

impl<W: Write> PdfWriter<W> {
    /// Creates a new PDF writer
    pub fn new(writer: W) -> Self {
        PdfWriter {
            writer,
            offset: 0,
            xref: Vec::new(),
        }
    }

    /// Returns the current byte offset
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the xref entries
    pub fn xref_entries(&self) -> &[(PdfRef, usize)] {
        &self.xref
    }

    /// Writes bytes to the output
    pub fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)?;
        self.offset += bytes.len();
        Ok(())
    }

    /// Writes a string to the output
    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        self.write_bytes(s.as_bytes())
    }

    /// Writes a newline
    pub fn write_newline(&mut self) -> io::Result<()> {
        self.write_bytes(b"\n")
    }

    /// Writes the PDF header
    pub fn write_header(&mut self, major: u8, minor: u8) -> io::Result<()> {
        self.write_str(&format!("%PDF-{}.{}\n", major, minor))?;
        // Binary comment to indicate this is a binary file
        self.write_bytes(&[b'%', 0xE2, 0xE3, 0xCF, 0xD3, b'\n'])?;
        Ok(())
    }

    /// Writes an indirect object
    pub fn write_indirect_object(&mut self, obj_ref: PdfRef, object: &PdfObject) -> Result<()> {
        // Record xref entry
        self.xref.push((obj_ref, self.offset));

        // Write object header
        self.write_str(&format!(
            "{} {} obj\n",
            obj_ref.object_number(),
            obj_ref.generation()
        ))?;

        // Write object
        self.write_object(object)?;

        // Write object footer
        self.write_str("\nendobj\n")?;

        Ok(())
    }

    /// Writes a PDF object
    pub fn write_object(&mut self, object: &PdfObject) -> Result<()> {
        match object {
            PdfObject::Null => self.write_str("null")?,
            PdfObject::Bool(b) => self.write_str(if *b { "true" } else { "false" })?,
            PdfObject::Integer(i) => self.write_str(&i.to_string())?,
            PdfObject::Real(r) => {
                // Format real numbers efficiently
                if r.fract() == 0.0 && r.abs() < i64::MAX as f64 {
                    self.write_str(&format!("{:.1}", r))?;
                } else {
                    let s = format!("{:.6}", r);
                    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
                    self.write_str(if trimmed.is_empty() { "0" } else { trimmed })?;
                }
            }
            PdfObject::Name(name) => self.write_str(&name.encode())?,
            PdfObject::String(s) => self.write_str(&s.encode())?,
            PdfObject::HexString(s) => self.write_str(&s.encode())?,
            PdfObject::Array(arr) => {
                self.write_str("[")?;
                for (i, obj) in arr.iter().enumerate() {
                    if i > 0 {
                        self.write_str(" ")?;
                    }
                    self.write_object(obj)?;
                }
                self.write_str("]")?;
            }
            PdfObject::Dict(dict) => {
                self.write_dict(dict)?;
            }
            PdfObject::Stream(stream) => {
                self.write_stream(stream)?;
            }
            PdfObject::Reference(r) => {
                self.write_str(&format!("{} {} R", r.object_number(), r.generation()))?;
            }
        }
        Ok(())
    }

    /// Writes a dictionary
    fn write_dict(&mut self, dict: &PdfDict) -> Result<()> {
        self.write_str("<<")?;
        for (key, value) in dict.iter() {
            self.write_str(&key.encode())?;
            self.write_str(" ")?;
            self.write_object(value)?;
            self.write_str(" ")?;
        }
        self.write_str(">>")?;
        Ok(())
    }

    /// Writes a stream
    fn write_stream(&mut self, stream: &PdfStream) -> Result<()> {
        self.write_dict(stream.dict())?;
        self.write_str("\nstream\n")?;
        self.write_bytes(stream.data())?;
        self.write_str("\nendstream")?;
        Ok(())
    }

    /// Writes the xref table
    pub fn write_xref(&mut self) -> io::Result<usize> {
        let xref_offset = self.offset;

        self.write_str("xref\n")?;

        // Sort xref entries by object number
        let mut entries = self.xref.clone();
        entries.sort_by_key(|(r, _)| r.object_number());

        // Write entries (simplified - assumes consecutive object numbers starting from 0)
        let max_obj = entries.iter().map(|(r, _)| r.object_number()).max().unwrap_or(0);

        self.write_str(&format!("0 {}\n", max_obj + 1))?;

        // Object 0 is always free
        self.write_str("0000000000 65535 f \n")?;

        // Write in-use entries
        for obj_num in 1..=max_obj {
            if let Some((_, offset)) = entries.iter().find(|(r, _)| r.object_number() == obj_num) {
                self.write_str(&format!("{:010} {:05} n \n", offset, 0))?;
            } else {
                // Free entry
                self.write_str("0000000000 65535 f \n")?;
            }
        }

        Ok(xref_offset)
    }

    /// Writes the trailer
    pub fn write_trailer(&mut self, trailer_dict: &PdfDict, xref_offset: usize) -> Result<()> {
        self.write_str("trailer\n")?;
        self.write_dict(trailer_dict)?;
        self.write_str(&format!("\nstartxref\n{}\n%%EOF\n", xref_offset))?;
        Ok(())
    }

    /// Flushes the writer
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    /// Consumes the writer and returns the inner writer
    pub fn into_inner(self) -> W {
        self.writer
    }
}

/// Writes a complete PDF document to bytes
pub fn write_pdf(
    objects: &[(PdfRef, PdfObject)],
    root_ref: PdfRef,
    info_ref: Option<PdfRef>,
) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new(&mut buffer);

    // Write header
    writer.write_header(1, 7)?;

    // Write objects
    for (obj_ref, object) in objects {
        writer.write_indirect_object(*obj_ref, object)?;
    }

    // Write xref
    let xref_offset = writer.write_xref()?;

    // Build trailer dict
    let mut trailer = PdfDict::new();
    trailer.set("Size", PdfObject::Integer((objects.len() + 1) as i64));
    trailer.set("Root", PdfObject::Reference(root_ref));
    if let Some(info) = info_ref {
        trailer.set("Info", PdfObject::Reference(info));
    }

    // Write trailer
    writer.write_trailer(&trailer, xref_offset)?;

    writer.flush()?;

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{PdfArray, PdfName};

    #[test]
    fn test_write_simple_objects() {
        let mut buffer = Vec::new();
        let mut writer = PdfWriter::new(&mut buffer);

        writer.write_object(&PdfObject::Null).unwrap();
        assert_eq!(buffer, b"null");

        buffer.clear();
        writer = PdfWriter::new(&mut buffer);
        writer.write_object(&PdfObject::Bool(true)).unwrap();
        assert_eq!(buffer, b"true");

        buffer.clear();
        writer = PdfWriter::new(&mut buffer);
        writer.write_object(&PdfObject::Integer(42)).unwrap();
        assert_eq!(buffer, b"42");
    }

    #[test]
    fn test_write_array() {
        let mut buffer = Vec::new();
        let mut writer = PdfWriter::new(&mut buffer);

        let mut arr = PdfArray::new();
        arr.push(PdfObject::Integer(1));
        arr.push(PdfObject::Integer(2));
        arr.push(PdfObject::Integer(3));

        writer.write_object(&PdfObject::Array(arr)).unwrap();
        assert_eq!(buffer, b"[1 2 3]");
    }

    #[test]
    fn test_write_dict() {
        let mut buffer = Vec::new();
        let mut writer = PdfWriter::new(&mut buffer);

        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::new("Page")));

        writer.write_object(&PdfObject::Dict(dict)).unwrap();
        let s = String::from_utf8(buffer).unwrap();
        assert!(s.contains("/Type"));
        assert!(s.contains("/Page"));
    }

    #[test]
    fn test_write_header() {
        let mut buffer = Vec::new();
        let mut writer = PdfWriter::new(&mut buffer);

        writer.write_header(1, 7).unwrap();
        let s = String::from_utf8_lossy(&buffer);
        assert!(s.starts_with("%PDF-1.7"));
    }
}
