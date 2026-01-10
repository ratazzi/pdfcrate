//! TrueType font embedding
//!
//! This module handles embedding TrueType/OpenType fonts in PDF documents.
//! Supports font subsetting to reduce file size by including only used glyphs.

use std::collections::{BTreeMap, HashSet};

use crate::error::{Error, Result};
use crate::objects::{PdfArray, PdfDict, PdfName, PdfObject, PdfRef, PdfStream};

/// Generates a 6-letter uppercase subset tag from font data
/// This creates a deterministic but unique-looking prefix
fn generate_subset_tag(data: &[u8]) -> String {
    // Use a simple hash-based approach to generate 6 uppercase letters
    let mut hash: u32 = 5381;
    for byte in data.iter().take(1000) {
        hash = hash.wrapping_mul(33).wrapping_add(*byte as u32);
    }

    let mut tag = String::with_capacity(6);
    let mut h = hash;
    for _ in 0..6 {
        tag.push((b'A' + (h % 26) as u8) as char);
        h /= 26;
    }
    tag
}

/// Embedded TrueType font data
#[derive(Debug, Clone)]
pub struct EmbeddedFont {
    /// Font name (for use with set_font)
    pub name: String,
    /// PostScript name from the font
    pub postscript_name: String,
    /// Subset tag (6 uppercase letters for subset naming)
    subset_tag: String,
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
    /// Mapping from glyph ID to unicode text (built on-demand for subsetting)
    /// This is more efficient than storing full cmap for CJK fonts
    pub(crate) glyph_set: BTreeMap<u16, String>,
    /// Characters used in the document (for subsetting) - legacy field
    pub(crate) used_chars: HashSet<char>,
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

        // Don't pre-build full cmap - it's too slow for CJK fonts (65,536+ lookups)
        // Instead, we build glyph_set on-demand as characters are used

        let subset_tag = generate_subset_tag(&data);

