//! XRef table parsing
//!
//! This module handles parsing PDF cross-reference (xref) tables and streams.

use crate::error::{Error, Result};
use crate::objects::{PdfDict, PdfObject, PdfRef};

const MAX_XREF_ENTRIES: usize = 5_000_000;
const MAX_XREF_FIELD_WIDTH: usize = 8;

/// An entry in the XRef table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XRefEntry {
    /// Free object (not in use)
    Free {
        /// Next free object number
        next_free: u32,
        /// Generation number
        generation: u16,
    },
    /// In-use object at a byte offset
    InUse {
        /// Byte offset in the file
        offset: u64,
        /// Generation number
        generation: u16,
    },
    /// Compressed object (stored in an object stream)
    Compressed {
        /// Object number of the containing object stream
        stream_obj: u32,
        /// Index within the object stream
        index: u32,
    },
}

/// Cross-reference table
#[derive(Debug, Default)]
pub struct XRefTable {
    /// Entries indexed by object number
    entries: Vec<Option<XRefEntry>>,
    /// Trailer dictionary
    trailer: Option<PdfDict>,
}

impl XRefTable {
    /// Creates a new empty XRef table
    pub fn new() -> Self {
        XRefTable {
            entries: Vec::new(),
            trailer: None,
        }
    }

    /// Gets an entry by object number
    pub fn get(&self, obj_num: u32) -> Option<&XRefEntry> {
        self.entries.get(obj_num as usize).and_then(|e| e.as_ref())
    }

    /// Sets an entry
    pub fn set(&mut self, obj_num: u32, entry: XRefEntry) {
        let idx = obj_num as usize;
        if idx >= self.entries.len() {
            self.entries.resize(idx + 1, None);
        }
        self.entries[idx] = Some(entry);
    }

    /// Gets the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Gets the trailer dictionary
    pub fn trailer(&self) -> Option<&PdfDict> {
        self.trailer.as_ref()
    }

    /// Sets the trailer dictionary
    pub fn set_trailer(&mut self, trailer: PdfDict) {
        self.trailer = Some(trailer);
    }

    /// Gets the catalog reference from the trailer
    pub fn catalog_ref(&self) -> Option<PdfRef> {
        self.trailer.as_ref()?.get_ref("Root")
    }

    /// Gets the info dictionary reference from the trailer
    pub fn info_ref(&self) -> Option<PdfRef> {
        self.trailer.as_ref()?.get_ref("Info")
    }

    /// Merges another xref table into this one (for incremental updates)
    /// Entries from the other table take precedence
    pub fn merge(&mut self, other: &XRefTable) {
        // Ensure we have enough capacity
        if other.entries.len() > self.entries.len() {
            self.entries.resize(other.entries.len(), None);
        }

        // Merge entries (newer table takes precedence)
        for (i, entry) in other.entries.iter().enumerate() {
            if entry.is_some() {
                self.entries[i] = *entry;
            }
        }

        // Update trailer with new values while preserving Prev chain
        if let Some(new_trailer) = &other.trailer {
            if let Some(ref mut existing) = self.trailer {
                // Merge trailer dictionaries, new takes precedence
                for (key, value) in new_trailer.iter() {
                    if key.as_str() != "Prev" {
                        existing.insert(key.clone(), value.clone());
                    }
                }
            } else {
                self.trailer = Some(new_trailer.clone());
            }
        }
    }
}

/// XRef parser
pub struct XRefParser<'a> {
    data: &'a [u8],
}

impl<'a> XRefParser<'a> {
    /// Creates a new XRef parser
    pub fn new(data: &'a [u8]) -> Self {
        XRefParser { data }
    }

    /// Finds the startxref value (byte offset to xref table)
    pub fn find_startxref(&self) -> Result<u64> {
        // Search backwards from end of file for "startxref"
        // PDF spec says it should be within the last 1024 bytes
        let search_len = std::cmp::min(self.data.len(), 1024);
        let search_start = self.data.len().saturating_sub(search_len);
        let search_data = &self.data[search_start..];

        // Look for "startxref" keyword
        let pattern = b"startxref";
        let mut pos = None;

        for i in (0..search_data.len().saturating_sub(pattern.len())).rev() {
            if &search_data[i..i + pattern.len()] == pattern {
                pos = Some(i);
                break;
            }
        }

        let pos = pos.ok_or_else(|| Error::Parse {
            message: "Could not find startxref".to_string(),
            position: self.data.len(),
        })?;

        // Skip "startxref" and whitespace
        let mut offset_start = search_start + pos + pattern.len();
        while offset_start < self.data.len() && is_whitespace(self.data[offset_start]) {
            offset_start += 1;
        }

        // Read the offset value
        let mut offset_end = offset_start;
        while offset_end < self.data.len() && self.data[offset_end].is_ascii_digit() {
            offset_end += 1;
        }

        let offset_str =
            std::str::from_utf8(&self.data[offset_start..offset_end]).map_err(|_| {
                Error::Parse {
                    message: "Invalid startxref offset encoding".to_string(),
                    position: offset_start,
                }
            })?;

        offset_str.parse::<u64>().map_err(|_| Error::Parse {
            message: format!("Invalid startxref offset: {}", offset_str),
            position: offset_start,
        })
    }

