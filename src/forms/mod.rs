//! PDF AcroForms support
//!
//! This module provides support for creating interactive PDF forms.
//!
//! # Form Field Types
//!
//! - `TextField` - Single or multi-line text input
//! - `CheckBox` - Boolean checkbox
//! - `RadioGroup` - Mutually exclusive radio buttons
//! - `Dropdown` - Combo box / dropdown list
//! - `ListBox` - Scrollable list selection
//! - `Button` - Push button (for actions)
//!
//! # Example
//!
//! ```rust,ignore
//! use pdf_rs::prelude::*;
//!
//! let mut doc = Document::new();
//! doc.add_text_field("name", [100.0, 700.0, 300.0, 720.0])?;
//! doc.add_checkbox("agree", [100.0, 650.0, 120.0, 670.0], false)?;
//! ```

mod appearance;
mod fields;

pub use appearance::*;
pub use fields::*;

use crate::objects::PdfDict;

/// Form field flags (PDF spec Table 221)
#[derive(Debug, Clone, Copy, Default)]
pub struct FieldFlags(u32);

impl FieldFlags {
    pub const NONE: u32 = 0;
    /// Field is read-only
    pub const READ_ONLY: u32 = 1 << 0;
    /// Field is required
    pub const REQUIRED: u32 = 1 << 1;
    /// Field should not be exported
    pub const NO_EXPORT: u32 = 1 << 2;

    // Text field flags (Table 226)
    /// Multi-line text field
    pub const MULTILINE: u32 = 1 << 12;
    /// Password field (characters obscured)
    pub const PASSWORD: u32 = 1 << 13;
    /// File select field
    pub const FILE_SELECT: u32 = 1 << 20;
    /// Do not spell check
    pub const DO_NOT_SPELL_CHECK: u32 = 1 << 22;
    /// Do not scroll
    pub const DO_NOT_SCROLL: u32 = 1 << 23;
    /// Comb of characters (fixed width)
    pub const COMB: u32 = 1 << 24;
    /// Rich text
    pub const RICH_TEXT: u32 = 1 << 25;

    // Button flags (Table 225)
    /// No toggle to off (radio buttons)
    pub const NO_TOGGLE_TO_OFF: u32 = 1 << 14;
    /// Radio button (vs checkbox)
    pub const RADIO: u32 = 1 << 15;
    /// Push button
    pub const PUSH_BUTTON: u32 = 1 << 16;
    /// Radio buttons in unison
    pub const RADIOS_IN_UNISON: u32 = 1 << 25;

    // Choice field flags (Table 227)
    /// Combo box (vs list box)
    pub const COMBO: u32 = 1 << 17;
    /// Editable combo box
    pub const EDIT: u32 = 1 << 18;
    /// Sort options alphabetically
    pub const SORT: u32 = 1 << 19;
    /// Multi-select list
    pub const MULTI_SELECT: u32 = 1 << 21;
    /// Commit on selection change
    pub const COMMIT_ON_SEL_CHANGE: u32 = 1 << 26;

    pub fn new(flags: u32) -> Self {
        FieldFlags(flags)
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }

    pub fn clear(&mut self, flag: u32) {
        self.0 &= !flag;
    }

    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }
}

/// Text alignment for form fields
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Left = 0,
    Center = 1,
    Right = 2,
}

/// Rectangle bounds [x1, y1, x2, y2]
pub type Rect = [f64; 4];

/// A form field definition (before being added to document)
#[derive(Debug, Clone)]
pub struct FormField {
    /// Field name (unique identifier)
    pub name: String,
    /// Field type
    pub field_type: FieldType,
    /// Bounding rectangle [x1, y1, x2, y2]
    pub rect: Rect,
    /// Field flags
    pub flags: FieldFlags,
    /// Default value
    pub value: Option<String>,
    /// Tooltip / alternate text
    pub tooltip: Option<String>,
    /// Text alignment
    pub align: TextAlign,
    /// Maximum length (for text fields)
    pub max_length: Option<u32>,
    /// Options (for choice fields)
    pub options: Vec<String>,
    /// Font name to use
    pub font: String,
    /// Font size (0 = auto)
    pub font_size: f64,
    /// Border color (RGB)
    pub border_color: Option<[f64; 3]>,
    /// Background color (RGB)
    pub background_color: Option<[f64; 3]>,
    /// Text color (RGB)
    pub text_color: [f64; 3],
    /// Page index (0-based) this field belongs to
    pub page_index: usize,
}

