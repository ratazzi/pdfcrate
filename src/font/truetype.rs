//! TrueType font embedding
//!
//! This module handles embedding TrueType/OpenType fonts in PDF documents.

use crate::error::{Error, Result};
use crate::objects::{PdfArray, PdfDict, PdfName, PdfObject, PdfRef, PdfStream};

/// Embedded TrueType font data
#[derive(Debug, Clone)]
pub struct EmbeddedFont {
    /// Font name (for use with set_font)
    pub name: String,
    /// PostScript name from the font
    pub postscript_name: String,
    /// Font flags
    pub flags: u32,
    /// Italic angle
    pub italic_angle: f64,
    /// Ascender (in PDF units, 1000 = 1 em)
    pub ascender: i32,
    /// Descender (negative, in PDF units)
    pub descender: i32,
    /// Cap height
    pub cap_height: i32,
    /// Stem vertical width (estimated)
    pub stem_v: i32,
    /// Font bounding box [x_min, y_min, x_max, y_max]
    pub bbox: [i32; 4],
    /// Units per em in the original font
    pub units_per_em: u16,
    /// Character widths (glyph ID -> width in PDF units)
    pub widths: Vec<u16>,
    /// Maximum glyph ID used
    pub max_gid: u16,
    /// Raw font data (for embedding)
    pub data: Vec<u8>,
    /// Mapping from Unicode codepoint to glyph ID
    pub cmap: Vec<(u32, u16)>,
}

impl EmbeddedFont {
    /// Parses a TrueType font from bytes
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        use ttf_parser::Face;

        let face = Face::parse(&data, 0)
            .map_err(|e| Error::Font(format!("Failed to parse font: {:?}", e)))?;

        // Get font names - use PostScript name as primary identifier (unique per variant)
        let postscript_name = face
            .names()
            .into_iter()
            .find(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
            .and_then(|n| n.to_string())
            .unwrap_or_else(|| "UnknownFont".to_string());

        // Family name is just for reference, we use postscript_name as the key
        let _family_name = face
            .names()
            .into_iter()
            .find(|n| n.name_id == ttf_parser::name_id::FAMILY)
            .and_then(|n| n.to_string())
            .unwrap_or_else(|| postscript_name.clone());

        // Get metrics
        let units_per_em = face.units_per_em();
        let scale = 1000.0 / units_per_em as f64;

        let ascender = (face.ascender() as f64 * scale) as i32;
        let descender = (face.descender() as f64 * scale) as i32;
        let cap_height = face
            .capital_height()
            .map(|h| (h as f64 * scale) as i32)
            .unwrap_or(ascender);

        // Font bounding box
        let global_bbox = face.global_bounding_box();
        let bbox = [
            (global_bbox.x_min as f64 * scale) as i32,
            (global_bbox.y_min as f64 * scale) as i32,
            (global_bbox.x_max as f64 * scale) as i32,
            (global_bbox.y_max as f64 * scale) as i32,
        ];

        // Italic angle
        let italic_angle = face.italic_angle().unwrap_or(0.0) as f64;

        // Calculate flags
        let mut flags: u32 = 0;
        if face.is_monospaced() {
            flags |= 1 << 0; // FixedPitch
        }
        // Assume symbolic if not Latin
        flags |= 1 << 2; // Symbolic (required for TrueType)
        if italic_angle != 0.0 {
            flags |= 1 << 6; // Italic
        }

        // Estimate StemV (vertical stem width)
        let stem_v = if face.weight().to_number() >= 700 {
            140
        } else {
            80
        };

        // Get glyph widths
        let num_glyphs = face.number_of_glyphs();
        let mut widths = Vec::with_capacity(num_glyphs as usize);
        let mut max_gid: u16 = 0;

        for gid in 0..num_glyphs {
            let glyph_id = ttf_parser::GlyphId(gid);
            let width = face
                .glyph_hor_advance(glyph_id)
                .map(|w| (w as f64 * scale) as u16)
                .unwrap_or(0);
            widths.push(width);
            if width > 0 {
                max_gid = gid;
            }
        }

        // Build Unicode to GlyphID mapping
        let mut cmap: Vec<(u32, u16)> = Vec::new();

        // Iterate through common Unicode ranges
        for codepoint in 0x0020u32..0xFFFFu32 {
            if let Some(char) = char::from_u32(codepoint) {
                if let Some(gid) = face.glyph_index(char) {
                    cmap.push((codepoint, gid.0));
                }
            }
        }

        Ok(EmbeddedFont {
            name: postscript_name.clone(),
            postscript_name,
            flags,
            italic_angle,
            ascender,
            descender,
            cap_height,
            stem_v,
            bbox,
            units_per_em,
            widths,
            max_gid,
            data,
            cmap,
        })
    }