    /// Parses the complete xref table including all incremental updates
    pub fn parse_all(&self) -> Result<XRefTable> {
        let mut xref = XRefTable::new();
        let startxref = self.find_startxref()?;

        // Parse xref tables, following Prev links
        self.parse_xref_chain(startxref, &mut xref)?;

        Ok(xref)
    }

    /// Parses xref chain following Prev links
    fn parse_xref_chain(&self, offset: u64, xref: &mut XRefTable) -> Result<()> {
        // Check if this is a traditional xref table or an xref stream
        let pos = u64_to_usize(offset, "XRef offset", self.data.len())?;
        if pos + 4 > self.data.len() {
            return Err(Error::Parse {
                message: "XRef offset beyond end of file".to_string(),
                position: pos,
            });
        }

        // Skip whitespace
        let mut check_pos = pos;
        while check_pos < self.data.len() && is_whitespace(self.data[check_pos]) {
            check_pos += 1;
        }

        // Check for "xref" keyword (traditional) or number (xref stream)
        if check_pos + 4 <= self.data.len() && &self.data[check_pos..check_pos + 4] == b"xref" {
            // Traditional xref table
            let (table, trailer) = self.parse_traditional_xref(pos)?;

            // Get Prev pointer before merging
            let prev = match trailer.get_integer("Prev") {
                Some(prev) => {
                    if prev < 0 {
                        return Err(Error::Parse {
                            message: "Invalid Prev offset".to_string(),
                            position: pos,
                        });
                    }
                    Some(prev as u64)
                }
                None => None,
            };

            // Merge this table
            xref.merge(&table);
            if xref.trailer.is_none() {
                xref.set_trailer(trailer);
            }

            // Follow Prev link if present
            if let Some(prev_offset) = prev {
                self.parse_xref_chain(prev_offset, xref)?;
            }
        } else {
            // XRef stream
            let (table, stream_dict) = self.parse_xref_stream(pos)?;

            // Get Prev pointer
            let prev = match stream_dict.get_integer("Prev") {
                Some(prev) => {
                    if prev < 0 {
                        return Err(Error::Parse {
                            message: "Invalid Prev offset".to_string(),
                            position: pos,
                        });
                    }
                    Some(prev as u64)
                }
                None => None,
            };

            // Merge this table
            xref.merge(&table);
            if xref.trailer.is_none() {
                // XRef stream dict contains trailer info
                xref.set_trailer(stream_dict);
            }

            // Follow Prev link if present
            if let Some(prev_offset) = prev {
                self.parse_xref_chain(prev_offset, xref)?;
            }
        }

        Ok(())
    }