impl FormField {
    /// Creates a new text field
    pub fn text(name: impl Into<String>, rect: Rect) -> Self {
        FormField {
            name: name.into(),
            field_type: FieldType::Text,
            rect,
            flags: FieldFlags::default(),
            value: None,
            tooltip: None,
            align: TextAlign::Left,
            max_length: None,
            options: Vec::new(),
            font: "Helvetica".to_string(),
            font_size: 0.0, // auto
            border_color: Some([0.0, 0.0, 0.0]),
            background_color: Some([1.0, 1.0, 1.0]),
            text_color: [0.0, 0.0, 0.0],
            page_index: 0,
        }
    }

    /// Creates a new checkbox field
    pub fn checkbox(name: impl Into<String>, rect: Rect, checked: bool) -> Self {
        FormField {
            name: name.into(),
            field_type: FieldType::CheckBox,
            rect,
            flags: FieldFlags::default(),
            value: Some(if checked { "Yes" } else { "Off" }.to_string()),
            tooltip: None,
            align: TextAlign::Left,
            max_length: None,
            options: Vec::new(),
            font: "ZapfDingbats".to_string(),
            font_size: 0.0,
            border_color: Some([0.0, 0.0, 0.0]),
            background_color: Some([1.0, 1.0, 1.0]),
            text_color: [0.0, 0.0, 0.0],
            page_index: 0,
        }
    }

    /// Creates a new dropdown/combo box field
    pub fn dropdown(name: impl Into<String>, rect: Rect, options: Vec<String>) -> Self {
        let mut flags = FieldFlags::default();
        flags.set(FieldFlags::COMBO);

        FormField {
            name: name.into(),
            field_type: FieldType::Choice,
            rect,
            flags,
            value: options.first().cloned(),
            tooltip: None,
            align: TextAlign::Left,
            max_length: None,
            options,
            font: "Helvetica".to_string(),
            font_size: 0.0,
            border_color: Some([0.0, 0.0, 0.0]),
            background_color: Some([1.0, 1.0, 1.0]),
            text_color: [0.0, 0.0, 0.0],
            page_index: 0,
        }
    }

    /// Creates a new list box field
    pub fn listbox(name: impl Into<String>, rect: Rect, options: Vec<String>) -> Self {
        FormField {
            name: name.into(),
            field_type: FieldType::Choice,
            rect,
            flags: FieldFlags::default(),
            value: None,
            tooltip: None,
            align: TextAlign::Left,
            max_length: None,
            options,
            font: "Helvetica".to_string(),
            font_size: 0.0,
            border_color: Some([0.0, 0.0, 0.0]),
            background_color: Some([1.0, 1.0, 1.0]),
            text_color: [0.0, 0.0, 0.0],
            page_index: 0,
        }
    }

    // Builder methods

    /// Sets the field as read-only
    pub fn read_only(mut self) -> Self {
        self.flags.set(FieldFlags::READ_ONLY);
        self
    }

    /// Sets the field as required
    pub fn required(mut self) -> Self {
        self.flags.set(FieldFlags::REQUIRED);
        self
    }

    /// Sets the default value
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Sets the tooltip
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Sets text alignment
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Sets maximum length for text fields
    pub fn with_max_length(mut self, max: u32) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Sets multiline mode for text fields
    pub fn multiline(mut self) -> Self {
        self.flags.set(FieldFlags::MULTILINE);
        self
    }

    /// Sets password mode for text fields
    pub fn password(mut self) -> Self {
        self.flags.set(FieldFlags::PASSWORD);
        self
    }