        Ok(EmbeddedFont {
            name: postscript_name.clone(),
            postscript_name,
            subset_tag,
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
            glyph_set: BTreeMap::new(),
            used_chars: HashSet::new(),
        })
    }

    /// Looks up a glyph ID for a character using the font's cmap
    pub fn glyph_id(&self, c: char) -> Option<u16> {
        use ttf_parser::Face;
        if let Ok(face) = Face::parse(&self.data, 0) {
            face.glyph_index(c).map(|gid| gid.0)
        } else {
            None
        }
    }

    /// Records characters as used (for subsetting)
    /// Also builds the glyph_set mapping on-demand
    pub fn mark_chars_used(&mut self, text: &str) {
        use ttf_parser::Face;

        // Parse font once for this batch of characters
        if let Ok(face) = Face::parse(&self.data, 0) {
            for c in text.chars() {
                self.used_chars.insert(c);

                // Build glyph_set on-demand
                if let Some(gid) = face.glyph_index(c) {
                    // Store glyph ID -> unicode text mapping
                    // For CJK, each char is independent; for ligatures, this could be multiple chars
                    self.glyph_set.entry(gid.0).or_insert_with(|| c.to_string());
                }
            }
        } else {
            // Fallback: just track used chars
            for c in text.chars() {
                self.used_chars.insert(c);
            }
        }
    }

    /// Returns the number of unique characters used
    pub fn used_char_count(&self) -> usize {
        self.used_chars.len()
    }

    /// Returns the font name with subset prefix (e.g., "ABCDEF+FontName")
    /// This is required by PDF spec for subset fonts
    pub fn subset_font_name(&self) -> String {
        format!("{}+{}", self.subset_tag, self.postscript_name)
    }

    /// Creates a subset of the font containing only used glyphs
    ///
    /// Returns SubsettedFont with subset data and glyph mappings
    pub fn create_subset(&self) -> Result<SubsettedFont> {
        use subsetter::{subset, GlyphRemapper};

        // Create a glyph remapper from glyph_set
        let mut remapper = GlyphRemapper::new();

        // Always include .notdef (glyph 0)
        remapper.remap(0);

        // Add all glyphs from glyph_set
        for &gid in self.glyph_set.keys() {
            remapper.remap(gid);
        }

        // Use subsetter to create subset font
        let subset_data = subset(&self.data, 0, &remapper)
            .map_err(|e| Error::Font(format!("Subsetting failed: {:?}", e)))?;

        // Build glyph_set with new GIDs (gid -> unicode text)
        let mut new_glyph_set: BTreeMap<u16, String> = BTreeMap::new();
        for (&old_gid, text) in &self.glyph_set {
            if let Some(new_gid) = remapper.get(old_gid) {
                new_glyph_set.insert(new_gid, text.clone());
            }
        }

        // Build old GID -> new GID mapping
        let mut gid_mapping: Vec<(u16, u16)> = Vec::new();
        gid_mapping.push((0, remapper.get(0).unwrap_or(0))); // .notdef
        for &old_gid in self.glyph_set.keys() {
            if let Some(new_gid) = remapper.get(old_gid) {
                gid_mapping.push((old_gid, new_gid));
            }
        }

        // Update widths for new GIDs (collect all remapped gids)
        let mut new_widths: Vec<u16> = Vec::new();
        for old_gid in remapper.remapped_gids() {
            let width = self.widths.get(old_gid as usize).copied().unwrap_or(0);
            new_widths.push(width);
        }

        let max_new_gid = if new_widths.is_empty() {
            0
        } else {
            (new_widths.len() - 1) as u16
        };

        Ok(SubsettedFont {
            data: subset_data,
            glyph_set: new_glyph_set,
            widths: new_widths,
            max_gid: max_new_gid,
            gid_mapping,
        })
    }

    /// Gets the width of a character in PDF units (1/1000 of text space)
    pub fn char_width(&self, c: char) -> u16 {
        // Look up glyph ID from font's cmap
        if let Some(gid) = self.glyph_id(c) {
            self.widths.get(gid as usize).copied().unwrap_or(0)
        } else {
            0
        }
    }

    /// Measures the width of a string in PDF units
    pub fn text_width(&self, text: &str, font_size: f64) -> f64 {
        let width: u32 = text.chars().map(|c| self.char_width(c) as u32).sum();
        width as f64 * font_size / 1000.0
    }

    /// Creates the font file stream (for embedding)
    ///
    /// If subsetting is enabled and characters have been used, creates a subset font.
    /// Otherwise embeds the full font.
    pub fn create_font_file_stream(&self) -> PdfStream {
        // If no characters used, embed full font
        if self.used_chars.is_empty() {
            let mut stream = PdfStream::from_data_compressed(self.data.clone());
            let dict = stream.dict_mut();
            dict.set("Length1", PdfObject::Integer(self.data.len() as i64));
            return stream;
        }

        // Try to create subset
        match self.create_subset() {
            Ok(subset) => {
                let mut stream = PdfStream::from_data_compressed(subset.data.clone());
                let dict = stream.dict_mut();
                dict.set("Length1", PdfObject::Integer(subset.data.len() as i64));
                stream
            }
            Err(_) => {
                // Fallback to full font if subsetting fails
                let mut stream = PdfStream::from_data_compressed(self.data.clone());
                let dict = stream.dict_mut();
                dict.set("Length1", PdfObject::Integer(self.data.len() as i64));
                stream
            }
        }
    }

    /// Creates the font file stream without subsetting (full font)
    pub fn create_font_file_stream_full(&self) -> PdfStream {
        let mut stream = PdfStream::from_data_compressed(self.data.clone());
        let dict = stream.dict_mut();
        dict.set("Length1", PdfObject::Integer(self.data.len() as i64));
        stream
    }

    /// Creates the font descriptor dictionary
    ///
    /// If `is_subset` is true, uses the subset font name (ABCDEF+FontName)
    pub fn create_font_descriptor(&self, font_file_ref: PdfRef, is_subset: bool) -> PdfDict {
        let font_name = if is_subset {
            self.subset_font_name()
        } else {
            self.postscript_name.clone()
        };

        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::new("FontDescriptor")));
        dict.set("FontName", PdfObject::Name(PdfName::new(&font_name)));
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
    ///
    /// If `cid_to_gid_map_ref` is Some, uses a custom CIDToGIDMap stream for subsetting.
    /// Otherwise uses Identity mapping.
    /// If `is_subset` is true, uses the subset font name (ABCDEF+FontName)
    pub fn create_cid_font(
        &self,
        font_descriptor_ref: PdfRef,
        cid_to_gid_map_ref: Option<PdfRef>,
        is_subset: bool,
    ) -> PdfDict {
        let font_name = if is_subset {
            self.subset_font_name()
        } else {
            self.postscript_name.clone()
        };

        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::font()));
        dict.set("Subtype", PdfObject::Name(PdfName::new("CIDFontType2")));
        dict.set("BaseFont", PdfObject::Name(PdfName::new(&font_name)));

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

        // CIDToGIDMap - use custom map for subsetting, or Identity
        if let Some(map_ref) = cid_to_gid_map_ref {
            dict.set("CIDToGIDMap", PdfObject::Reference(map_ref));
        } else {
            dict.set("CIDToGIDMap", PdfObject::Name(PdfName::new("Identity")));
        }

        dict
    }

    /// Creates a CIDToGIDMap stream for font subsetting
    ///
    /// This maps original glyph IDs (used in content stream) to new subset glyph IDs.
    /// The stream is a binary table where position [2*cid, 2*cid+2) contains the big-endian GID.
    pub fn create_cid_to_gid_map(&self) -> Option<PdfStream> {
        if self.glyph_set.is_empty() {
            return None;
        }

        if let Ok(subset) = self.create_subset() {
            // Find the maximum original GID we need to map
            let max_cid = subset
                .gid_mapping
                .iter()
                .map(|&(old, _)| old)
                .max()
                .unwrap_or(0) as usize;

            // Create the mapping table (2 bytes per CID, big-endian)
            let mut data = vec![0u8; (max_cid + 1) * 2];

            for &(old_gid, new_gid) in &subset.gid_mapping {
                let offset = (old_gid as usize) * 2;
                if offset + 1 < data.len() {
                    data[offset] = (new_gid >> 8) as u8;
                    data[offset + 1] = (new_gid & 0xFF) as u8;
                }
            }

            Some(PdfStream::from_data_compressed(data))
        } else {
            None
        }
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

    /// Creates the ToUnicode CMap stream from glyph_set
    pub fn create_to_unicode_cmap(&self) -> PdfStream {
        let mut cmap_str = String::new();

        cmap_str.push_str("/CIDInit /ProcSet findresource begin\n");
        cmap_str.push_str("12 dict begin\n");
        cmap_str.push_str("begincmap\n");
        cmap_str.push_str("/CIDSystemInfo <<\n");
        cmap_str.push_str("  /Registry (Adobe)\n");
        cmap_str.push_str("  /Ordering (UCS)\n");
        cmap_str.push_str("  /Supplement 0\n");
        cmap_str.push_str(">> def\n");
        cmap_str.push_str("/CMapName /Adobe-Identity-UCS def\n");
        cmap_str.push_str("/CMapType 2 def\n");
        cmap_str.push_str("1 begincodespacerange\n");
        cmap_str.push_str("<0000> <FFFF>\n");
        cmap_str.push_str("endcodespacerange\n");

        // Build mappings from glyph_set (gid -> unicode text)
        let mappings: Vec<_> = self
            .glyph_set
            .iter()
            .filter(|(&gid, text)| gid > 0 && !text.is_empty())
            .collect();

        for chunk in mappings.chunks(100) {
            cmap_str.push_str(&format!("{} beginbfchar\n", chunk.len()));
            for (&gid, text) in chunk {
                // Encode unicode text as hex (supports multi-char like ligatures)
                let unicode_hex: String =
                    text.chars().map(|c| format!("{:04X}", c as u32)).collect();
                cmap_str.push_str(&format!("<{:04X}> <{}>\n", gid, unicode_hex));
            }
            cmap_str.push_str("endbfchar\n");
        }

        cmap_str.push_str("endcmap\n");
        cmap_str.push_str("CMapName currentdict /CMap defineresource pop\n");
        cmap_str.push_str("end\n");
        cmap_str.push_str("end\n");

        PdfStream::from_data_compressed(cmap_str.into_bytes())
    }

    /// Creates the Type0 font dictionary (the main font object)
    ///
    /// If `is_subset` is true, uses the subset font name (ABCDEF+FontName)
    pub fn create_type0_font(
        &self,
        cid_font_ref: PdfRef,
        to_unicode_ref: PdfRef,
        is_subset: bool,
    ) -> PdfDict {
        let font_name = if is_subset {
            self.subset_font_name()
        } else {
            self.postscript_name.clone()
        };

        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::font()));
        dict.set("Subtype", PdfObject::Name(PdfName::new("Type0")));
        dict.set("BaseFont", PdfObject::Name(PdfName::new(&font_name)));
        dict.set("Encoding", PdfObject::Name(PdfName::new("Identity-H")));

        // DescendantFonts (array with single CIDFont)
        let descendants = PdfArray::from(vec![PdfObject::Reference(cid_font_ref)]);
        dict.set("DescendantFonts", PdfObject::Array(descendants));

        dict.set("ToUnicode", PdfObject::Reference(to_unicode_ref));

        dict
    }

    /// Encodes text for use with this font (returns hex string content)
    ///
    /// Uses subset glyph IDs if subsetting is active, otherwise uses original GIDs.
    pub fn encode_text(&self, text: &str) -> String {
        // If we have glyph_set (subsetting is active), use subset GID mapping
        if !self.glyph_set.is_empty() {
            if let Ok(subset) = self.create_subset() {
                return subset.encode_text(text);
            }
        }

        // Fallback to original mapping (look up from font directly)
        let mut hex = String::new();
        for c in text.chars() {
            let gid = self.glyph_id(c).unwrap_or(0);
            hex.push_str(&format!("{:04X}", gid));
        }
        hex
    }

    /// Creates widths array for PDF using original GIDs
    ///
    /// When subsetting, this uses original GIDs (which match the content stream)
    /// and the CIDToGIDMap handles mapping to actual glyph positions.
    pub fn create_widths_array_for_pdf(&self) -> PdfArray {
        if !self.glyph_set.is_empty() {
            // Create widths array for used glyphs only, using original GIDs
            return self.create_widths_array_for_used_glyphs();
        }
        self.create_widths_array()
    }

    /// Creates widths array containing only used glyphs (by original GID)
    fn create_widths_array_for_used_glyphs(&self) -> PdfArray {
        let mut result = PdfArray::new();

        // Get sorted list of used original GIDs
        let mut used_gids: Vec<u16> = self.glyph_set.keys().copied().collect();
        used_gids.sort();

        // Group consecutive GIDs
        let mut i = 0;
        while i < used_gids.len() {
            let start_gid = used_gids[i];
            let mut widths = PdfArray::new();

            // Collect consecutive GIDs
            while i < used_gids.len() {
                let gid = used_gids[i];
                // Check if consecutive (or first in group)
                if widths.is_empty() || gid == start_gid + widths.len() as u16 {
                    let width = self.widths.get(gid as usize).copied().unwrap_or(0);
                    widths.push(PdfObject::Integer(width as i64));
                    i += 1;

                    if widths.len() >= 100 {
                        break;
                    }
                } else {
                    break;
                }
            }

            if !widths.is_empty() {
                result.push(PdfObject::Integer(start_gid as i64));
                result.push(PdfObject::Array(widths));
            }
        }

        result
    }

    /// Creates ToUnicode CMap using original GIDs
    ///
    /// When subsetting, this uses original GIDs (which match the content stream).
    pub fn create_to_unicode_cmap_for_pdf(&self) -> PdfStream {
        // Always use the original glyph_set which maps original GID -> Unicode
        // This is correct because:
        // - Content stream uses original GIDs as CIDs
        // - ToUnicode maps CIDs to Unicode strings
        self.create_to_unicode_cmap()
    }
}