    /// Parses a traditional xref table
    fn parse_traditional_xref(&self, offset: usize) -> Result<(XRefTable, PdfDict)> {
        let mut table = XRefTable::new();
        let mut pos = offset;

        // Skip whitespace
        while pos < self.data.len() && is_whitespace(self.data[pos]) {
            pos += 1;
        }

        // Check for "xref" keyword
        if pos + 4 > self.data.len() || &self.data[pos..pos + 4] != b"xref" {
            return Err(Error::Parse {
                message: "Expected 'xref' keyword".to_string(),
                position: pos,
            });
        }
        pos += 4;

        // Parse subsections
        loop {
            // Skip whitespace
            while pos < self.data.len() && is_whitespace(self.data[pos]) {
                pos += 1;
            }

            // Check for "trailer" keyword
            if pos + 7 <= self.data.len() && &self.data[pos..pos + 7] == b"trailer" {
                break;
            }

            // Read first object number and count
            let subsection_pos = pos;
            let (first_obj, new_pos) = self.read_integer(pos)?;
            pos = new_pos;

            while pos < self.data.len() && is_whitespace(self.data[pos]) {
                pos += 1;
            }

            let (count, new_pos) = self.read_integer(pos)?;
            pos = new_pos;

            let first_obj = i64_to_usize(first_obj, "XRef subsection start", subsection_pos)?;
            let count = i64_to_usize(count, "XRef subsection count", subsection_pos)?;
            let end_obj = first_obj.checked_add(count).ok_or_else(|| Error::Parse {
                message: "XRef subsection size overflow".to_string(),
                position: subsection_pos,
            })?;

            if end_obj > MAX_XREF_ENTRIES {
                return Err(Error::Parse {
                    message: "XRef subsection too large".to_string(),
                    position: subsection_pos,
                });
            }

            // Parse entries
            for i in 0..count {
                // Skip whitespace/newlines
                while pos < self.data.len() && is_whitespace(self.data[pos]) {
                    pos += 1;
                }

                // Each entry is exactly 20 bytes: nnnnnnnnnn ggggg n/f
                if pos + 18 > self.data.len() {
                    return Err(Error::Parse {
                        message: "Incomplete xref entry".to_string(),
                        position: pos,
                    });
                }

                // Parse offset (10 digits)
                let offset_str =
                    std::str::from_utf8(&self.data[pos..pos + 10]).map_err(|_| Error::Parse {
                        message: "Invalid xref entry offset".to_string(),
                        position: pos,
                    })?;
                let entry_offset: u64 = offset_str.trim().parse().map_err(|_| Error::Parse {
                    message: format!("Invalid offset: {}", offset_str),
                    position: pos,
                })?;
                pos += 10;

                // Skip space
                pos += 1;

                // Parse generation (5 digits)
                let gen_str =
                    std::str::from_utf8(&self.data[pos..pos + 5]).map_err(|_| Error::Parse {
                        message: "Invalid xref entry generation".to_string(),
                        position: pos,
                    })?;
                let generation: u16 = gen_str.trim().parse().map_err(|_| Error::Parse {
                    message: format!("Invalid generation: {}", gen_str),
                    position: pos,
                })?;
                pos += 5;

                // Skip space
                pos += 1;

                // Parse type (n or f)
                let entry_type = self.data[pos];
                pos += 1;

                let obj_num = u32::try_from(first_obj + i).map_err(|_| Error::Parse {
                    message: "XRef object number out of range".to_string(),
                    position: pos,
                })?;
                let entry = match entry_type {
                    b'n' => XRefEntry::InUse {
                        offset: entry_offset,
                        generation,
                    },
                    b'f' => XRefEntry::Free {
                        next_free: u64_to_u32(entry_offset, "XRef free entry offset", pos)?,
                        generation,
                    },
                    _ => {
                        return Err(Error::Parse {
                            message: format!("Invalid xref entry type: {:02X}", entry_type),
                            position: pos - 1,
                        });
                    }
                };

                table.set(obj_num, entry);

                // Skip remaining whitespace in entry (CR, LF, or CRLF)
                while pos < self.data.len()
                    && (self.data[pos] == b'\r'
                        || self.data[pos] == b'\n'
                        || self.data[pos] == b' ')
                {
                    pos += 1;
                }
            }
        }

        // Parse trailer
        pos += 7; // Skip "trailer"
        while pos < self.data.len() && is_whitespace(self.data[pos]) {
            pos += 1;
        }

        let trailer = self.parse_dict_at(pos)?;

        Ok((table, trailer))
    }

