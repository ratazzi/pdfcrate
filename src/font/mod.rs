//! PDF Font handling
//!
//! This module handles PDF fonts, both standard and embedded.

#[cfg(feature = "fonts")]
pub mod truetype;

pub mod kern_tables;

#[cfg(feature = "fonts")]
pub use truetype::{EmbeddedFont, ShapedGlyph};

use crate::objects::{PdfDict, PdfName, PdfObject};
use pdf_canvas::{BuiltinFont, FontSource};

/// Standard PDF fonts
///
/// These 14 fonts are built into every PDF reader and don't need to be embedded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardFont {
    Courier,
    CourierBold,
    CourierOblique,
    CourierBoldOblique,
    Helvetica,
    HelveticaBold,
    HelveticaOblique,
    HelveticaBoldOblique,
    TimesRoman,
    TimesBold,
    TimesItalic,
    TimesBoldItalic,
    Symbol,
    ZapfDingbats,
}

impl StandardFont {
    /// Returns the PDF name for this font
    pub fn pdf_name(&self) -> &'static str {
        match self {
            StandardFont::Courier => "Courier",
            StandardFont::CourierBold => "Courier-Bold",
            StandardFont::CourierOblique => "Courier-Oblique",
            StandardFont::CourierBoldOblique => "Courier-BoldOblique",
            StandardFont::Helvetica => "Helvetica",
            StandardFont::HelveticaBold => "Helvetica-Bold",
            StandardFont::HelveticaOblique => "Helvetica-Oblique",
            StandardFont::HelveticaBoldOblique => "Helvetica-BoldOblique",
            StandardFont::TimesRoman => "Times-Roman",
            StandardFont::TimesBold => "Times-Bold",
            StandardFont::TimesItalic => "Times-Italic",
            StandardFont::TimesBoldItalic => "Times-BoldItalic",
            StandardFont::Symbol => "Symbol",
            StandardFont::ZapfDingbats => "ZapfDingbats",
        }
    }

    /// Creates the font dictionary for this standard font
    pub fn to_dict(&self) -> PdfDict {
        let mut dict = PdfDict::new();
        dict.set("Type", PdfObject::Name(PdfName::font()));
        dict.set("Subtype", PdfObject::Name(PdfName::new("Type1")));
        dict.set("BaseFont", PdfObject::Name(PdfName::new(self.pdf_name())));

        // Add encoding for non-symbol fonts
        if !matches!(self, StandardFont::Symbol | StandardFont::ZapfDingbats) {
            dict.set("Encoding", PdfObject::Name(PdfName::new("WinAnsiEncoding")));
        }

        dict
    }

    /// Parses a font name string into a StandardFont
    pub fn from_name(name: &str) -> Option<StandardFont> {
        match name {
            "Courier" => Some(StandardFont::Courier),
            "Courier-Bold" => Some(StandardFont::CourierBold),
            "Courier-Oblique" => Some(StandardFont::CourierOblique),
            "Courier-BoldOblique" => Some(StandardFont::CourierBoldOblique),
            "Helvetica" => Some(StandardFont::Helvetica),
            "Helvetica-Bold" => Some(StandardFont::HelveticaBold),
            "Helvetica-Oblique" => Some(StandardFont::HelveticaOblique),
            "Helvetica-BoldOblique" => Some(StandardFont::HelveticaBoldOblique),
            "Times-Roman" | "Times" => Some(StandardFont::TimesRoman),
            "Times-Bold" => Some(StandardFont::TimesBold),
            "Times-Italic" => Some(StandardFont::TimesItalic),
            "Times-BoldItalic" => Some(StandardFont::TimesBoldItalic),
            "Symbol" => Some(StandardFont::Symbol),
            "ZapfDingbats" => Some(StandardFont::ZapfDingbats),
            _ => None,
        }
    }
}

/// Font metrics for standard fonts
pub struct FontMetrics {
    /// Average character width (in 1/1000 of text space)
    pub avg_width: i32,
    /// Ascender height
    pub ascender: i32,
    /// Descender depth (negative)
    pub descender: i32,
    /// Cap height
    pub cap_height: i32,
    /// x height
    pub x_height: i32,
    /// Line gap (extra space between lines, in 1/1000 of text space)
    pub line_gap: i32,
}

impl StandardFont {
    /// Returns the corresponding pdf_canvas::BuiltinFont
    fn as_builtin_font(&self) -> BuiltinFont {
        match self {
            StandardFont::Courier => BuiltinFont::Courier,
            StandardFont::CourierBold => BuiltinFont::Courier_Bold,
            StandardFont::CourierOblique => BuiltinFont::Courier_Oblique,
            StandardFont::CourierBoldOblique => BuiltinFont::Courier_BoldOblique,
            StandardFont::Helvetica => BuiltinFont::Helvetica,
            StandardFont::HelveticaBold => BuiltinFont::Helvetica_Bold,
            StandardFont::HelveticaOblique => BuiltinFont::Helvetica_Oblique,
            StandardFont::HelveticaBoldOblique => BuiltinFont::Helvetica_BoldOblique,
            StandardFont::TimesRoman => BuiltinFont::Times_Roman,
            StandardFont::TimesBold => BuiltinFont::Times_Bold,
            StandardFont::TimesItalic => BuiltinFont::Times_Italic,
            StandardFont::TimesBoldItalic => BuiltinFont::Times_BoldItalic,
            StandardFont::Symbol => BuiltinFont::Symbol,
            StandardFont::ZapfDingbats => BuiltinFont::ZapfDingbats,
        }
    }

