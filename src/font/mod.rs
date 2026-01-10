//! PDF Font handling
//!
//! This module handles PDF fonts, both standard and embedded.

#[cfg(feature = "fonts")]
pub mod truetype;

#[cfg(feature = "fonts")]
pub use truetype::EmbeddedFont;

use crate::objects::{PdfDict, PdfName, PdfObject};

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
}

impl StandardFont {
    /// Returns approximate metrics for this font
    pub fn metrics(&self) -> FontMetrics {
        // Simplified metrics - in production, load from AFM files
        match self {
            StandardFont::Courier
            | StandardFont::CourierBold
            | StandardFont::CourierOblique
            | StandardFont::CourierBoldOblique => FontMetrics {
                avg_width: 600,
                ascender: 629,
                descender: -157,
                cap_height: 562,
                x_height: 426,
            },
            StandardFont::Helvetica
            | StandardFont::HelveticaBold
            | StandardFont::HelveticaOblique
            | StandardFont::HelveticaBoldOblique => FontMetrics {
                avg_width: 500,
                ascender: 718,
                descender: -207,
                cap_height: 718,
                x_height: 523,
            },
            StandardFont::TimesRoman
            | StandardFont::TimesBold
            | StandardFont::TimesItalic
            | StandardFont::TimesBoldItalic => FontMetrics {
                avg_width: 500,
                ascender: 683,
                descender: -217,
                cap_height: 662,
                x_height: 450,
            },
            StandardFont::Symbol => FontMetrics {
                avg_width: 500,
                ascender: 1010,
                descender: -293,
                cap_height: 1010,
                x_height: 500,
            },
            StandardFont::ZapfDingbats => FontMetrics {
                avg_width: 500,
                ascender: 820,
                descender: -143,
                cap_height: 820,
                x_height: 500,
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
}