    /// Parses an xref stream
    fn parse_xref_stream(&self, offset: usize) -> Result<(XRefTable, PdfDict)> {
        use crate::parser::{Lexer, Parser};

        // Parse the stream object
        let mut lexer = Lexer::new(&self.data[offset..]);
        let mut parser = Parser::new(&mut lexer);

        // Read object number and generation
        let _obj_num = parser.parse_object()?;
        let _gen = parser.parse_object()?;

        // Expect 'obj' keyword
        let obj = parser.parse_object()?;
        let stream = match obj {
            PdfObject::Stream(s) => s,
            _ => {
                return Err(Error::Parse {
                    message: "Expected stream for xref stream".to_string(),
                    position: offset,
                });
            }
        };

        let dict = stream.dict().clone();

        // Check that it's an XRef stream
        if dict.get_type() != Some("XRef") {
            return Err(Error::Parse {
                message: "Expected Type /XRef for xref stream".to_string(),
                position: offset,
            });
        }

        // Get stream parameters
        let size = dict.get_integer("Size").ok_or_else(|| Error::Parse {
            message: "XRef stream missing Size".to_string(),
            position: offset,
        })?;
        let size = i64_to_usize(size, "XRef stream Size", offset)?;

        if size > MAX_XREF_ENTRIES {
            return Err(Error::Parse {
                message: "XRef stream Size too large".to_string(),
                position: offset,
            });
        }

        // W array specifies byte widths for each field
        let w = dict.get_array("W").ok_or_else(|| Error::Parse {
            message: "XRef stream missing W".to_string(),
            position: offset,
        })?;

        if w.len() != 3 {
            return Err(Error::Parse {
                message: "XRef stream W array must have 3 elements".to_string(),
                position: offset,
            });
        }

        let w1 = i64_to_usize(w.get_integer(0).unwrap_or(0), "XRef stream W[0]", offset)?;
        let w2 = i64_to_usize(w.get_integer(1).unwrap_or(0), "XRef stream W[1]", offset)?;
        let w3 = i64_to_usize(w.get_integer(2).unwrap_or(0), "XRef stream W[2]", offset)?;

        if w1 > MAX_XREF_FIELD_WIDTH || w2 > MAX_XREF_FIELD_WIDTH || w3 > MAX_XREF_FIELD_WIDTH {
            return Err(Error::Parse {
                message: "XRef stream W entries out of range".to_string(),
                position: offset,
            });
        }

        let entry_size = w1
            .checked_add(w2)
            .and_then(|value| value.checked_add(w3))
            .ok_or_else(|| Error::Parse {
                message: "XRef stream entry size overflow".to_string(),
                position: offset,
            })?;

        // Decode the stream data
        let data = stream.decode()?;

        // Get Index array (optional, defaults to [0 Size])
        let index = dict.get_array("Index");
        let subsections: Vec<(usize, usize)> = if let Some(idx) = index {
            if idx.len() % 2 != 0 {
                return Err(Error::Parse {
                    message: "XRef stream Index array must have even length".to_string(),
                    position: offset,
                });
            }
            let mut subsections = Vec::new();
            for i in (0..idx.len()).step_by(2) {
                let first =
                    i64_to_usize(idx.get_integer(i).unwrap_or(0), "XRef stream Index", offset)?;
                let count = i64_to_usize(
                    idx.get_integer(i + 1).unwrap_or(0),
                    "XRef stream Index",
                    offset,
                )?;
                let end = first.checked_add(count).ok_or_else(|| Error::Parse {
                    message: "XRef stream Index overflow".to_string(),
                    position: offset,
                })?;
                if end > MAX_XREF_ENTRIES {
                    return Err(Error::Parse {
                        message: "XRef stream Index out of range".to_string(),
                        position: offset,
                    });
                }
                subsections.push((first, count));
            }
            subsections
        } else {
            vec![(0, size)]
        };

        // Parse entries
        let mut table = XRefTable::new();
        let mut data_pos: usize = 0;

        for (first_obj, count) in subsections {
            for i in 0..count {
                let end = data_pos
                    .checked_add(entry_size)
                    .ok_or_else(|| Error::Parse {
                        message: "XRef stream entry size overflow".to_string(),
                        position: offset,
                    })?;
                if end > data.len() {
                    return Err(Error::Parse {
                        message: "XRef stream data truncated".to_string(),
                        position: offset,
                    });
                }

                // Read fields
                let field1 = if w1 > 0 {
                    read_be_uint(&data[data_pos..data_pos + w1])
                } else {
                    1 // Default type is 1 (in-use)
                };
                let field2 = if w2 > 0 {
                    read_be_uint(&data[data_pos + w1..data_pos + w1 + w2])
                } else {
                    0
                };
                let field3 = if w3 > 0 {
                    read_be_uint(&data[data_pos + w1 + w2..data_pos + entry_size])
                } else {
                    0
                };

                data_pos = end;

                let obj_num = u32::try_from(first_obj + i).map_err(|_| Error::Parse {
                    message: "XRef stream object number out of range".to_string(),
                    position: offset,
                })?;
                let entry = match field1 {
                    0 => XRefEntry::Free {
                        next_free: u64_to_u32(field2, "XRef stream next free", offset)?,
                        generation: u64_to_u16(field3, "XRef stream generation", offset)?,
                    },
                    1 => XRefEntry::InUse {
                        offset: field2,
                        generation: u64_to_u16(field3, "XRef stream generation", offset)?,
                    },
                    2 => XRefEntry::Compressed {
                        stream_obj: u64_to_u32(field2, "XRef stream object stream", offset)?,
                        index: u64_to_u32(field3, "XRef stream object index", offset)?,
                    },
                    _ => {
                        return Err(Error::Parse {
                            message: format!("Unknown xref entry type: {}", field1),
                            position: offset,
                        });
                    }
                };

                table.set(obj_num, entry);
            }
        }

        Ok((table, dict))
    }

