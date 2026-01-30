//! Inline format parser for HTML-like markup
//!
//! Parses a subset of HTML tags (similar to Prawn's inline formatting) into
//! a sequence of `TextFragment` values with the appropriate styling applied.
//!
//! Supported tags:
//! - `<b>`, `<strong>` - Bold
//! - `<i>`, `<em>` - Italic
//! - `<u>` - Underline
//! - `<strikethrough>` - Strikethrough
//! - `<sub>` - Subscript
//! - `<sup>` - Superscript
//! - `<link href="...">` or `<a href="...">` - Hyperlink
//! - `<color rgb="#RRGGBB">` - Text color
//! - `<font name="..." size="...">` - Font override
//! - `<br>` / `<br/>` - Line break (converted to `\n`)
//! - `&amp;`, `&lt;`, `&gt;` - HTML entities

use std::sync::LazyLock;

use regex::Regex;

use super::color::Color;
use super::layout::{FontStyle, TextFragment};

// Single regex that matches all recognized tags and plain text segments.
// Order matters: tags before the catch-all text pattern.
static PARSER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"\n|",
        r"<b>|</b>|<strong>|</strong>|",
        r"<i>|</i>|<em>|</em>|",
        r"<u>|</u>|",
        r"<strikethrough>|</strikethrough>|",
        r"<sub>|</sub>|<sup>|</sup>|",
        r"<link[^>]*>|</link>|",
        r"<a[^>]*>|</a>|",
        r"<color[^>]*>|</color>|",
        r"<font[^>]*>|</font>|",
        r"[^<\n]+"
    ))
    .unwrap()
});

// Helpers for extracting attribute values from tags
static HREF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href\s*=\s*["']([^"']*)["']"#).unwrap());