    /// Sets the font
    pub fn with_font(mut self, font: impl Into<String>, size: f64) -> Self {
        self.font = font.into();
        self.font_size = size;
        self
    }

    /// Sets border color
    pub fn with_border_color(mut self, r: f64, g: f64, b: f64) -> Self {
        self.border_color = Some([r, g, b]);
        self
    }

    /// Removes border
    pub fn no_border(mut self) -> Self {
        self.border_color = None;
        self
    }

    /// Sets background color
    pub fn with_background_color(mut self, r: f64, g: f64, b: f64) -> Self {
        self.background_color = Some([r, g, b]);
        self
    }

    /// Sets text color
    pub fn with_text_color(mut self, r: f64, g: f64, b: f64) -> Self {
        self.text_color = [r, g, b];
        self
    }
}

/// Form field type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// Text input (Tx)
    Text,
    /// Checkbox or radio button (Btn)
    CheckBox,
    /// Radio button group (Btn with Radio flag)
    Radio,
    /// Push button (Btn with PushButton flag)
    Button,
    /// Dropdown or list box (Ch)
    Choice,
    /// Signature field (Sig)
    Signature,
}

impl FieldType {
    /// Returns the PDF field type name
    pub fn pdf_name(&self) -> &'static str {
        match self {
            FieldType::Text => "Tx",
            FieldType::CheckBox | FieldType::Radio | FieldType::Button => "Btn",
            FieldType::Choice => "Ch",
            FieldType::Signature => "Sig",
        }
    }
}

/// AcroForm structure for the document
#[derive(Debug, Default)]
pub struct AcroForm {
    /// Form fields
    pub fields: Vec<FormField>,
    /// Need appearances flag
    pub need_appearances: bool,
    /// Signature flags
    pub sig_flags: u32,
    /// Default appearance string
    pub default_appearance: Option<String>,
    /// Default resources
    pub default_resources: Option<PdfDict>,
}

impl AcroForm {
    /// Creates a new empty AcroForm
    pub fn new() -> Self {
        AcroForm {
            fields: Vec::new(),
            need_appearances: true,
            sig_flags: 0,
            default_appearance: Some("/Helv 0 Tf 0 g".to_string()),
            default_resources: None,
        }
    }

    /// Adds a field to the form
    pub fn add_field(&mut self, field: FormField) {
        self.fields.push(field);
    }

    /// Returns true if the form has any fields
    pub fn has_fields(&self) -> bool {
        !self.fields.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_flags() {
        let mut flags = FieldFlags::default();
        assert!(!flags.has(FieldFlags::READ_ONLY));

        flags.set(FieldFlags::READ_ONLY);
        assert!(flags.has(FieldFlags::READ_ONLY));

        flags.clear(FieldFlags::READ_ONLY);
        assert!(!flags.has(FieldFlags::READ_ONLY));
    }

    #[test]
    fn test_text_field() {
        let field = FormField::text("name", [100.0, 700.0, 300.0, 720.0])
            .with_value("John Doe")
            .required();

        assert_eq!(field.name, "name");
        assert_eq!(field.field_type, FieldType::Text);
        assert!(field.flags.has(FieldFlags::REQUIRED));
        assert_eq!(field.value, Some("John Doe".to_string()));
    }

    #[test]
    fn test_checkbox_field() {
        let field = FormField::checkbox("agree", [100.0, 650.0, 120.0, 670.0], true);

        assert_eq!(field.field_type, FieldType::CheckBox);
        assert_eq!(field.value, Some("Yes".to_string()));
    }

    #[test]
    fn test_dropdown_field() {
        let options = vec!["Option 1".to_string(), "Option 2".to_string()];
        let field = FormField::dropdown("choice", [100.0, 600.0, 300.0, 620.0], options);

        assert_eq!(field.field_type, FieldType::Choice);
        assert!(field.flags.has(FieldFlags::COMBO));
        assert_eq!(field.options.len(), 2);
    }
}
