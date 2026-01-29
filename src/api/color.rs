//! Color types for PDF generation
//!
//! This module provides Color and ColorInput types for flexible color specification.

/// RGB color for text and graphics
///
/// Supports parsing from CSS color strings using `csscolorparser`.
///
/// # Examples
///
/// ```rust
/// use pdfcrate::api::Color;
///
/// // Named colors
/// let red = Color::parse("red");
/// let coral = Color::parse("coral");
///
/// // Hex colors
/// let hex = Color::parse("#FF5733");
/// let hex_short = Color::parse("#F53");
///
/// // RGB/RGBA
/// let rgb = Color::parse("rgb(255, 87, 51)");
/// let rgba = Color::parse("rgba(255, 87, 51, 0.5)");
///
/// // HSL/HSLA
/// let hsl = Color::parse("hsl(14, 100%, 60%)");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red component (0.0-1.0)
    pub r: f64,
    /// Green component (0.0-1.0)
    pub g: f64,
    /// Blue component (0.0-1.0)
    pub b: f64,
    /// Alpha component (0.0-1.0)
    pub a: f64,
}

impl Color {
    /// Creates a new RGB color (alpha = 1.0)
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Color { r, g, b, a: 1.0 }
    }

    /// Creates a new RGBA color
    pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Color { r, g, b, a }
    }

    /// Creates a grayscale color
    pub fn gray(value: f64) -> Self {
        Color {
            r: value,
            g: value,
            b: value,
            a: 1.0,
        }
    }

    /// Parses a color from any CSS color string
    ///
    /// Supports:
    /// - Named colors: "red", "blue", "coral", "rebeccapurple", etc.
    /// - Hex: "#RGB", "#RRGGBB", "#RRGGBBAA"
    /// - RGB/RGBA: "rgb(255, 0, 0)", "rgba(255, 0, 0, 0.5)"
    /// - HSL/HSLA: "hsl(0, 100%, 50%)", "hsla(0, 100%, 50%, 0.5)"
    /// - HWB: "hwb(0 0% 0%)"
    ///
    /// Returns black if parsing fails.
    pub fn parse(s: &str) -> Self {
        s.parse::<csscolorparser::Color>()
            .map(|c| {
                let [r, g, b, a] = c.to_array();
                Color {
                    r: r.into(),
                    g: g.into(),
                    b: b.into(),
                    a: a.into(),
                }
            })
            .unwrap_or(Color::BLACK)
    }

    /// Creates a color from hex string (e.g., "FF0000" or "#FF0000")
    ///
    /// This is a convenience method. For full CSS color support, use `parse()`.
    pub fn hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        // Try parsing with csscolorparser first
        if let Ok(c) = format!("#{}", hex).parse::<csscolorparser::Color>() {
            let [r, g, b, a] = c.to_array();
            return Color {
                r: r.into(),
                g: g.into(),
                b: b.into(),
                a: a.into(),
            };
        }
        Color::BLACK
    }

    /// Black color
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// White color
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    /// Red color
    pub const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// Green color
    pub const GREEN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };

    /// Blue color
    pub const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
}

impl Default for Color {
    fn default() -> Self {
        Color::BLACK
    }
}

impl core::str::FromStr for Color {
    type Err = csscolorparser::ParseColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let c = s.parse::<csscolorparser::Color>()?;
        let [r, g, b, a] = c.to_array();
        Ok(Color {
            r: r.into(),
            g: g.into(),
            b: b.into(),
            a: a.into(),
        })
    }
}

impl From<csscolorparser::Color> for Color {
    fn from(c: csscolorparser::Color) -> Self {
        let [r, g, b, a] = c.to_array();
        Color {
            r: r.into(),
            g: g.into(),
            b: b.into(),
            a: a.into(),
        }
    }
}

/// Helper type for accepting colors from various input formats
///
/// This allows color methods to accept hex strings, RGB tuples,
/// arrays, or Color objects, following Prawn's flexible API.
///
/// # Examples
///
/// ```rust,no_run
/// use pdfcrate::api::Document;
///
/// let mut doc = Document::new();
/// doc.stroke(|ctx| {
///     ctx.color("FF0000");              // hex string
///     ctx.color((0.5, 0.5, 0.5));       // RGB tuple
///     ctx.color([0.5, 0.5, 0.5]);       // RGB array
/// });
/// ```
#[derive(Debug, Clone)]
pub enum ColorInput {
    /// A CSS color string (hex, named, rgb, hsl, etc.)
    Css(String),
    /// A Color object
    Color(Color),
}

impl ColorInput {
    /// Converts the input to a Color
    pub fn to_color(&self) -> Color {
        match self {
            ColorInput::Css(s) => Color::parse(s),
            ColorInput::Color(c) => *c,
        }
    }
}

impl From<&str> for ColorInput {
    fn from(s: &str) -> Self {
        ColorInput::Css(s.to_string())
    }
}

impl From<String> for ColorInput {
    fn from(s: String) -> Self {
        ColorInput::Css(s)
    }
}

impl From<Color> for ColorInput {
    fn from(c: Color) -> Self {
        ColorInput::Color(c)
    }
}

impl From<&Color> for ColorInput {
    fn from(c: &Color) -> Self {
        ColorInput::Color(*c)
    }
}

impl From<(f64, f64, f64)> for ColorInput {
    fn from((r, g, b): (f64, f64, f64)) -> Self {
        ColorInput::Color(Color::rgb(r, g, b))
    }
}

impl From<[f64; 3]> for ColorInput {
    fn from([r, g, b]: [f64; 3]) -> Self {
        ColorInput::Color(Color::rgb(r, g, b))
    }
}