static COLOR_RGB_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"rgb\s*=\s*["']([^"']*)["']"#).unwrap());

static FONT_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"name\s*=\s*["']([^"']*)["']"#).unwrap());

static FONT_SIZE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"size\s*=\s*["']([^"']*)["']"#).unwrap());

// Pre-replace <br> / <br/>
static BR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<br\s*/?>").unwrap());

/// Style state that can be pushed/popped as tags open and close.
#[derive(Debug, Clone, Default)]
struct StyleState {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    superscript: bool,
    subscript: bool,
    link: Option<String>,
    color: Option<Color>,
    font: Option<String>,
    size: Option<f64>,
}

impl StyleState {
    fn font_style(&self) -> FontStyle {
        match (self.bold, self.italic) {
            (true, true) => FontStyle::BoldItalic,
            (true, false) => FontStyle::Bold,
            (false, true) => FontStyle::Italic,
            (false, false) => FontStyle::Normal,
        }
    }

    fn to_fragment(&self, text: String) -> TextFragment {
        TextFragment {
            text,
            style: self.font_style(),
            color: self.color,
            size: self.size,
            font: self.font.clone(),
            underline: self.underline,
            strikethrough: self.strikethrough,
            superscript: self.superscript,
            subscript: self.subscript,
            link: self.link.clone(),
        }
    }
}

/// Decode basic HTML entities in text.
fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Parse inline-formatted text into a sequence of `TextFragment` values.
///
/// # Example
///
/// ```rust
/// use pdfcrate::api::inline_format::parse;
///
/// let fragments = parse("Hello <b>world</b>!");
/// assert_eq!(fragments.len(), 3);
/// assert_eq!(fragments[0].text, "Hello ");
/// assert_eq!(fragments[1].text, "world");
/// assert_eq!(fragments[2].text, "!");
/// ```
pub fn parse(input: &str) -> Vec<TextFragment> {
    // Step 1: replace <br> / <br/> with newlines
    let input = BR_RE.replace_all(input, "\n");

    let mut fragments: Vec<TextFragment> = Vec::new();

    // Style stack: each opening tag pushes the current state
    let mut stack: Vec<StyleState> = Vec::new();
    let mut current = StyleState::default();

    for m in PARSER_REGEX.find_iter(&input) {
        let token = m.as_str();

        match token {
            // Bold
            "<b>" | "<strong>" => {
                stack.push(current.clone());
                current.bold = true;
            }
            "</b>" | "</strong>" => {
                if let Some(prev) = stack.pop() {
                    current.bold = prev.bold;
                }
            }

            // Italic
            "<i>" | "<em>" => {
                stack.push(current.clone());
                current.italic = true;
            }
            "</i>" | "</em>" => {
                if let Some(prev) = stack.pop() {
                    current.italic = prev.italic;
                }
            }

            // Underline
            "<u>" => {
                stack.push(current.clone());
                current.underline = true;
            }
            "</u>" => {
                if let Some(prev) = stack.pop() {
                    current.underline = prev.underline;
                }
            }

            // Strikethrough
            "<strikethrough>" => {
                stack.push(current.clone());
                current.strikethrough = true;
            }
            "</strikethrough>" => {
                if let Some(prev) = stack.pop() {
                    current.strikethrough = prev.strikethrough;
                }
            }

            // Subscript / Superscript
            "<sub>" => {
                stack.push(current.clone());
                current.subscript = true;
            }
            "</sub>" => {
                if let Some(prev) = stack.pop() {
                    current.subscript = prev.subscript;
                }
            }
            "<sup>" => {
                stack.push(current.clone());
                current.superscript = true;
            }
            "</sup>" => {
                if let Some(prev) = stack.pop() {
                    current.superscript = prev.superscript;
                }
            }

            // Closing tags for link / color / font
            "</link>" | "</a>" => {
                if let Some(prev) = stack.pop() {
                    current.link = prev.link.clone();
                }
            }
            "</color>" => {
                if let Some(prev) = stack.pop() {
                    current.color = prev.color;
                }
            }
            "</font>" => {
                if let Some(prev) = stack.pop() {
                    current.font = prev.font.clone();
                    current.size = prev.size;
                }
            }

            // Newline
            "\n" => {
                fragments.push(current.to_fragment("\n".to_string()));
            }

            // Opening tags with attributes
            _ if token.starts_with("<link") || token.starts_with("<a") => {
                stack.push(current.clone());
                if let Some(caps) = HREF_RE.captures(token) {
                    current.link = Some(caps[1].to_string());
                }
            }
            _ if token.starts_with("<color") => {
                stack.push(current.clone());
                if let Some(caps) = COLOR_RGB_RE.captures(token) {
                    current.color = Some(Color::hex(&caps[1]));
                }
            }
            _ if token.starts_with("<font") => {
                stack.push(current.clone());
                if let Some(caps) = FONT_NAME_RE.captures(token) {
                    current.font = Some(caps[1].to_string());
                }
                if let Some(caps) = FONT_SIZE_RE.captures(token) {
                    if let Ok(sz) = caps[1].parse::<f64>() {
                        current.size = Some(sz);
                    }
                }
            }

            // Plain text
            _ => {
                let text = decode_entities(token);
                fragments.push(current.to_fragment(text));
            }
        }
    }

    fragments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let frags = parse("Hello world");
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].text, "Hello world");
        assert_eq!(frags[0].style, FontStyle::Normal);
    }

    #[test]
    fn test_bold() {
        let frags = parse("Hello <b>bold</b> world");
        assert_eq!(frags.len(), 3);
        assert_eq!(frags[0].text, "Hello ");
        assert_eq!(frags[0].style, FontStyle::Normal);
        assert_eq!(frags[1].text, "bold");
        assert_eq!(frags[1].style, FontStyle::Bold);
        assert_eq!(frags[2].text, " world");
        assert_eq!(frags[2].style, FontStyle::Normal);
    }

    #[test]
    fn test_strong_tag() {
        let frags = parse("<strong>strong</strong>");
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].text, "strong");
        assert_eq!(frags[0].style, FontStyle::Bold);
    }

    #[test]
    fn test_italic() {
        let frags = parse("Hello <i>italic</i> world");
        assert_eq!(frags.len(), 3);
        assert_eq!(frags[1].text, "italic");
        assert_eq!(frags[1].style, FontStyle::Italic);
    }

    #[test]
    fn test_em_tag() {
        let frags = parse("<em>emphasis</em>");
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].style, FontStyle::Italic);
    }

    #[test]
    fn test_bold_italic_nested() {
        let frags = parse("<b><i>bold-italic</i></b>");
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].text, "bold-italic");
        assert_eq!(frags[0].style, FontStyle::BoldItalic);
    }

    #[test]
    fn test_underline() {
        let frags = parse("Hello <u>underlined</u> world");
        assert_eq!(frags.len(), 3);
        assert!(!frags[0].underline);
        assert!(frags[1].underline);
        assert_eq!(frags[1].text, "underlined");
        assert!(!frags[2].underline);
    }

    #[test]
    fn test_strikethrough() {
        let frags = parse("Hello <strikethrough>struck</strikethrough> world");
        assert_eq!(frags.len(), 3);
        assert!(!frags[0].strikethrough);
        assert!(frags[1].strikethrough);
        assert!(!frags[2].strikethrough);
    }

    #[test]
    fn test_superscript() {
        let frags = parse("x<sup>2</sup>");
        assert_eq!(frags.len(), 2);
        assert!(!frags[0].superscript);
        assert!(frags[1].superscript);
        assert_eq!(frags[1].text, "2");
    }

    #[test]
    fn test_subscript() {
        let frags = parse("H<sub>2</sub>O");
        assert_eq!(frags.len(), 3);
        assert!(frags[1].subscript);
        assert_eq!(frags[1].text, "2");
    }

    #[test]
    fn test_link_tag() {
        let frags = parse(r#"Click <link href="https://example.com">here</link> now"#);
        assert_eq!(frags.len(), 3);
        assert!(frags[0].link.is_none());
        assert_eq!(frags[1].link.as_deref(), Some("https://example.com"));
        assert_eq!(frags[1].text, "here");
        assert!(frags[2].link.is_none());
    }

    #[test]
    fn test_a_tag() {
        let frags = parse(r#"<a href="https://rust-lang.org">Rust</a>"#);
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].link.as_deref(), Some("https://rust-lang.org"));
        assert_eq!(frags[0].text, "Rust");
    }

    #[test]
    fn test_color_tag() {
        let frags = parse(r##"Normal <color rgb="#FF0000">red</color> normal"##);
        assert_eq!(frags.len(), 3);
        assert!(frags[0].color.is_none());
        assert!(frags[1].color.is_some());
        let c = frags[1].color.unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.g - 0.0).abs() < 0.01);
        assert!((c.b - 0.0).abs() < 0.01);
        assert!(frags[2].color.is_none());
    }

    #[test]
    fn test_font_tag() {
        let frags = parse(r#"Normal <font name="Courier" size="18">mono</font> normal"#);
        assert_eq!(frags.len(), 3);
        assert!(frags[0].font.is_none());
        assert_eq!(frags[1].font.as_deref(), Some("Courier"));
        assert_eq!(frags[1].size, Some(18.0));
        assert!(frags[2].font.is_none());
    }

    #[test]
    fn test_font_name_only() {
        let frags = parse(r#"<font name="Times">text</font>"#);
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].font.as_deref(), Some("Times"));
        assert!(frags[0].size.is_none());
    }

    #[test]
    fn test_br_tag() {
        let frags = parse("Line 1<br>Line 2<br/>Line 3");
        // Should produce: "Line 1", "\n", "Line 2", "\n", "Line 3"
        assert_eq!(frags.len(), 5);
        assert_eq!(frags[0].text, "Line 1");
        assert_eq!(frags[1].text, "\n");
        assert_eq!(frags[2].text, "Line 2");
        assert_eq!(frags[3].text, "\n");
        assert_eq!(frags[4].text, "Line 3");
    }

    #[test]
    fn test_html_entities() {
        let frags = parse("a &amp; b &lt; c &gt; d");
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0].text, "a & b < c > d");
    }

    #[test]
    fn test_nested_styles() {
        let frags = parse("<b>bold <i>bold-italic</i> bold</b>");
        assert_eq!(frags.len(), 3);
        assert_eq!(frags[0].style, FontStyle::Bold);
        assert_eq!(frags[0].text, "bold ");
        assert_eq!(frags[1].style, FontStyle::BoldItalic);
        assert_eq!(frags[1].text, "bold-italic");
        assert_eq!(frags[2].style, FontStyle::Bold);
        assert_eq!(frags[2].text, " bold");
    }

    #[test]
    fn test_empty_input() {
        let frags = parse("");
        assert!(frags.is_empty());
    }

    #[test]
    fn test_complex_mixed() {
        let frags = parse(
            r##"Hello <b>bold</b> and <i>italic</i> with <color rgb="#0000FF"><u>blue underline</u></color>"##,
        );
        assert_eq!(frags.len(), 6);
        assert_eq!(frags[0].text, "Hello ");
        assert_eq!(frags[1].text, "bold");
        assert_eq!(frags[1].style, FontStyle::Bold);
        assert_eq!(frags[2].text, " and ");
        assert_eq!(frags[3].text, "italic");
        assert_eq!(frags[3].style, FontStyle::Italic);
        assert_eq!(frags[4].text, " with ");
        assert_eq!(frags[5].text, "blue underline");
        assert!(frags[5].underline);
        assert!(frags[5].color.is_some());
    }

    #[test]
    fn test_newline_in_text() {
        let frags = parse("Line 1\nLine 2");
        assert_eq!(frags.len(), 3);
        assert_eq!(frags[0].text, "Line 1");
        assert_eq!(frags[1].text, "\n");
        assert_eq!(frags[2].text, "Line 2");
    }
}