    /// Reads an integer from the data
    fn read_integer(&self, pos: usize) -> Result<(i64, usize)> {
        let mut end = pos;
        if end < self.data.len() && (self.data[end] == b'+' || self.data[end] == b'-') {
            end += 1;
        }
        while end < self.data.len() && self.data[end].is_ascii_digit() {
            end += 1;
        }

        let s = std::str::from_utf8(&self.data[pos..end]).map_err(|_| Error::Parse {
            message: "Invalid integer encoding".to_string(),
            position: pos,
        })?;

        let value = s.parse::<i64>().map_err(|_| Error::Parse {
            message: format!("Invalid integer: {}", s),
            position: pos,
        })?;

        Ok((value, end))
    }

    /// Parses a dictionary at the given position
    fn parse_dict_at(&self, pos: usize) -> Result<PdfDict> {
        use crate::parser::{Lexer, Parser};

        let mut lexer = Lexer::new(&self.data[pos..]);
        let mut parser = Parser::new(&mut lexer);

        let obj = parser.parse_object()?;
        match obj {
            PdfObject::Dict(d) => Ok(d),
            _ => Err(Error::Parse {
                message: "Expected dictionary".to_string(),
                position: pos,
            }),
        }
    }
}

fn i64_to_usize(value: i64, context: &str, position: usize) -> Result<usize> {
    if value < 0 {
        return Err(Error::Parse {
            message: format!("{} must be non-negative", context),
            position,
        });
    }
    usize::try_from(value).map_err(|_| Error::Parse {
        message: format!("{} out of range", context),
        position,
    })
}

fn u64_to_usize(value: u64, context: &str, position: usize) -> Result<usize> {
    usize::try_from(value).map_err(|_| Error::Parse {
        message: format!("{} out of range", context),
        position,
    })
}

fn u64_to_u32(value: u64, context: &str, position: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| Error::Parse {
        message: format!("{} out of range", context),
        position,
    })
}

fn u64_to_u16(value: u64, context: &str, position: usize) -> Result<u16> {
    u16::try_from(value).map_err(|_| Error::Parse {
        message: format!("{} out of range", context),
        position,
    })
}

/// Reads a big-endian unsigned integer of variable length
fn read_be_uint(bytes: &[u8]) -> u64 {
    let mut result: u64 = 0;
    for &byte in bytes {
        result = (result << 8) | (byte as u64);
    }
    result
}

/// Checks if a byte is PDF whitespace
fn is_whitespace(byte: u8) -> bool {
    matches!(byte, 0 | 9 | 10 | 12 | 13 | 32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xref_entry() {
        let mut table = XRefTable::new();
        table.set(
            1,
            XRefEntry::InUse {
                offset: 100,
                generation: 0,
            },
        );
        table.set(
            2,
            XRefEntry::InUse {
                offset: 200,
                generation: 0,
            },
        );

        assert_eq!(table.len(), 3); // 0, 1, 2
        assert!(table.get(0).is_none());
        assert!(matches!(
            table.get(1),
            Some(XRefEntry::InUse {
                offset: 100,
                generation: 0
            })
        ));
    }

    #[test]
    fn test_read_be_uint() {
        assert_eq!(read_be_uint(&[0x00]), 0);
        assert_eq!(read_be_uint(&[0x01]), 1);
        assert_eq!(read_be_uint(&[0xFF]), 255);
        assert_eq!(read_be_uint(&[0x01, 0x00]), 256);
        assert_eq!(read_be_uint(&[0x01, 0x00, 0x00]), 65536);
    }

    #[test]
    fn test_find_startxref() {
        let pdf = b"%PDF-1.4\nsome content\nstartxref\n12345\n%%EOF";
        let parser = XRefParser::new(pdf);
        assert_eq!(parser.find_startxref().unwrap(), 12345);
    }

    #[test]
    fn test_traditional_xref_negative_count() {
        let pdf = b"xref\n0 -1\ntrailer\n<<>>";
        let parser = XRefParser::new(pdf);
        assert!(parser.parse_traditional_xref(0).is_err());
    }
}
