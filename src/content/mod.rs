//! PDF Content Stream
//!
//! This module handles PDF content streams and operators.

mod operators;

pub use operators::*;

use crate::objects::format_real;

/// Content stream builder
///
/// Builds PDF content streams using a fluent API.
#[derive(Debug, Default)]
pub struct ContentBuilder {
    operations: Vec<u8>,
}

impl ContentBuilder {
    /// Creates a new content builder
    pub fn new() -> Self {
        ContentBuilder {
            operations: Vec::new(),
        }
    }

    /// Returns the built content as bytes
    pub fn build(self) -> Vec<u8> {
        self.operations
    }

    /// Writes raw content
    pub fn raw(&mut self, content: &[u8]) -> &mut Self {
        self.operations.extend_from_slice(content);
        self
    }

    /// Writes a line of content
    pub fn line(&mut self, content: &str) -> &mut Self {
        self.operations.extend_from_slice(content.as_bytes());
        self.operations.push(b'\n');
        self
    }

    // Graphics state operators

    /// Save graphics state (q)
    pub fn save_state(&mut self) -> &mut Self {
        self.line("q")
    }

    /// Restore graphics state (Q)
    pub fn restore_state(&mut self) -> &mut Self {
        self.line("Q")
    }

    /// Set line width (w)
    pub fn set_line_width(&mut self, width: f64) -> &mut Self {
        self.line(&format!("{} w", format_number(width)))
    }

    /// Set line cap style (J)
    pub fn set_line_cap(&mut self, cap: LineCap) -> &mut Self {
        self.line(&format!("{} J", cap as u8))
    }

    /// Set line join style (j)
    pub fn set_line_join(&mut self, join: LineJoin) -> &mut Self {
        self.line(&format!("{} j", join as u8))
    }

    /// Set dash pattern (d)
    pub fn set_dash(&mut self, pattern: &[f64], phase: f64) -> &mut Self {
        let arr: Vec<String> = pattern.iter().map(|&n| format_number(n)).collect();
        self.line(&format!("[{}] {} d", arr.join(" "), format_number(phase)))
    }

    /// Clear dash pattern (solid line)
    pub fn clear_dash(&mut self) -> &mut Self {
        self.line("[] 0 d")
    }

    /// Set miter limit (M)
    pub fn set_miter_limit(&mut self, limit: f64) -> &mut Self {
        self.line(&format!("{} M", format_number(limit)))
    }

    // Color operators

    /// Set stroke color (RGB) (RG)
    pub fn set_stroke_color_rgb(&mut self, r: f64, g: f64, b: f64) -> &mut Self {
        self.line(&format!(
            "{} {} {} RG",
            format_number(r),
            format_number(g),
            format_number(b)
        ))
    }

    /// Set fill color (RGB) (rg)
    pub fn set_fill_color_rgb(&mut self, r: f64, g: f64, b: f64) -> &mut Self {
        self.line(&format!(
            "{} {} {} rg",
            format_number(r),
            format_number(g),
            format_number(b)
        ))
    }

    /// Set stroke color (grayscale) (G)
    pub fn set_stroke_color_gray(&mut self, gray: f64) -> &mut Self {
        self.line(&format!("{} G", format_number(gray)))
    }

    /// Set fill color (grayscale) (g)
    pub fn set_fill_color_gray(&mut self, gray: f64) -> &mut Self {
        self.line(&format!("{} g", format_number(gray)))
    }

    /// Set stroke color (CMYK) (K)
    pub fn set_stroke_color_cmyk(&mut self, c: f64, m: f64, y: f64, k: f64) -> &mut Self {
        self.line(&format!(
            "{} {} {} {} K",
            format_number(c),
            format_number(m),
            format_number(y),
            format_number(k)
        ))
    }

    /// Set fill color (CMYK) (k)
    pub fn set_fill_color_cmyk(&mut self, c: f64, m: f64, y: f64, k: f64) -> &mut Self {
        self.line(&format!(
            "{} {} {} {} k",
            format_number(c),
            format_number(m),
            format_number(y),
            format_number(k)
        ))
    }

    /// Set graphics state from extended graphics state dictionary (gs)
    pub fn set_graphics_state(&mut self, name: &str) -> &mut Self {
        self.line(&format!("/{} gs", name))
    }

    // Path construction operators

