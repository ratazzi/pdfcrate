//! PDF Content Stream Operators
//!
//! Constants for PDF content stream operators.

/// Graphics state operators
pub mod graphics_state {
    pub const SAVE: &str = "q";
    pub const RESTORE: &str = "Q";
    pub const LINE_WIDTH: &str = "w";
    pub const LINE_CAP: &str = "J";
    pub const LINE_JOIN: &str = "j";
    pub const MITER_LIMIT: &str = "M";
    pub const DASH_PATTERN: &str = "d";
    pub const COLOR_RENDERING_INTENT: &str = "ri";
    pub const FLATNESS: &str = "i";
    pub const GRAPHICS_STATE: &str = "gs";
}

/// Path construction operators
pub mod path {
    pub const MOVE_TO: &str = "m";
    pub const LINE_TO: &str = "l";
    pub const CURVE_TO: &str = "c";
    pub const CURVE_TO_V: &str = "v";
    pub const CURVE_TO_Y: &str = "y";
    pub const CLOSE: &str = "h";
    pub const RECT: &str = "re";
}

/// Path painting operators
pub mod paint {
    pub const STROKE: &str = "S";
    pub const CLOSE_STROKE: &str = "s";
    pub const FILL: &str = "f";
    pub const FILL_COMPAT: &str = "F";
    pub const FILL_EVEN_ODD: &str = "f*";
    pub const FILL_STROKE: &str = "B";
    pub const FILL_STROKE_EVEN_ODD: &str = "B*";
    pub const CLOSE_FILL_STROKE: &str = "b";
    pub const CLOSE_FILL_STROKE_EVEN_ODD: &str = "b*";
    pub const END_PATH: &str = "n";
}

/// Clipping path operators
pub mod clip {
    pub const CLIP: &str = "W";
    pub const CLIP_EVEN_ODD: &str = "W*";
}

/// Text object operators
pub mod text_object {
    pub const BEGIN: &str = "BT";
    pub const END: &str = "ET";
}

/// Text state operators
pub mod text_state {
    pub const CHAR_SPACING: &str = "Tc";
    pub const WORD_SPACING: &str = "Tw";
    pub const HORIZONTAL_SCALE: &str = "Tz";
    pub const LEADING: &str = "TL";
    pub const FONT: &str = "Tf";
    pub const RENDER_MODE: &str = "Tr";
    pub const RISE: &str = "Ts";
}

/// Text positioning operators
pub mod text_position {
    pub const MOVE: &str = "Td";
    pub const MOVE_SET_LEADING: &str = "TD";
    pub const MATRIX: &str = "Tm";
    pub const NEXT_LINE: &str = "T*";
}

/// Text showing operators
pub mod text_show {
    pub const SHOW: &str = "Tj";
    pub const SHOW_ARRAY: &str = "TJ";
    pub const NEXT_LINE_SHOW: &str = "'";
    pub const NEXT_LINE_SHOW_SPACING: &str = "\"";
}

/// Color operators
pub mod color {
    pub const STROKE_COLORSPACE: &str = "CS";
    pub const FILL_COLORSPACE: &str = "cs";
    pub const STROKE_COLOR: &str = "SC";
    pub const STROKE_COLOR_N: &str = "SCN";
    pub const FILL_COLOR: &str = "sc";
    pub const FILL_COLOR_N: &str = "scn";
    pub const STROKE_GRAY: &str = "G";
    pub const FILL_GRAY: &str = "g";
    pub const STROKE_RGB: &str = "RG";
    pub const FILL_RGB: &str = "rg";
    pub const STROKE_CMYK: &str = "K";
    pub const FILL_CMYK: &str = "k";
}

/// XObject operators
pub mod xobject {
    pub const DO: &str = "Do";
}

/// Inline image operators
pub mod inline_image {
    pub const BEGIN: &str = "BI";
    pub const DATA: &str = "ID";
    pub const END: &str = "EI";
}

/// Transformation operators
pub mod transform {
    pub const CTM: &str = "cm";
}
