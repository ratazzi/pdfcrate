//! PDF Writer
//!
//! Serializes PDF objects to bytes.

use std::io::{self, Write};

use crate::objects::{format_real, PdfDict, PdfObject, PdfRef, PdfStream};
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
                self.write_str(&format_real(*r))?;
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
        let max_obj = entries
            .iter()
            .map(|(r, _)| r.object_number())
            .max()
            .unwrap_or(0);

        self.write_str(&format!("0 {}\n", max_obj + 1))?;

        // Build list of free object numbers for linked list
        let used_obj_nums: std::collections::HashSet<u32> =
            entries.iter().map(|(r, _)| r.object_number()).collect();
        let mut free_obj_nums: Vec<u32> = (1..=max_obj)
            .filter(|n| !used_obj_nums.contains(n))
            .collect();
        free_obj_nums.push(0); // Append 0 to end the free list (circular back to head)

        // Object 0 is always free, points to first free object (or 0 if none)
        // Each xref entry must be exactly 20 bytes: "nnnnnnnnnn ggggg n\r\n"
        let first_free = free_obj_nums.first().copied().unwrap_or(0);
        self.write_str(&format!("{:010} 65535 f\r\n", first_free))?;

        // Write in-use and free entries
        for obj_num in 1..=max_obj {
            if let Some((r, offset)) = entries.iter().find(|(r, _)| r.object_number() == obj_num) {
                // In-use entry: use actual generation from PdfRef
                self.write_str(&format!("{:010} {:05} n\r\n", offset, r.generation()))?;
            } else {
                // Free entry: point to next free object in the list
                let next_free = free_obj_nums
                    .iter()
                    .find(|&&n| n > obj_num)
                    .copied()
                    .unwrap_or(0);
                self.write_str(&format!("{:010} {:05} f\r\n", next_free, 0))?;
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
    // Size must be max object number + 1 (not count of objects, in case of sparse IDs)
    let max_obj_num = objects
        .iter()
        .map(|(r, _)| r.object_number())
        .max()
        .unwrap_or(0);
    let mut trailer = PdfDict::new();
    trailer.set("Size", PdfObject::Integer((max_obj_num + 1) as i64));
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

    #[test]
    fn test_xref_entry_line_endings() {
        // PDF spec requires xref entries to use CRLF and be exactly 20 bytes
        let mut buffer = Vec::new();
        let mut writer = PdfWriter::new(&mut buffer);

        // Write an object to populate xref
        let obj_ref = PdfRef::new(1);
        writer
            .write_indirect_object(obj_ref, &PdfObject::Null)
            .unwrap();

        writer.write_xref().unwrap();

        let xref_str = String::from_utf8_lossy(&buffer);

        // Find xref entries (after "xref\n0 2\n")
        if let Some(pos) = xref_str.find("xref\n") {
            let after_xref = &xref_str[pos + 5..];
            // Skip subsection header "0 2\n"
            if let Some(newline_pos) = after_xref.find('\n') {
                let entries_start = newline_pos + 1;
                let entries = &after_xref[entries_start..];

                // Each entry should be exactly 20 bytes with CRLF
                // Format: "nnnnnnnnnn ggggg n \r\n" or "nnnnnnnnnn ggggg f \r\n"
                let lines: Vec<&str> = entries.split("\r\n").collect();
                // Should have at least 2 entries (object 0 free + object 1 in-use)
                assert!(lines.len() >= 2, "Expected at least 2 xref entries");

                // Verify first entry (object 0, free)
                // Format: "nnnnnnnnnn ggggg f" = 18 chars (without CRLF)
                assert_eq!(lines[0].len(), 18, "Entry without CRLF should be 18 chars");
                assert!(lines[0].ends_with(" f"), "Object 0 should be free entry");

                // Verify second entry (object 1, in-use)
                assert_eq!(lines[1].len(), 18, "Entry without CRLF should be 18 chars");
                assert!(lines[1].ends_with(" n"), "Object 1 should be in-use entry");
            }
        }
    }

    #[test]
    fn test_xref_free_list_chain() {
        // Test that free entries form a proper linked list
        let mut buffer = Vec::new();
        let mut writer = PdfWriter::new(&mut buffer);

        // Write objects 1, 3, 5 (leaving 2, 4 as free)
        writer
            .write_indirect_object(PdfRef::new(1), &PdfObject::Null)
            .unwrap();
        writer
            .write_indirect_object(PdfRef::new(3), &PdfObject::Null)
            .unwrap();
        writer
            .write_indirect_object(PdfRef::new(5), &PdfObject::Null)
            .unwrap();

        writer.write_xref().unwrap();

        let xref_str = String::from_utf8_lossy(&buffer);

        // Parse xref entries
        if let Some(pos) = xref_str.find("xref\n") {
            let after_xref = &xref_str[pos + 5..];
            if let Some(newline_pos) = after_xref.find('\n') {
                let entries_start = newline_pos + 1;
                let entries = &after_xref[entries_start..];
                let lines: Vec<&str> = entries.split("\r\n").filter(|s| !s.is_empty()).collect();

                // Should have 6 entries (0-5)
                assert_eq!(lines.len(), 6, "Expected 6 xref entries");

                // Object 0 (free, head of list) should point to first free object (2)
                assert!(
                    lines[0].starts_with("0000000002"),
                    "Object 0 should point to 2"
                );
                assert!(lines[0].ends_with(" f"), "Object 0 should be free");

                // Object 2 (free) should point to next free object (4)
                assert!(
                    lines[2].starts_with("0000000004"),
                    "Object 2 should point to 4"
                );
                assert!(lines[2].ends_with(" f"), "Object 2 should be free");

                // Object 4 (free) should point back to 0 (end of list)
                assert!(
                    lines[4].starts_with("0000000000"),
                    "Object 4 should point to 0"
                );
                assert!(lines[4].ends_with(" f"), "Object 4 should be free");

                // In-use entries (1, 3, 5) should have 'n' marker
                assert!(lines[1].ends_with(" n"), "Object 1 should be in-use");
                assert!(lines[3].ends_with(" n"), "Object 3 should be in-use");
                assert!(lines[5].ends_with(" n"), "Object 5 should be in-use");
            }
        }
    }

    #[test]
    fn test_xref_generation_number() {
        // Test that in-use entries use the correct generation from PdfRef
        let mut buffer = Vec::new();
        let mut writer = PdfWriter::new(&mut buffer);

        // Write object with non-zero generation
        let obj_ref = PdfRef::with_generation(1, 5);
        writer
            .write_indirect_object(obj_ref, &PdfObject::Null)
            .unwrap();

        writer.write_xref().unwrap();

        let xref_str = String::from_utf8_lossy(&buffer);

        // Find the in-use entry for object 1
        if let Some(pos) = xref_str.find("xref\n") {
            let after_xref = &xref_str[pos + 5..];
            if let Some(newline_pos) = after_xref.find('\n') {
                let entries_start = newline_pos + 1;
                let entries = &after_xref[entries_start..];
                let lines: Vec<&str> = entries.split("\r\n").filter(|s| !s.is_empty()).collect();

                // Object 1 entry should have generation 5
                // Format: "nnnnnnnnnn 00005 n"
                assert!(
                    lines[1].ends_with(" 00005 n"),
                    "Object 1 should have generation 5, got: {}",
                    lines[1]
                );
            }
        }
    }

    #[test]
    fn test_trailer_size_with_sparse_ids() {
        // Test that trailer /Size uses max object number, not count
        let objects = vec![
            (PdfRef::new(1), PdfObject::Null),
            (PdfRef::new(5), PdfObject::Null), // Sparse: skip 2,3,4
            (PdfRef::new(10), PdfObject::Null), // Sparse: skip 6,7,8,9
        ];

        let result = write_pdf(&objects, PdfRef::new(1), None).unwrap();
        let pdf_str = String::from_utf8_lossy(&result);

        // /Size should be 11 (max object 10 + 1), not 4 (3 objects + 1)
        assert!(
            pdf_str.contains("/Size 11"),
            "Trailer /Size should be 11 for max object 10, got: {}",
            pdf_str
        );
    }
}