    /// Move to (m)
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.line(&format!("{} {} m", format_number(x), format_number(y)))
    }

    /// Line to (l)
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.line(&format!("{} {} l", format_number(x), format_number(y)))
    }

    /// Cubic Bezier curve (c)
    pub fn curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> &mut Self {
        self.line(&format!(
            "{} {} {} {} {} {} c",
            format_number(x1),
            format_number(y1),
            format_number(x2),
            format_number(y2),
            format_number(x3),
            format_number(y3)
        ))
    }

    /// Close path (h)
    pub fn close_path(&mut self) -> &mut Self {
        self.line("h")
    }

    /// Rectangle (re) - Prawn-compatible, top-left origin
    ///
    /// Draws a rectangle where (x, y) is the **top-left corner**, matching Prawn's convention.
    /// Internally converts to PDF's native format (bottom-left origin).
    ///
    /// # Parameters
    ///
    /// * `width` and `height` must be positive values.
    ///
    /// For PDF-native coordinates (bottom-left origin), use [`rect_bl`](Self::rect_bl).
    pub fn rect(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        // PDF's `re` operator uses bottom-left origin, so convert from top-left
        self.line(&format!(
            "{} {} {} {} re",
            format_number(x),
            format_number(y - height), // Convert top-left y to bottom-left y
            format_number(width),
            format_number(height)
        ))
    }

    /// Rectangle with bottom-left origin (PDF native coordinates)
    ///
    /// This method directly uses PDF's native coordinate system
    /// where (x, y) specifies the **bottom-left corner** of the rectangle.
    ///
    /// # Parameters
    ///
    /// * `width` and `height` must be positive values.
    pub fn rect_bl(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        self.line(&format!(
            "{} {} {} {} re",
            format_number(x),
            format_number(y),
            format_number(width),
            format_number(height)
        ))
    }

    /// Circle (approximated with Bezier curves)
    ///
    /// Draws a circle centered at (cx, cy) with the given radius.
    pub fn circle(&mut self, cx: f64, cy: f64, r: f64) -> &mut Self {
        self.ellipse(cx, cy, r, r)
    }

    /// Ellipse (approximated with Bezier curves)
    ///
    /// Draws an ellipse centered at (cx, cy) with horizontal radius rx
    /// and vertical radius ry. Drawing direction is counter-clockwise,
    /// matching Prawn's behavior.
    ///
    /// After drawing, the current point is moved to the center of the ellipse.
    pub fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) -> &mut Self {
        // Magic number for approximating a quarter circle with a Bezier curve
        // kappa = 4 * (sqrt(2) - 1) / 3 ≈ 0.5522847498
        const KAPPA: f64 = 0.5522847498;

        let ox = rx * KAPPA; // Control point offset horizontal
        let oy = ry * KAPPA; // Control point offset vertical

        // Start at the right-most point (3 o'clock position)
        self.move_to(cx + rx, cy);

        // Upper right quadrant (3 o'clock to 12 o'clock) - counter-clockwise
        self.curve_to(cx + rx, cy + oy, cx + ox, cy + ry, cx, cy + ry);

        // Upper left quadrant (12 o'clock to 9 o'clock)
        self.curve_to(cx - ox, cy + ry, cx - rx, cy + oy, cx - rx, cy);

        // Lower left quadrant (9 o'clock to 6 o'clock)
        self.curve_to(cx - rx, cy - oy, cx - ox, cy - ry, cx, cy - ry);

        // Lower right quadrant (6 o'clock to 3 o'clock)
        self.curve_to(cx + ox, cy - ry, cx + rx, cy - oy, cx + rx, cy);

        // Move to center (Prawn behavior - does NOT close path)
        self.move_to(cx, cy)
    }

    /// Rounded rectangle - Prawn-compatible, top-left origin
    ///
    /// Draws a rectangle with rounded corners. The point (x, y) specifies the
    /// **top-left corner** of the rectangle, matching Prawn's convention.
    ///
    /// The vertices are drawn clockwise: top-left → top-right → bottom-right → bottom-left.
    ///
    /// # Parameters
    ///
    /// * `width`, `height`, and `radius` must be positive values.
    ///
    /// For PDF-native coordinates (bottom-left origin), use [`rounded_rect_bl`](Self::rounded_rect_bl).
    pub fn rounded_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        // Clamp radius to half the minimum dimension
        let r = radius.min(width / 2.0).min(height / 2.0);

        // Magic number for Bezier curve approximation
        const KAPPA: f64 = 0.5522847498;
        let k = r * KAPPA;

        // Prawn draws clockwise from top-left:
        // top-left corner → top-right corner → bottom-right corner → bottom-left corner
        // Note: In PDF coordinates, y increases upward, so "down" means y decreases

        // Start at top-left, just after the corner arc (on the top edge)
        self.move_to(x + r, y);

        // Top edge (left to right)
        self.line_to(x + width - r, y);

        // Top-right corner (curve down-right)
        self.curve_to(x + width - r + k, y, x + width, y - r + k, x + width, y - r);

        // Right edge (top to bottom, i.e., y decreasing)
        self.line_to(x + width, y - height + r);

        // Bottom-right corner (curve down-left)
        self.curve_to(
            x + width,
            y - height + r - k,
            x + width - r + k,
            y - height,
            x + width - r,
            y - height,
        );

        // Bottom edge (right to left)
        self.line_to(x + r, y - height);

        // Bottom-left corner (curve up-left)
        self.curve_to(
            x + r - k,
            y - height,
            x,
            y - height + r - k,
            x,
            y - height + r,
        );

        // Left edge (bottom to top, i.e., y increasing)
        self.line_to(x, y - r);

        // Top-left corner (curve up-right)
        self.curve_to(x, y - r + k, x + r - k, y, x + r, y);

        self.close_path()
    }

    /// Rounded rectangle with bottom-left origin (PDF native coordinates)
    ///
    /// This method uses PDF's native coordinate system where (x, y) specifies
    /// the **bottom-left corner** of the rectangle.
    ///
    /// # Parameters
    ///
    /// * `width`, `height`, and `radius` must be positive values.
    pub fn rounded_rect_bl(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        // Convert bottom-left to top-left for Prawn-compatible method
        self.rounded_rect(x, y + height, width, height, radius)
    }

    // Path painting operators

    /// Stroke path (S)
    pub fn stroke(&mut self) -> &mut Self {
        self.line("S")
    }

    /// Close and stroke path (s)
    pub fn close_and_stroke(&mut self) -> &mut Self {
        self.line("s")
    }

    /// Fill path (f)
    pub fn fill(&mut self) -> &mut Self {
        self.line("f")
    }

    /// Fill path (even-odd rule) (f*)
    pub fn fill_even_odd(&mut self) -> &mut Self {
        self.line("f*")
    }

    /// Fill and stroke (B)
    pub fn fill_and_stroke(&mut self) -> &mut Self {
        self.line("B")
    }

    /// End path without filling or stroking (n)
    pub fn end_path(&mut self) -> &mut Self {
        self.line("n")
    }

    // Clipping path operators

    /// Set clipping path using nonzero winding number rule (W)
    ///
    /// Modifies the current clipping path by intersecting it with the current path.
    /// Should be followed by a path-painting operator (stroke, fill, or end_path).
    pub fn clip(&mut self) -> &mut Self {
        self.line("W")
    }

    /// Set clipping path using even-odd rule (W*)
    ///
    /// Modifies the current clipping path by intersecting it with the current path.
    /// Should be followed by a path-painting operator (stroke, fill, or end_path).
    pub fn clip_even_odd(&mut self) -> &mut Self {
        self.line("W*")
    }

    // Text operators

    /// Begin text object (BT)
    pub fn begin_text(&mut self) -> &mut Self {
        self.line("BT")
    }

    /// End text object (ET)
    pub fn end_text(&mut self) -> &mut Self {
        self.line("ET")
    }

    /// Set text font and size (Tf)
    pub fn set_font(&mut self, font_name: &str, size: f64) -> &mut Self {
        self.line(&format!("/{} {} Tf", font_name, format_number(size)))
    }

    /// Move text position (Td)
    pub fn move_text_pos(&mut self, tx: f64, ty: f64) -> &mut Self {
        self.line(&format!("{} {} Td", format_number(tx), format_number(ty)))
    }

    /// Set text matrix (Tm)
    pub fn set_text_matrix(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> &mut Self {
        self.line(&format!(
            "{} {} {} {} {} {} Tm",
            format_number(a),
            format_number(b),
            format_number(c),
            format_number(d),
            format_number(e),
            format_number(f)
        ))
    }

    /// Show text (Tj)
    pub fn show_text(&mut self, text: &str) -> &mut Self {
        // Escape special characters in PDF string
        let escaped = escape_pdf_string(text);
        self.line(&format!("({}) Tj", escaped))
    }

    /// Show text using hex string (Tj) - for TrueType/CID fonts
    pub fn show_text_hex(&mut self, hex: &str) -> &mut Self {
        self.line(&format!("<{}> Tj", hex))
    }

    /// Show glyphs with adjustments (TJ) - for kerning/shaped text
    pub fn show_text_hex_adjusted(&mut self, glyphs: &[u16], adjustments: &[i32]) -> &mut Self {
        if glyphs.is_empty() {
            return self;
        }

        let mut line = String::new();
        line.push('[');
        for (idx, gid) in glyphs.iter().enumerate() {
            line.push('<');
            line.push_str(&format!("{:04X}", gid));
            line.push('>');
            if idx < glyphs.len().saturating_sub(1) {
                if let Some(adj) = adjustments.get(idx) {
                    line.push(' ');
                    line.push_str(&format_number(*adj as f64));
                }
            }
            if idx + 1 < glyphs.len() {
                line.push(' ');
            }
        }
        line.push(']');
        line.push_str(" TJ");
        self.line(&line)
    }

    /// Set text leading (TL)
    pub fn set_text_leading(&mut self, leading: f64) -> &mut Self {
        self.line(&format!("{} TL", format_number(leading)))
    }

    /// Move to next line (T*)
    pub fn next_line(&mut self) -> &mut Self {
        self.line("T*")
    }

    /// Set character spacing (Tc)
    ///
    /// Sets the spacing between characters. The value is in text space units.
    /// Default is 0. Positive values increase spacing, negative values decrease.
    pub fn set_character_spacing(&mut self, spacing: f64) -> &mut Self {
        self.line(&format!("{} Tc", format_number(spacing)))
    }

    /// Set word spacing (Tw)
    ///
    /// Sets the spacing added to the space character (U+0020).
    /// The value is in text space units. Default is 0.
    pub fn set_word_spacing(&mut self, spacing: f64) -> &mut Self {
        self.line(&format!("{} Tw", format_number(spacing)))
    }

    // Transformation operators

    /// Concatenate matrix (cm)
    pub fn concat_matrix(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> &mut Self {
        self.line(&format!(
            "{} {} {} {} {} {} cm",
            format_number(a),
            format_number(b),
            format_number(c),
            format_number(d),
            format_number(e),
            format_number(f)
        ))
    }

    // XObject operators

    /// Paint XObject (Do)
    pub fn draw_xobject(&mut self, name: &str) -> &mut Self {
        self.line(&format!("/{} Do", name))
    }
}

/// Line cap styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LineCap {
    Butt = 0,
    Round = 1,
    Square = 2,
}