/// Subsetted font data for PDF embedding
#[derive(Debug, Clone)]
pub struct SubsettedFont {
    /// Subsetted font data
    pub data: Vec<u8>,
    /// Mapping from new glyph ID to unicode text
    pub glyph_set: BTreeMap<u16, String>,
    /// Widths for new glyph IDs
    pub widths: Vec<u16>,
    /// Maximum new glyph ID
    pub max_gid: u16,
    /// Mapping from old GID to new GID
    pub gid_mapping: Vec<(u16, u16)>,
}

impl SubsettedFont {
    /// Encodes text using subset glyph IDs
    pub fn encode_text(&self, text: &str) -> String {
        let mut hex = String::new();
        for c in text.chars() {
            // Find glyph ID by searching glyph_set for matching unicode
            let gid = self
                .glyph_set
                .iter()
                .find(|(_, unicode)| unicode.chars().next() == Some(c))
                .map(|(&gid, _)| gid)
                .unwrap_or(0);
            hex.push_str(&format!("{:04X}", gid));
        }
        hex
    }

    /// Creates the widths array for the CIDFont (subset version)
    pub fn create_widths_array(&self) -> PdfArray {
        let mut result = PdfArray::new();

        let mut i = 0;
        while i <= self.max_gid as usize {
            if i >= self.widths.len() || self.widths[i] == 0 {
                i += 1;
                continue;
            }

            let start = i;
            let mut widths = PdfArray::new();

            while i <= self.max_gid as usize && i < self.widths.len() {
                let w = self.widths[i];
                if w == 0 && i > start + 1 {
                    break;
                }
                widths.push(PdfObject::Integer(w as i64));
                i += 1;

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

    /// Creates the ToUnicode CMap stream (subset version)
    pub fn create_to_unicode_cmap(&self) -> PdfStream {
        let mut cmap_str = String::new();

        cmap_str.push_str("/CIDInit /ProcSet findresource begin\n");
        cmap_str.push_str("12 dict begin\n");
        cmap_str.push_str("begincmap\n");
        cmap_str.push_str("/CIDSystemInfo <<\n");
        cmap_str.push_str("  /Registry (Adobe)\n");
        cmap_str.push_str("  /Ordering (UCS)\n");
        cmap_str.push_str("  /Supplement 0\n");
        cmap_str.push_str(">> def\n");
        cmap_str.push_str("/CMapName /Adobe-Identity-UCS def\n");
        cmap_str.push_str("/CMapType 2 def\n");
        cmap_str.push_str("1 begincodespacerange\n");
        cmap_str.push_str("<0000> <FFFF>\n");
        cmap_str.push_str("endcodespacerange\n");

        // Build mappings from glyph_set (gid -> unicode text)
        let mappings: Vec<_> = self
            .glyph_set
            .iter()
            .filter(|(&gid, text)| gid > 0 && !text.is_empty())
            .collect();

        for chunk in mappings.chunks(100) {
            cmap_str.push_str(&format!("{} beginbfchar\n", chunk.len()));
            for (&gid, text) in chunk {
                // Encode unicode text as hex (supports multi-char like ligatures)
                let unicode_hex: String =
                    text.chars().map(|c| format!("{:04X}", c as u32)).collect();
                cmap_str.push_str(&format!("<{:04X}> <{}>\n", gid, unicode_hex));
            }
            cmap_str.push_str("endbfchar\n");
        }

        cmap_str.push_str("endcmap\n");
        cmap_str.push_str("CMapName currentdict /CMap defineresource pop\n");
        cmap_str.push_str("end\n");
        cmap_str.push_str("end\n");

        PdfStream::from_data_compressed(cmap_str.into_bytes())
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
            subset_tag: "ABCDEF".to_string(),
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
            glyph_set: BTreeMap::new(),
            used_chars: HashSet::new(),
        };

        let widths = font.create_widths_array();
        assert!(!widths.is_empty());
    }

    #[test]
    fn test_mark_chars_used() {
        // This test needs a real font to work since mark_chars_used parses font data
        // We'll just test the basic used_chars tracking without font parsing
        let mut font = EmbeddedFont {
            name: "Test".to_string(),
            postscript_name: "Test".to_string(),
            subset_tag: "ABCDEF".to_string(),
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
            data: vec![], // Empty data - will use fallback path
            glyph_set: BTreeMap::new(),
            used_chars: HashSet::new(),
        };

        assert_eq!(font.used_char_count(), 0);
        font.mark_chars_used("ABC");
        assert_eq!(font.used_char_count(), 3);
        font.mark_chars_used("AAA"); // Duplicates should not increase count
        assert_eq!(font.used_char_count(), 3);
        font.mark_chars_used("D");
        assert_eq!(font.used_char_count(), 4);
    }
}
