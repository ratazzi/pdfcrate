//! Form field PDF object generation
//!
//! This module handles creating PDF dictionaries for form fields.

use super::{FieldType, FormField, TextAlign};
use crate::objects::{PdfArray, PdfDict, PdfName, PdfObject, PdfRef};

/// Creates the widget annotation dictionary for a form field
pub fn create_widget_annotation(
    field: &FormField,
    page_ref: PdfRef,
    appearance_ref: Option<PdfRef>,
) -> PdfDict {
    let mut dict = PdfDict::new();

    // Type and Subtype
    dict.set("Type", PdfObject::Name(PdfName::new("Annot")));
    dict.set("Subtype", PdfObject::Name(PdfName::new("Widget")));

    // Rectangle
    let rect = PdfArray::from(vec![
        PdfObject::Real(field.rect[0]),
        PdfObject::Real(field.rect[1]),
        PdfObject::Real(field.rect[2]),
        PdfObject::Real(field.rect[3]),
    ]);
    dict.set("Rect", PdfObject::Array(rect));

    // Page reference
    dict.set("P", PdfObject::Reference(page_ref));

    // Field name
    dict.set("T", PdfObject::String(field.name.as_str().into()));

    // Field type
    dict.set(
        "FT",
        PdfObject::Name(PdfName::new(field.field_type.pdf_name())),
    );

    // Flags
    if field.flags.value() != 0 {
        dict.set("Ff", PdfObject::Integer(field.flags.value() as i64));
    }

    // Value
    if let Some(ref value) = field.value {
        dict.set("V", PdfObject::Name(PdfName::new(value)));
        // Default value
        dict.set("DV", PdfObject::Name(PdfName::new(value)));
    }

    // Tooltip
    if let Some(ref tooltip) = field.tooltip {
        dict.set("TU", PdfObject::String(tooltip.as_str().into()));
    }

    // Text alignment (Q)
    if field.align != TextAlign::Left {
        dict.set("Q", PdfObject::Integer(field.align as i64));
    }

    // Max length (for text fields)
    if let Some(max_len) = field.max_length {
        dict.set("MaxLen", PdfObject::Integer(max_len as i64));
    }

    // Options (for choice fields)
    if !field.options.is_empty() {
        let opts: Vec<PdfObject> = field
            .options
            .iter()
            .map(|s| PdfObject::String(s.as_str().into()))
            .collect();
        dict.set("Opt", PdfObject::Array(PdfArray::from(opts)));
    }

    // Default appearance
    let da = format!(
        "/{} {} Tf {} {} {} rg",
        field.font,
        if field.font_size == 0.0 {
            12.0
        } else {
            field.font_size
        },
        field.text_color[0],
        field.text_color[1],
        field.text_color[2]
    );
    dict.set("DA", PdfObject::String(da.into()));

    // Border style
    let mut bs = PdfDict::new();
    bs.set("W", PdfObject::Integer(1)); // 1pt border
    bs.set("S", PdfObject::Name(PdfName::new("S"))); // Solid
    dict.set("BS", PdfObject::Dict(bs));

    // Appearance characteristics (MK)
    let mut mk = PdfDict::new();
    if let Some(ref bg) = field.background_color {
        let bg_arr = PdfArray::from(vec![
            PdfObject::Real(bg[0]),
            PdfObject::Real(bg[1]),
            PdfObject::Real(bg[2]),
        ]);
        mk.set("BG", PdfObject::Array(bg_arr));
    }
    if let Some(ref bc) = field.border_color {
        let bc_arr = PdfArray::from(vec![
            PdfObject::Real(bc[0]),
            PdfObject::Real(bc[1]),
            PdfObject::Real(bc[2]),
        ]);
        mk.set("BC", PdfObject::Array(bc_arr));
    }
    // Caption for checkboxes
    if field.field_type == FieldType::CheckBox {
        mk.set("CA", PdfObject::String("4".into())); // Checkmark character
    }
    dict.set("MK", PdfObject::Dict(mk));

    // Appearance dictionary
    if let Some(ap_ref) = appearance_ref {
        let mut ap = PdfDict::new();
        ap.set("N", PdfObject::Reference(ap_ref));
        dict.set("AP", PdfObject::Dict(ap));
    }

    // Annotation flags (print, no zoom, no rotate)
    dict.set("F", PdfObject::Integer(4)); // Print flag

    dict
}

/// Creates the AcroForm dictionary for the document catalog
pub fn create_acro_form_dict(
    field_refs: &[PdfRef],
    font_refs: &[(String, PdfRef)],
    need_appearances: bool,
    default_appearance: Option<&str>,
) -> PdfDict {
    let mut dict = PdfDict::new();

    // Fields array
    let fields: Vec<PdfObject> = field_refs
        .iter()
        .map(|r| PdfObject::Reference(*r))
        .collect();
    dict.set("Fields", PdfObject::Array(PdfArray::from(fields)));

    // NeedAppearances
    if need_appearances {
        dict.set("NeedAppearances", PdfObject::Bool(true));
    }

    // Default appearance
    if let Some(da) = default_appearance {
        dict.set("DA", PdfObject::String(da.into()));
    }

    // Default resources (fonts)
    if !font_refs.is_empty() {
        let mut dr = PdfDict::new();
        let mut font_dict = PdfDict::new();

        for (name, font_ref) in font_refs {
            font_dict.set(name, PdfObject::Reference(*font_ref));
        }

        // Add standard font aliases
        add_standard_font_resources(&mut font_dict);

        dr.set("Font", PdfObject::Dict(font_dict));
        dict.set("DR", PdfObject::Dict(dr));
    }

    dict
}

/// Adds standard font resource aliases commonly used in forms
fn add_standard_font_resources(_font_dict: &mut PdfDict) {
    // These are aliases commonly used in DA strings
    // The actual font objects would need to be created separately
    // For now, we just note that forms often reference these names
}

/// Creates a text field value for the V entry
pub fn create_text_value(text: &str) -> PdfObject {
    PdfObject::String(text.into())
}

/// Creates a choice field value for the V entry
pub fn create_choice_value(selected: &str) -> PdfObject {
    PdfObject::String(selected.into())
}

/// Creates a checkbox/radio button value
pub fn create_button_value(checked: bool) -> PdfObject {
    if checked {
        PdfObject::Name(PdfName::new("Yes"))
    } else {
        PdfObject::Name(PdfName::new("Off"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_widget_annotation() {
        let field = FormField::text("test", [100.0, 700.0, 300.0, 720.0]);
        let page_ref = PdfRef::new(1);

        let dict = create_widget_annotation(&field, page_ref, None);

        assert_eq!(dict.get_type(), Some("Annot"));
        assert!(dict.get("Rect").is_some());
        assert!(dict.get("T").is_some());
    }

    #[test]
    fn test_create_acro_form_dict() {
        let field_refs = vec![PdfRef::new(5), PdfRef::new(6)];
        let font_refs = vec![];

        let dict = create_acro_form_dict(&field_refs, &font_refs, true, Some("/Helv 12 Tf 0 g"));

        assert!(dict.get("Fields").is_some());
        assert!(dict.get("NeedAppearances").is_some());
    }
}