/// Line join styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LineJoin {
    Miter = 0,
    Round = 1,
    Bevel = 2,
}

/// Formats a number for PDF output using ryu for optimal precision
fn format_number(n: f64) -> String {
    format_real(n)
}

/// Escapes a string for PDF
fn escape_pdf_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '(' => result.push_str("\\("),
            ')' => result.push_str("\\)"),
            '\\' => result.push_str("\\\\"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_builder() {
        let mut builder = ContentBuilder::new();
        builder
            .save_state()
            .set_stroke_color_rgb(1.0, 0.0, 0.0)
            .move_to(100.0, 100.0)
            .line_to(200.0, 200.0)
            .stroke()
            .restore_state();
        let content = builder.build();

        let s = String::from_utf8(content).unwrap();
        assert!(s.contains("q\n"));
        assert!(s.contains("1 0 0 RG"));
        assert!(s.contains("100 100 m"));
        assert!(s.contains("200 200 l"));
        assert!(s.contains("S\n"));
        assert!(s.contains("Q\n"));
    }

    #[test]
    fn test_text_content() {
        let mut builder = ContentBuilder::new();
        builder
            .begin_text()
            .set_font("Helvetica", 12.0)
            .move_text_pos(100.0, 700.0)
            .show_text("Hello World")
            .end_text();
        let content = builder.build();

        let s = String::from_utf8(content).unwrap();
        assert!(s.contains("BT"));
        assert!(s.contains("/Helvetica 12 Tf"));
        assert!(s.contains("(Hello World) Tj"));
        assert!(s.contains("ET"));
    }
}