    /// Measures the width of a string in 1/1000 em units
    ///
    /// Uses AFM metrics from pdf-canvas for accurate text measurement.
    pub fn string_width(&self, text: &str) -> i32 {
        self.as_builtin_font().get_width_raw(text) as i32
    }

    /// Returns approximate metrics for this font
    pub fn metrics(&self) -> FontMetrics {
        // Metrics from AFM files, line_gap calculated to match Prawn behavior
        match self {
            // line_gap derived from AFM FontBBox: (bbox_top - bbox_bottom) - (ascender - descender)
            StandardFont::Courier | StandardFont::CourierOblique => FontMetrics {
                avg_width: 600,
                ascender: 629,
                descender: -157,
                cap_height: 562,
                x_height: 426,
                line_gap: 269,
            },
            StandardFont::CourierBold | StandardFont::CourierBoldOblique => FontMetrics {
                avg_width: 600,
                ascender: 629,
                descender: -157,
                cap_height: 562,
                x_height: 426,
                line_gap: 265,
            },
            StandardFont::Helvetica | StandardFont::HelveticaOblique => FontMetrics {
                avg_width: 500,
                ascender: 718,
                descender: -207,
                cap_height: 718,
                x_height: 523,
                line_gap: 231,
            },
            StandardFont::HelveticaBold | StandardFont::HelveticaBoldOblique => FontMetrics {
                avg_width: 500,
                ascender: 718,
                descender: -207,
                cap_height: 718,
                x_height: 523,
                line_gap: 265,
            },
            StandardFont::TimesRoman => FontMetrics {
                avg_width: 500,
                ascender: 683,
                descender: -217,
                cap_height: 662,
                x_height: 450,
                line_gap: 216,
            },
            StandardFont::TimesBold => FontMetrics {
                avg_width: 500,
                ascender: 683,
                descender: -217,
                cap_height: 662,
                x_height: 450,
                line_gap: 253,
            },
            StandardFont::TimesItalic => FontMetrics {
                avg_width: 500,
                ascender: 683,
                descender: -217,
                cap_height: 662,
                x_height: 450,
                line_gap: 200,
            },
            StandardFont::TimesBoldItalic => FontMetrics {
                avg_width: 500,
                ascender: 683,
                descender: -217,
                cap_height: 662,
                x_height: 450,
                line_gap: 239,
            },
            StandardFont::Symbol => FontMetrics {
                avg_width: 500,
                ascender: 1010,
                descender: -293,
                cap_height: 1010,
                x_height: 500,
                line_gap: 200,
            },
            StandardFont::ZapfDingbats => FontMetrics {
                avg_width: 500,
                ascender: 820,
                descender: -143,
                cap_height: 820,
                x_height: 500,
                line_gap: 200,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_font_name() {
        assert_eq!(StandardFont::Helvetica.pdf_name(), "Helvetica");
        assert_eq!(StandardFont::TimesRoman.pdf_name(), "Times-Roman");
    }

    #[test]
    fn test_font_dict() {
        let dict = StandardFont::Helvetica.to_dict();
        assert_eq!(dict.get_type(), Some("Font"));
        assert_eq!(dict.get_name("Subtype").map(|n| n.as_str()), Some("Type1"));
    }

    #[test]
    fn test_from_name() {
        assert_eq!(
            StandardFont::from_name("Helvetica"),
            Some(StandardFont::Helvetica)
        );
        assert_eq!(StandardFont::from_name("Unknown"), None);
    }

    #[test]
    fn test_string_width() {
        // Uses pdf-canvas for AFM metrics
        // "Hello World" in Helvetica should match pdf-canvas's get_width_raw
        assert_eq!(StandardFont::Helvetica.string_width("Hello World"), 5167);

        // Courier is monospace: 600 per character
        assert_eq!(StandardFont::Courier.string_width("ABCD"), 2400);
        assert_eq!(StandardFont::Courier.string_width("abcd"), 2400);
    }

    #[test]
    fn test_kerning_width_matches_prawn() {
        use super::kern_tables;

        // Compare with Prawn's width_of("The quick brown fox...", kerning: true)
        // Prawn at 9pt: 179.361 (with kerning), 180.576 (without)
        let font = StandardFont::Helvetica;
        let text = "The quick brown fox jumps over the lazy dog.";
        let raw = font.string_width(text) as f64; // 20064 units
        let kern = kern_tables::total_kern_adjustment(&font, text) as f64;
        let width_9pt = (raw + kern) * 9.0 / 1000.0;
        // Prawn: 179.361
        assert!(
            (width_9pt - 179.361).abs() < 0.01,
            "kerned width at 9pt: {}, expected ~179.361",
            width_9pt
        );
    }

    #[test]
    fn test_text_box_line_widths_match_prawn() {
        use super::kern_tables;

        let font = StandardFont::Helvetica;
        let size = 9.0;

        // Prawn line breaks for text_box_overflow_demo at 220pt width
        let lines = [
            (
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
                219.078,
            ),
            (
                "Sed do eiusmod tempor incididunt ut labore et dolore",
                211.104,
            ),
            (
                "magna aliqua. Ut enim ad minim veniam, quis nostrud",
                214.488,
            ),
            (
                "exercitation ullamco laboris. Duis aute irure dolor in",
                203.661,
            ),
            ("reprehenderit in voluptate velit esse cillum.", 169.749),
        ];

        for (text, prawn_width) in &lines {
            let raw = font.string_width(text) as f64;
            let kern = kern_tables::total_kern_adjustment(&font, text) as f64;
            let width = (raw + kern) * size / 1000.0;
            assert!(
                (width - prawn_width).abs() < 0.01,
                "width mismatch for \"{}\": pdfcrate={:.3}, prawn={:.3}",
                text,
                width,
                prawn_width
            );
        }
    }
}