    /// Gets the width of a character in PDF units (1/1000 of text space)
    pub fn char_width(&self, c: char) -> u16 {
        // Find glyph ID for character
        for &(codepoint, gid) in &self.cmap {
            if codepoint == c as u32 {
                return self.widths.get(gid as usize).copied().unwrap_or(0);
            }
        }
        0
    }

    /// Measures the width of a string in PDF units
    pub fn text_width(&self, text: &str, font_size: f64) -> f64 {
        let width: u32 = text.chars().map(|c| self.char_width(c) as u32).sum();
        width as f64 * font_size / 1000.0
    }

    /// Creates the font file stream (for embedding)
    pub fn create_font_file_stream(&self) -> PdfStream {
        let mut stream = PdfStream::from_data_compressed(self.data.clone());
        let dict = stream.dict_mut();
        dict.set("Length1", PdfObject::Integer(self.data.len() as i64));
        stream
    }

    /// Creates the font descriptor dictionary
    pub fn create_font_descriptor(&self, font_file_ref: PdfRef) -> PdfDict {
        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::new("FontDescriptor")));
        dict.set(
            "FontName",
            PdfObject::Name(PdfName::new(&self.postscript_name)),
        );
        dict.set("Flags", PdfObject::Integer(self.flags as i64));

        // Bounding box
        let bbox = PdfArray::from(vec![
            PdfObject::Integer(self.bbox[0] as i64),
            PdfObject::Integer(self.bbox[1] as i64),
            PdfObject::Integer(self.bbox[2] as i64),
            PdfObject::Integer(self.bbox[3] as i64),
        ]);
        dict.set("FontBBox", PdfObject::Array(bbox));

        dict.set("ItalicAngle", PdfObject::Real(self.italic_angle));
        dict.set("Ascent", PdfObject::Integer(self.ascender as i64));
        dict.set("Descent", PdfObject::Integer(self.descender as i64));
        dict.set("CapHeight", PdfObject::Integer(self.cap_height as i64));
        dict.set("StemV", PdfObject::Integer(self.stem_v as i64));
        dict.set("FontFile2", PdfObject::Reference(font_file_ref));

        dict
    }

    /// Creates the CIDFont dictionary (descendant of Type0)
    pub fn create_cid_font(&self, font_descriptor_ref: PdfRef) -> PdfDict {
        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::font()));
        dict.set("Subtype", PdfObject::Name(PdfName::new("CIDFontType2")));
        dict.set(
            "BaseFont",
            PdfObject::Name(PdfName::new(&self.postscript_name)),
        );

        // CIDSystemInfo
        let mut cid_info = PdfDict::new();
        cid_info.set("Registry", PdfObject::String("Adobe".into()));
        cid_info.set("Ordering", PdfObject::String("Identity".into()));
        cid_info.set("Supplement", PdfObject::Integer(0));
        dict.set("CIDSystemInfo", PdfObject::Dict(cid_info));

        dict.set("FontDescriptor", PdfObject::Reference(font_descriptor_ref));

        // DW (default width)
        dict.set("DW", PdfObject::Integer(1000));

        // W (widths array) - format: [start [w1 w2 w3 ...]]
        let w_array = self.create_widths_array();
        dict.set("W", PdfObject::Array(w_array));

        // CIDToGIDMap - Identity mapping
        dict.set("CIDToGIDMap", PdfObject::Name(PdfName::new("Identity")));

        dict
    }

    /// Creates the widths array for the CIDFont
    fn create_widths_array(&self) -> PdfArray {
        let mut result = PdfArray::new();

        // Group consecutive glyphs with widths
        let mut i = 0;
        while i <= self.max_gid as usize {
            // Skip glyphs with zero width
            if i >= self.widths.len() || self.widths[i] == 0 {
                i += 1;
                continue;
            }

            // Start a new range
            let start = i;
            let mut widths = PdfArray::new();

            // Collect consecutive non-zero widths
            while i <= self.max_gid as usize && i < self.widths.len() {
                let w = self.widths[i];
                if w == 0 && i > start + 1 {
                    // End range on zero width (but include at least 2)
                    break;
                }
                widths.push(PdfObject::Integer(w as i64));
                i += 1;

                // Limit range size
                if widths.len() >= 100 {
                    break;
                }
            }

            if !widths.is_empty() {
                result.push(PdfObject::Integer(start as i64));
                result.push(PdfObject::Array(widths));
            }
        }

        result
    }

    /// Creates the ToUnicode CMap stream
    pub fn create_to_unicode_cmap(&self) -> PdfStream {
        let mut cmap = String::new();

        cmap.push_str("/CIDInit /ProcSet findresource begin\n");
        cmap.push_str("12 dict begin\n");
        cmap.push_str("begincmap\n");
        cmap.push_str("/CIDSystemInfo <<\n");
        cmap.push_str("  /Registry (Adobe)\n");
        cmap.push_str("  /Ordering (UCS)\n");
        cmap.push_str("  /Supplement 0\n");
        cmap.push_str(">> def\n");
        cmap.push_str("/CMapName /Adobe-Identity-UCS def\n");
        cmap.push_str("/CMapType 2 def\n");
        cmap.push_str("1 begincodespacerange\n");
        cmap.push_str("<0000> <FFFF>\n");
        cmap.push_str("endcodespacerange\n");

        // Write character mappings in batches of 100
        let mappings: Vec<_> = self
            .cmap
            .iter()
            .filter(|&&(cp, gid)| gid > 0 && cp < 0x10000)
            .collect();

        for chunk in mappings.chunks(100) {
            cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
            for &&(codepoint, gid) in chunk {
                cmap.push_str(&format!("<{:04X}> <{:04X}>\n", gid, codepoint));
            }
            cmap.push_str("endbfchar\n");
        }

        cmap.push_str("endcmap\n");
        cmap.push_str("CMapName currentdict /CMap defineresource pop\n");
        cmap.push_str("end\n");
        cmap.push_str("end\n");

        PdfStream::from_data_compressed(cmap.into_bytes())
    }

    /// Creates the Type0 font dictionary (the main font object)
    pub fn create_type0_font(&self, cid_font_ref: PdfRef, to_unicode_ref: PdfRef) -> PdfDict {
        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::font()));
        dict.set("Subtype", PdfObject::Name(PdfName::new("Type0")));
        dict.set(
            "BaseFont",
            PdfObject::Name(PdfName::new(&self.postscript_name)),
        );
        dict.set("Encoding", PdfObject::Name(PdfName::new("Identity-H")));

        // DescendantFonts (array with single CIDFont)
        let descendants = PdfArray::from(vec![PdfObject::Reference(cid_font_ref)]);
        dict.set("DescendantFonts", PdfObject::Array(descendants));

        dict.set("ToUnicode", PdfObject::Reference(to_unicode_ref));

        dict
    }

    /// Encodes text for use with this font (returns hex string content)
    pub fn encode_text(&self, text: &str) -> String {
        let mut hex = String::new();
        for c in text.chars() {
            // Find glyph ID
            let gid = self
                .cmap
                .iter()
                .find(|&&(cp, _)| cp == c as u32)
                .map(|&(_, gid)| gid)
                .unwrap_or(0);
            hex.push_str(&format!("{:04X}", gid));
        }
        hex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_font_widths_array() {
        let font = EmbeddedFont {
            name: "Test".to_string(),
            postscript_name: "Test".to_string(),
            flags: 4,
            italic_angle: 0.0,
            ascender: 800,
            descender: -200,
            cap_height: 700,
            stem_v: 80,
            bbox: [0, -200, 1000, 800],
            units_per_em: 1000,
            widths: vec![0, 500, 600, 700, 0, 800],
            max_gid: 5,
            data: vec![],
            cmap: vec![(65, 1), (66, 2), (67, 3)],
        };

        let widths = font.create_widths_array();
        assert!(!widths.is_empty());
    }
}
