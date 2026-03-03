// Ruby bindings for pdfcrate (Prawn-compatible API)

use magnus::{function, method, prelude::*, Error, RArray, RHash, Ruby, Symbol, Value};
use std::cell::RefCell;
use std::collections::HashMap;

use pdfcrate::api::layout::{
    Color as RustColor, GridOptions as RustGridOptions, LayoutDocument, Margin as RustMargin,
    Overflow as RustOverflow, TextAlign as RustTextAlign, TextFragment as RustTextFragment,
};
use pdfcrate::api::page::PageSize as RustPageSize;
use pdfcrate::api::Document as RustDocument;

fn rt_err(msg: impl Into<String>) -> Error {
    Error::new(magnus::exception::runtime_error(), msg.into())
}

fn arg_err(msg: impl Into<String>) -> Error {
    Error::new(magnus::exception::arg_error(), msg.into())
}

fn parse_hex_color(hex: &str) -> (f64, f64, f64) {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64 / 255.0;
        (r, g, b)
    } else {
        (0.0, 0.0, 0.0)
    }
}

fn symbol_to_align(sym: &str) -> RustTextAlign {
    match sym {
        "left" => RustTextAlign::Left,
        "center" => RustTextAlign::Center,
        "right" => RustTextAlign::Right,
        "justify" => RustTextAlign::Justify,
        _ => RustTextAlign::Left,
    }
}

fn parse_page_size(name: &str) -> RustPageSize {
    match name {
        "A4" | "a4" => RustPageSize::A4,
        "A3" | "a3" => RustPageSize::A3,
        "A5" | "a5" => RustPageSize::A5,
        "LETTER" | "Letter" | "letter" => RustPageSize::Letter,
        "LEGAL" | "Legal" | "legal" => RustPageSize::Legal,
        _ => RustPageSize::A4,
    }
}

fn extract_point(val: Value) -> Result<(f64, f64), Error> {
    let arr = RArray::try_convert(val).map_err(|_| arg_err("expected [x, y] array"))?;
    if arr.len() != 2 {
        return Err(arg_err("expected [x, y] array with 2 elements"));
    }
    let x: f64 = arr.entry(0)?;
    let y: f64 = arr.entry(1)?;
    Ok((x, y))
}

// Helper to get optional key from RHash
fn hash_get_string(hash: &RHash, key: &str) -> Option<String> {
    hash.get(Symbol::new(key))
        .and_then(|v: Value| String::try_convert(v).ok())
}

fn hash_get_f64(hash: &RHash, key: &str) -> Option<f64> {
    hash.get(Symbol::new(key))
        .and_then(|v: Value| f64::try_convert(v).ok())
}

fn hash_get_symbol_name(hash: &RHash, key: &str) -> Option<String> {
    hash.get(Symbol::new(key)).and_then(|v: Value| {
        Symbol::try_convert(v)
            .ok()
            .and_then(|s| s.name().ok())
            .map(|n| n.to_string())
    })
}

enum DocumentInner {
    Layout(Box<LayoutDocument>),
    Consumed,
}

#[derive(Default, Clone)]
struct FontFamilyMap {
    families: HashMap<String, HashMap<String, String>>,
}

impl FontFamilyMap {
    fn register(&mut self, name: &str, styles: HashMap<String, String>) {
        self.families.insert(name.to_string(), styles);
    }

    fn resolve(&self, family: &str, style: &str) -> Option<&str> {
        self.families
            .get(family)
            .and_then(|m| m.get(style))
            .map(|s| s.as_str())
    }
}

#[magnus::wrap(class = "Pdfcrate::Document")]
struct Document {
    inner: RefCell<DocumentInner>,
    fill_color: RefCell<(f64, f64, f64)>,
    stroke_color: RefCell<(f64, f64, f64)>,
    line_width: RefCell<f64>,
    current_font: RefCell<String>,
    current_font_size: RefCell<f64>,
    fill_alpha: RefCell<f64>,
    stroke_alpha: RefCell<f64>,
    dash_pattern: RefCell<Option<(Vec<f64>, f64)>>,
    font_families: RefCell<FontFamilyMap>,
    // Store page dimensions from creation time
    page_width: f64,
    page_height: f64,
}

impl Document {
    // Pdfcrate::Document.new(page_size:, margin:, info:)
    fn ruby_new(args: &[Value]) -> Result<Self, Error> {
        let kwargs = if args.is_empty() {
            RHash::new()
        } else {
            RHash::try_convert(args[0]).unwrap_or_else(|_| RHash::new())
        };
        // Parse page_size
        let (page_size, pw, ph) = if let Some(ps_val) = kwargs.get(Symbol::new("page_size")) {
            let ps_val: Value = ps_val;
            if let Ok(s) = String::try_convert(ps_val) {
                let ps = parse_page_size(&s);
                let (w, h) = ps.dimensions(pdfcrate::api::page::PageLayout::Portrait);
                (ps, w, h)
            } else if let Ok(arr) = RArray::try_convert(ps_val) {
                if arr.len() == 2 {
                    let w: f64 = arr.entry(0).unwrap_or(595.0);
                    let h: f64 = arr.entry(1).unwrap_or(842.0);
                    (RustPageSize::Custom(w, h), w, h)
                } else {
                    (RustPageSize::A4, 595.0, 842.0)
                }
            } else {
                (RustPageSize::A4, 595.0, 842.0)
            }
        } else {
            (RustPageSize::A4, 595.0, 842.0)
        };

        // Parse margin
        let margin = if let Some(m_val) = kwargs.get(Symbol::new("margin")) {
            let m_val: Value = m_val;
            if let Ok(n) = f64::try_convert(m_val) {
                RustMargin::all(n)
            } else if let Ok(arr) = RArray::try_convert(m_val) {
                match arr.len() {
                    1 => RustMargin::all(arr.entry::<f64>(0).unwrap_or(36.0)),
                    2 => RustMargin::symmetric(
                        arr.entry::<f64>(0).unwrap_or(36.0),
                        arr.entry::<f64>(1).unwrap_or(36.0),
                    ),
                    4 => RustMargin::new(
                        arr.entry::<f64>(0).unwrap_or(36.0),
                        arr.entry::<f64>(1).unwrap_or(36.0),
                        arr.entry::<f64>(2).unwrap_or(36.0),
                        arr.entry::<f64>(3).unwrap_or(36.0),
                    ),
                    _ => RustMargin::all(36.0),
                }
            } else {
                RustMargin::all(36.0)
            }
        } else {
            RustMargin::all(36.0)
        };

        let mut doc = RustDocument::new();
        doc.page_size(page_size);

        // Apply info metadata
        if let Some(info_val) = kwargs.get(Symbol::new("info")) {
            let info_val: Value = info_val;
            if let Ok(info) = RHash::try_convert(info_val) {
                if let Some(title) = hash_get_string(&info, "Title") {
                    doc.title(&title);
                }
                if let Some(author) = hash_get_string(&info, "Author") {
                    doc.author(&author);
                }
            }
        }

        let layout = LayoutDocument::with_margin(doc, margin);

        Ok(Self {
            inner: RefCell::new(DocumentInner::Layout(Box::new(layout))),
            fill_color: RefCell::new((0.0, 0.0, 0.0)),
            stroke_color: RefCell::new((0.0, 0.0, 0.0)),
            line_width: RefCell::new(1.0),
            current_font: RefCell::new("Helvetica".to_string()),
            current_font_size: RefCell::new(12.0),
            fill_alpha: RefCell::new(1.0),
            stroke_alpha: RefCell::new(1.0),
            dash_pattern: RefCell::new(None),
            font_families: RefCell::new(FontFamilyMap::default()),
            page_width: pw,
            page_height: ph,
        })
    }

    fn with_doc<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut RustDocument) -> R,
    {
        let mut guard = self.inner.borrow_mut();
        match &mut *guard {
            DocumentInner::Layout(layout) => Ok(f(layout.inner_mut())),
            DocumentInner::Consumed => Err(rt_err("Document already consumed")),
        }
    }

    fn with_layout<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut LayoutDocument) -> R,
    {
        let mut guard = self.inner.borrow_mut();
        match &mut *guard {
            DocumentInner::Layout(layout) => Ok(f(layout)),
            DocumentInner::Consumed => Err(rt_err("Document already consumed")),
        }
    }

    fn with_layout_ref<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&LayoutDocument) -> R,
    {
        let guard = self.inner.borrow();
        match &*guard {
            DocumentInner::Layout(layout) => Ok(f(layout)),
            DocumentInner::Consumed => Err(rt_err("Document already consumed")),
        }
    }

    /// Returns the current bounding box offset (left, bottom) for coordinate translation.
    /// Inside a `canvas` block, this returns (0, 0).
    fn bounds_offset(&self) -> Result<(f64, f64), Error> {
        self.with_layout_ref(|layout| {
            let b = layout.bounds();
            (b.absolute_left(), b.absolute_bottom())
        })
    }

    fn draw_shape<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnOnce(&mut RustDocument),
    {
        let fa = *self.fill_alpha.borrow();
        let sa = *self.stroke_alpha.borrow();
        self.with_doc(|doc| {
            if fa < 1.0 || sa < 1.0 {
                doc.transparent(fa, sa, f);
            } else {
                f(doc);
            }
        })
    }

    fn resolve_font(&self, name: &str, style: &str) -> String {
        let families = self.font_families.borrow();
        if let Some(path) = families.resolve(name, style) {
            return path.to_string();
        }
        match (name, style) {
            ("Helvetica", "bold") => "Helvetica-Bold".to_string(),
            ("Helvetica", "italic") => "Helvetica-Oblique".to_string(),
            ("Helvetica", "bold_italic") => "Helvetica-BoldOblique".to_string(),
            ("Times-Roman" | "Times", "bold") => "Times-Bold".to_string(),
            ("Times-Roman" | "Times", "italic") => "Times-Italic".to_string(),
            ("Times-Roman" | "Times", "bold_italic") => "Times-BoldItalic".to_string(),
            ("Courier", "bold") => "Courier-Bold".to_string(),
            ("Courier", "italic") => "Courier-Oblique".to_string(),
            ("Courier", "bold_italic") => "Courier-BoldOblique".to_string(),
            _ => name.to_string(),
        }
    }

    fn apply_font(&self) -> Result<(), Error> {
        let font_name = self.current_font.borrow().clone();
        let font_size = *self.current_font_size.borrow();
        self.with_doc(|doc| {
            doc.font(&font_name).size(font_size);
        })
    }

    // save_as(path)
    fn save_as(&self, path: String) -> Result<(), Error> {
        let bytes = self.render_bytes()?;
        std::fs::write(&path, bytes).map_err(|e| rt_err(format!("Failed to write file: {}", e)))?;
        Ok(())
    }

    fn render_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut guard = self.inner.borrow_mut();
        let inner = std::mem::replace(&mut *guard, DocumentInner::Consumed);
        match inner {
            DocumentInner::Layout(layout) => {
                let mut doc = layout.into_inner();
                doc.render()
                    .map_err(|e| rt_err(format!("Render error: {}", e)))
            }
            DocumentInner::Consumed => Err(rt_err("Document already consumed")),
        }
    }

    fn start_new_page(&self) -> Result<(), Error> {
        self.with_layout(|layout| {
            layout.start_new_page();
        })
    }

    fn set_fill_color(&self, hex: String) -> Result<(), Error> {
        *self.fill_color.borrow_mut() = parse_hex_color(&hex);
        Ok(())
    }

    fn set_stroke_color(&self, hex: String) -> Result<(), Error> {
        *self.stroke_color.borrow_mut() = parse_hex_color(&hex);
        Ok(())
    }

    fn set_line_width(&self, width: f64) -> Result<(), Error> {
        *self.line_width.borrow_mut() = width;
        Ok(())
    }

    // dash(on, space: off) - accepts kwargs hash
    fn set_dash(&self, on: f64, kwargs: RHash) -> Result<(), Error> {
        let off = hash_get_f64(&kwargs, "space").unwrap_or(on);
        *self.dash_pattern.borrow_mut() = Some((vec![on, off], 0.0));
        Ok(())
    }

    fn undash(&self) -> Result<(), Error> {
        *self.dash_pattern.borrow_mut() = None;
        Ok(())
    }

    // font('name', size: n, style: :bold) { block }
    fn set_font(&self, args: &[Value]) -> Result<Value, Error> {
        let ruby = unsafe { Ruby::get_unchecked() };

        if args.is_empty() {
            return Err(arg_err("font requires at least 1 argument"));
        }

        let name =
            String::try_convert(args[0]).map_err(|_| arg_err("font name must be a string"))?;

        // Parse optional kwargs hash (last arg if it's a hash)
        let mut size_opt: Option<f64> = None;
        let mut style_str = "normal".to_string();

        if args.len() > 1 {
            if let Ok(hash) = RHash::try_convert(args[args.len() - 1]) {
                size_opt = hash_get_f64(&hash, "size");
                if let Some(s) = hash_get_symbol_name(&hash, "style") {
                    style_str = s;
                }
            }
        }

        let resolved = self.resolve_font(&name, &style_str);

        let old_font = self.current_font.borrow().clone();
        let old_size = *self.current_font_size.borrow();

        *self.current_font.borrow_mut() = resolved;
        if let Some(sz) = size_opt {
            *self.current_font_size.borrow_mut() = sz;
        }
        self.apply_font()?;

        let block = ruby.block_proc();
        if let Ok(block) = block {
            let result = block.call::<_, Value>(());
            *self.current_font.borrow_mut() = old_font;
            *self.current_font_size.borrow_mut() = old_size;
            self.apply_font()?;
            return result.map_err(|e| rt_err(format!("block error: {}", e)));
        }

        Ok(ruby.qnil().as_value())
    }

    // width_of(text) or width_of(text, size: n)
    fn width_of(&self, args: &[Value]) -> Result<f64, Error> {
        if args.is_empty() {
            return Err(arg_err("width_of requires text argument"));
        }
        let text = String::try_convert(args[0]).map_err(|_| arg_err("text must be a string"))?;

        let mut size_opt: Option<f64> = None;
        if args.len() > 1 {
            if let Ok(hash) = RHash::try_convert(args[args.len() - 1]) {
                size_opt = hash_get_f64(&hash, "size");
            }
        }

        if let Some(sz) = size_opt {
            let old_size = *self.current_font_size.borrow();
            *self.current_font_size.borrow_mut() = sz;
            self.apply_font()?;
            let width = self.with_layout_ref(|layout| layout.measure_text(&text))?;
            *self.current_font_size.borrow_mut() = old_size;
            self.apply_font()?;
            Ok(width)
        } else {
            self.with_layout_ref(|layout| layout.measure_text(&text))
        }
    }

    // draw_text text, at: [x, y]
    fn draw_text(&self, args: &[Value]) -> Result<(), Error> {
        if args.is_empty() {
            return Err(arg_err("draw_text requires text argument"));
        }
        let text = String::try_convert(args[0]).map_err(|_| arg_err("text must be a string"))?;

        let mut at_found = false;
        if args.len() > 1 {
            if let Ok(hash) = RHash::try_convert(args[args.len() - 1]) {
                if let Some(at_val) = hash.get(Symbol::new("at")) {
                    at_found = true;
                    let at_val: Value = at_val;
                    let (x, y) = extract_point(at_val)?;
                    // Prawn's draw_text at: is relative to the current bounding box
                    self.with_layout(|layout| {
                        let bounds_left = layout.bounds().absolute_left();
                        let bounds_bottom = layout.bounds().absolute_bottom();
                        layout
                            .inner_mut()
                            .text_at(&text, [bounds_left + x, bounds_bottom + y]);
                    })?;
                }
            }
        }

        if !at_found {
            return Err(arg_err("draw_text requires at: [x, y]"));
        }

        Ok(())
    }

    // text text, align: :center, leading: n
    fn text_flow(&self, args: &[Value]) -> Result<(), Error> {
        if args.is_empty() {
            return Err(arg_err("text requires text argument"));
        }
        let text = String::try_convert(args[0]).map_err(|_| arg_err("text must be a string"))?;

        let mut align_sym: Option<String> = None;
        let mut leading_pt: Option<f64> = None;

        if args.len() > 1 {
            if let Ok(hash) = RHash::try_convert(args[args.len() - 1]) {
                align_sym = hash_get_symbol_name(&hash, "align");
                leading_pt = hash_get_f64(&hash, "leading");
            }
        }

        self.with_layout(|layout| {
            // Save and set alignment
            if let Some(ref sym) = align_sym {
                layout.align(symbol_to_align(sym));
            }

            // Save leading and convert Prawn additive leading to pdfcrate multiplicative
            let old_leading = layout.line_height() / layout.font_height();
            if let Some(extra) = leading_pt {
                let font_h = layout.font_height();
                if font_h > 0.0 {
                    layout.leading(1.0 + extra / font_h);
                }
            }

            layout.text_wrap(&text);

            // Restore leading
            if leading_pt.is_some() {
                layout.leading(old_leading);
            }
            // Restore alignment
            if align_sym.is_some() {
                layout.align(RustTextAlign::Left);
            }
        })?;

        Ok(())
    }

    // text_box text, at:, width:, height:, overflow:, min_font_size:
    fn text_box_method(&self, args: &[Value]) -> Result<Value, Error> {
        let ruby = unsafe { Ruby::get_unchecked() };
        if args.is_empty() {
            return Err(arg_err("text_box requires text argument"));
        }
        let text = String::try_convert(args[0]).map_err(|_| arg_err("text must be a string"))?;

        let mut point = [0.0f64, 0.0f64];
        let mut width = 200.0f64;
        let mut height = 100.0f64;
        let mut overflow = RustOverflow::Truncate;

        if args.len() > 1 {
            if let Ok(hash) = RHash::try_convert(args[args.len() - 1]) {
                if let Some(at_val) = hash.get(Symbol::new("at")) {
                    let at_val: Value = at_val;
                    let (x, y) = extract_point(at_val)?;
                    point = [x, y];
                }
                if let Some(w) = hash_get_f64(&hash, "width") {
                    width = w;
                }
                if let Some(h) = hash_get_f64(&hash, "height") {
                    height = h;
                }
                if let Some(min_sz) = hash_get_f64(&hash, "min_font_size") {
                    overflow = RustOverflow::ShrinkToFit(min_sz);
                } else if let Some(ov_name) = hash_get_symbol_name(&hash, "overflow") {
                    overflow = match ov_name.as_str() {
                        "truncate" => RustOverflow::Truncate,
                        "shrink_to_fit" => RustOverflow::ShrinkToFit(6.0),
                        "expand" => RustOverflow::Expand,
                        _ => RustOverflow::Truncate,
                    };
                }
            }
        }

        self.with_layout(|layout| {
            layout.text_box(&text, point, width, height, overflow);
        })?;

        Ok(ruby.qnil().as_value())
    }

    // formatted_text [{text:, styles:, color:, font:}, ...]
    fn formatted_text(&self, fragments: RArray) -> Result<(), Error> {
        let mut rust_frags = Vec::new();

        for i in 0..fragments.len() {
            let hash: RHash = fragments.entry(i as isize)?;

            let text: String = hash
                .get(Symbol::new("text"))
                .and_then(|v: Value| String::try_convert(v).ok())
                .unwrap_or_default();

            let mut frag = RustTextFragment::new(&text);

            // Parse styles array
            if let Some(styles_val) = hash.get(Symbol::new("styles")) {
                let styles_val: Value = styles_val;
                if let Ok(styles) = RArray::try_convert(styles_val) {
                    for j in 0..styles.len() {
                        if let Ok(sym) = styles.entry::<Symbol>(j as isize) {
                            if let Ok(name) = sym.name() {
                                match name.as_ref() {
                                    "bold" => {
                                        frag = frag.style(pdfcrate::api::layout::FontStyle::Bold);
                                    }
                                    "italic" => {
                                        frag = frag.style(pdfcrate::api::layout::FontStyle::Italic);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            if let Some(color_str) = hash
                .get(Symbol::new("color"))
                .and_then(|v: Value| String::try_convert(v).ok())
            {
                let (r, g, b) = parse_hex_color(&color_str);
                frag = frag.color(RustColor::rgb(r, g, b));
            }

            if let Some(font_name) = hash
                .get(Symbol::new("font"))
                .and_then(|v: Value| String::try_convert(v).ok())
            {
                frag = frag.font(&font_name);
            }

            if let Some(size) = hash
                .get(Symbol::new("size"))
                .and_then(|v: Value| f64::try_convert(v).ok())
            {
                frag = frag.size(size);
            }

            rust_frags.push(frag);
        }

        self.with_layout(|layout| {
            layout.formatted_text(&rust_frags);
        })
    }

    // fill_rectangle [x,y], w, h — point is top-left (Prawn convention)
    // All shape methods translate coordinates relative to current bounding box (Prawn-compatible)
    fn fill_rectangle(&self, pos: Value, width: f64, height: f64) -> Result<(), Error> {
        let (x, y) = extract_point(pos)?;
        let (ox, oy) = self.bounds_offset()?;
        let fc = *self.fill_color.borrow();
        self.draw_shape(|doc| {
            doc.fill(|ctx| {
                ctx.color(fc).rectangle([ox + x, oy + y], width, height);
            });
        })
    }

    fn stroke_rectangle(&self, pos: Value, width: f64, height: f64) -> Result<(), Error> {
        let (x, y) = extract_point(pos)?;
        let (ox, oy) = self.bounds_offset()?;
        let sc = *self.stroke_color.borrow();
        let lw = *self.line_width.borrow();
        self.draw_shape(|doc| {
            doc.stroke(|ctx| {
                ctx.color(sc)
                    .line_width(lw)
                    .rectangle([ox + x, oy + y], width, height);
            });
        })
    }

    fn fill_rounded_rectangle(
        &self,
        pos: Value,
        width: f64,
        height: f64,
        radius: f64,
    ) -> Result<(), Error> {
        let (x, y) = extract_point(pos)?;
        let (ox, oy) = self.bounds_offset()?;
        let fc = *self.fill_color.borrow();
        self.draw_shape(|doc| {
            doc.fill(|ctx| {
                ctx.color(fc)
                    .rounded_rectangle([ox + x, oy + y], width, height, radius);
            });
        })
    }

    fn stroke_rounded_rectangle(
        &self,
        pos: Value,
        width: f64,
        height: f64,
        radius: f64,
    ) -> Result<(), Error> {
        let (x, y) = extract_point(pos)?;
        let (ox, oy) = self.bounds_offset()?;
        let sc = *self.stroke_color.borrow();
        let lw = *self.line_width.borrow();
        self.draw_shape(|doc| {
            doc.stroke(|ctx| {
                ctx.color(sc)
                    .line_width(lw)
                    .rounded_rectangle([ox + x, oy + y], width, height, radius);
            });
        })
    }

    fn fill_circle(&self, center: Value, radius: f64) -> Result<(), Error> {
        let (cx, cy) = extract_point(center)?;
        let (ox, oy) = self.bounds_offset()?;
        let fc = *self.fill_color.borrow();
        self.draw_shape(|doc| {
            doc.fill(|ctx| {
                ctx.color(fc).circle([ox + cx, oy + cy], radius);
            });
        })
    }

    fn stroke_circle(&self, center: Value, radius: f64) -> Result<(), Error> {
        let (cx, cy) = extract_point(center)?;
        let (ox, oy) = self.bounds_offset()?;
        let sc = *self.stroke_color.borrow();
        let lw = *self.line_width.borrow();
        self.draw_shape(|doc| {
            doc.stroke(|ctx| {
                ctx.color(sc).line_width(lw).circle([ox + cx, oy + cy], radius);
            });
        })
    }

    fn fill_ellipse(&self, center: Value, rx: f64, ry: f64) -> Result<(), Error> {
        let (cx, cy) = extract_point(center)?;
        let (ox, oy) = self.bounds_offset()?;
        let fc = *self.fill_color.borrow();
        self.draw_shape(|doc| {
            doc.fill(|ctx| {
                ctx.color(fc).ellipse([ox + cx, oy + cy], rx, ry);
            });
        })
    }

    fn stroke_ellipse(&self, center: Value, rx: f64, ry: f64) -> Result<(), Error> {
        let (cx, cy) = extract_point(center)?;
        let (ox, oy) = self.bounds_offset()?;
        let sc = *self.stroke_color.borrow();
        let lw = *self.line_width.borrow();
        self.draw_shape(|doc| {
            doc.stroke(|ctx| {
                ctx.color(sc).line_width(lw).ellipse([ox + cx, oy + cy], rx, ry);
            });
        })
    }

    fn fill_polygon(&self, args: &[Value]) -> Result<(), Error> {
        let (ox, oy) = self.bounds_offset()?;
        let pts = self.parse_points_with_offset(args, ox, oy)?;
        let fc = *self.fill_color.borrow();
        self.draw_shape(|doc| {
            doc.fill(|ctx| {
                ctx.color(fc).polygon(&pts);
            });
        })
    }

    fn stroke_polygon(&self, args: &[Value]) -> Result<(), Error> {
        let (ox, oy) = self.bounds_offset()?;
        let pts = self.parse_points_with_offset(args, ox, oy)?;
        let sc = *self.stroke_color.borrow();
        let lw = *self.line_width.borrow();
        self.draw_shape(|doc| {
            doc.stroke(|ctx| {
                ctx.color(sc).line_width(lw).polygon(&pts);
            });
        })
    }

    fn parse_points_with_offset(
        &self,
        args: &[Value],
        ox: f64,
        oy: f64,
    ) -> Result<Vec<[f64; 2]>, Error> {
        let mut pts = Vec::new();
        for arg in args {
            let (x, y) = extract_point(*arg)?;
            pts.push([ox + x, oy + y]);
        }
        Ok(pts)
    }

    // stroke_horizontal_line x1, x2, at: y
    fn stroke_horizontal_line(&self, args: &[Value]) -> Result<(), Error> {
        if args.len() < 2 {
            return Err(arg_err("stroke_horizontal_line requires x1, x2"));
        }
        let x1 = f64::try_convert(args[0]).map_err(|_| arg_err("x1 must be numeric"))?;
        let x2 = f64::try_convert(args[1]).map_err(|_| arg_err("x2 must be numeric"))?;

        let mut y = 0.0;
        if args.len() > 2 {
            if let Ok(hash) = RHash::try_convert(args[args.len() - 1]) {
                if let Some(at_y) = hash_get_f64(&hash, "at") {
                    y = at_y;
                }
            }
        }

        let (ox, oy) = self.bounds_offset()?;
        let sc = *self.stroke_color.borrow();
        let lw = *self.line_width.borrow();
        let dash = self.dash_pattern.borrow().clone();

        self.draw_shape(|doc| {
            doc.stroke(|ctx| {
                ctx.color(sc).line_width(lw);
                if let Some((ref pattern, phase)) = dash {
                    ctx.dash_with_phase(pattern, phase);
                }
                ctx.line([ox + x1, oy + y], [ox + x2, oy + y]);
            });
        })
    }

    fn stroke_line(&self, start: Value, end_pt: Value) -> Result<(), Error> {
        let (x1, y1) = extract_point(start)?;
        let (x2, y2) = extract_point(end_pt)?;
        let (ox, oy) = self.bounds_offset()?;
        let sc = *self.stroke_color.borrow();
        let lw = *self.line_width.borrow();
        let dash = self.dash_pattern.borrow().clone();

        self.draw_shape(|doc| {
            doc.stroke(|ctx| {
                ctx.color(sc).line_width(lw);
                if let Some((ref pattern, phase)) = dash {
                    ctx.dash_with_phase(pattern, phase);
                }
                ctx.line([ox + x1, oy + y1], [ox + x2, oy + y2]);
            });
        })
    }

    // stroke_axis(at:, color:, step_length:)
    fn stroke_axis(&self, kwargs: RHash) -> Result<(), Error> {
        let at = if let Some(at_val) = kwargs.get(Symbol::new("at")) {
            let at_val: Value = at_val;
            extract_point(at_val)?
        } else {
            (20.0, 20.0)
        };

        let color = if let Some(hex) = hash_get_string(&kwargs, "color") {
            parse_hex_color(&hex)
        } else {
            (0.6, 0.6, 0.6)
        };

        let step = hash_get_f64(&kwargs, "step_length").unwrap_or(100.0);

        self.with_doc(|doc| {
            doc.stroke_axis(
                pdfcrate::api::AxisOptions::new()
                    .at(at.0, at.1)
                    .color(color)
                    .step_length(step),
            );
        })
    }

    // canvas { block }
    fn canvas(&self) -> Result<Value, Error> {
        let ruby = unsafe { Ruby::get_unchecked() };
        let block = ruby
            .block_proc()
            .map_err(|_| rt_err("canvas requires a block"))?;

        let old_cursor = self.with_layout_ref(|layout| layout.cursor())?;

        self.with_layout(|layout| {
            layout.push_bounding_box_absolute(0.0, self.page_height, self.page_width, Some(self.page_height));
        })?;

        let result = block.call::<_, Value>(());

        self.with_layout(|layout| {
            layout.pop_bounding_box(old_cursor, None);
        })?;

        result.map_err(|e| rt_err(format!("canvas block error: {}", e)))
    }

    fn cursor(&self) -> Result<f64, Error> {
        self.with_layout_ref(|layout| layout.cursor())
    }

    fn move_cursor_to(&self, y: f64) -> Result<(), Error> {
        self.with_layout(|layout| {
            layout.move_cursor_to(y);
        })
    }

    fn move_down(&self, n: f64) -> Result<(), Error> {
        self.with_layout(|layout| {
            layout.move_down(n);
        })
    }

    fn bounds(&self) -> Result<BoundsProxy, Error> {
        let (top, width, height) = self.with_layout_ref(|layout| {
            let b = layout.bounds();
            (b.height(), b.width(), b.height())
        })?;
        Ok(BoundsProxy { top, width, height })
    }

    // bounding_box([x,y], width:, height:) { block }
    fn bounding_box(&self, args: &[Value]) -> Result<(), Error> {
        let ruby = unsafe { Ruby::get_unchecked() };

        if args.is_empty() {
            return Err(arg_err("bounding_box requires position argument"));
        }

        let (x, y) = extract_point(args[0])?;

        let mut width = 100.0f64;
        let mut height: Option<f64> = None;

        if args.len() > 1 {
            if let Ok(hash) = RHash::try_convert(args[args.len() - 1]) {
                if let Some(w) = hash_get_f64(&hash, "width") {
                    width = w;
                }
                height = hash_get_f64(&hash, "height");
            }
        }

        let block = ruby
            .block_proc()
            .map_err(|_| rt_err("bounding_box requires a block"))?;

        let old_cursor =
            self.with_layout(|layout| layout.push_bounding_box([x, y], width, height))?;

        let result = block.call::<_, Value>(());

        self.with_layout(|layout| {
            layout.pop_bounding_box(old_cursor, height);
        })?;

        result.map_err(|e| rt_err(format!("bounding_box block error: {}", e)))?;
        Ok(())
    }

    fn stroke_bounds(&self) -> Result<(), Error> {
        self.with_layout(|layout| {
            layout.stroke_bounds();
        })
    }

    // indent(n) { block }
    fn indent(&self, left: f64) -> Result<Value, Error> {
        let ruby = unsafe { Ruby::get_unchecked() };
        let block = ruby
            .block_proc()
            .map_err(|_| rt_err("indent requires a block"))?;

        self.with_layout(|layout| layout.push_indent(left, 0.0))?;
        let result = block.call::<_, Value>(());
        self.with_layout(|layout| layout.pop_indent(left, 0.0))?;

        result.map_err(|e| rt_err(format!("indent block error: {}", e)))
    }

    // float { block }
    fn float(&self) -> Result<Value, Error> {
        let ruby = unsafe { Ruby::get_unchecked() };
        let block = ruby
            .block_proc()
            .map_err(|_| rt_err("float requires a block"))?;

        let (saved_cursor, saved_page) =
            self.with_layout_ref(|layout| (layout.cursor(), layout.inner().page_number()))?;

        let result = block.call::<_, Value>(());

        self.with_layout(|layout| {
            if layout.inner().page_number() != saved_page {
                layout.inner_mut().go_to_page(saved_page - 1);
            }
            layout.set_cursor(saved_cursor);
        })?;

        result.map_err(|e| rt_err(format!("float block error: {}", e)))
    }

    // transparent(opacity) { block }
    fn transparent(&self, opacity: f64) -> Result<Value, Error> {
        let ruby = unsafe { Ruby::get_unchecked() };
        let block = ruby
            .block_proc()
            .map_err(|_| rt_err("transparent requires a block"))?;

        let old_fill = *self.fill_alpha.borrow();
        let old_stroke = *self.stroke_alpha.borrow();
        *self.fill_alpha.borrow_mut() = opacity;
        *self.stroke_alpha.borrow_mut() = opacity;

        let result = block.call::<_, Value>(());

        *self.fill_alpha.borrow_mut() = old_fill;
        *self.stroke_alpha.borrow_mut() = old_stroke;

        result.map_err(|e| rt_err(format!("transparent block error: {}", e)))
    }

    // image path, fit: [w,h], position: :center, vposition: :center
    fn image(&self, args: &[Value]) -> Result<(), Error> {
        if args.is_empty() {
            return Err(arg_err("image requires path argument"));
        }
        let path = String::try_convert(args[0]).map_err(|_| arg_err("path must be a string"))?;

        let mut fit: Option<(f64, f64)> = None;
        let mut h_center = false;
        let mut v_center = false;

        if args.len() > 1 {
            if let Ok(hash) = RHash::try_convert(args[args.len() - 1]) {
                if let Some(fit_val) = hash.get(Symbol::new("fit")) {
                    let fit_val: Value = fit_val;
                    if let Ok(pt) = extract_point(fit_val) {
                        fit = Some(pt);
                    }
                }
                if let Some(pos) = hash.get(Symbol::new("position")) {
                    if let Ok(sym) = Symbol::try_convert(pos) {
                        if sym.name().map(|n| n == "center").unwrap_or(false) {
                            h_center = true;
                        }
                    }
                }
                if let Some(vpos) = hash.get(Symbol::new("vposition")) {
                    if let Ok(sym) = Symbol::try_convert(vpos) {
                        if sym.name().map(|n| n == "center").unwrap_or(false) {
                            v_center = true;
                        }
                    }
                }
            }
        }

        let data =
            std::fs::read(&path).map_err(|e| rt_err(format!("Failed to read image: {}", e)))?;

        self.with_layout(|layout| {
            // Copy bounds values to avoid borrow conflict with inner_mut()
            let bounds_left = layout.bounds().absolute_left();
            let bounds_bottom = layout.bounds().absolute_bottom();
            let bounds_w = layout.bounds().width();
            let bounds_h = layout.bounds().height();
            let cursor_y = bounds_bottom + layout.cursor();

            // Step 1: embed image to get pixel dimensions (without drawing)
            let embedded = layout
                .inner_mut()
                .embed_image(data.as_slice())
                .map_err(|e| rt_err(format!("Failed to embed image: {}", e)));
            let embedded = match embedded {
                Ok(e) => e,
                Err(_) => return,
            };

            // Step 2: calculate fitted dimensions
            let (fitted_width, fitted_height) = if let Some((max_w, max_h)) = fit {
                embedded.fit_dimensions(max_w, max_h)
            } else {
                (embedded.width as f64, embedded.height as f64)
            };

            // Step 3: calculate position with optional centering
            let img_x = if h_center {
                bounds_left + (bounds_w - fitted_width) / 2.0
            } else {
                bounds_left
            };
            let img_y = if v_center {
                let y_offset = (bounds_h - fitted_height) / 2.0;
                bounds_bottom + y_offset
            } else {
                cursor_y - fitted_height
            };

            let opts = pdfcrate::api::ImageOptions {
                at: Some([img_x, img_y]),
                width: Some(fitted_width),
                height: Some(fitted_height),
                ..Default::default()
            };

            layout.inner_mut().draw_embedded_image(&embedded, opts);
            layout.move_down(fitted_height);
        })?;

        Ok(())
    }

    fn embed_font(&self, path: String) -> Result<String, Error> {
        self.with_doc(|doc| {
            doc.embed_font_file(&path)
                .map_err(|e| rt_err(format!("Failed to embed font: {}", e)))
        })?
    }

    fn font_families_proxy(&self) -> Result<FontFamiliesProxy, Error> {
        Ok(FontFamiliesProxy {})
    }

    // register_font_family(name, normal: path, bold: path, ...)
    fn register_font_family(&self, args: &[Value]) -> Result<(), Error> {
        if args.is_empty() {
            return Err(arg_err("register_font_family requires name"));
        }
        let name = String::try_convert(args[0]).map_err(|_| arg_err("name must be a string"))?;

        let mut styles = HashMap::new();

        if args.len() > 1 {
            if let Ok(hash) = RHash::try_convert(args[args.len() - 1]) {
                for style_name in &["normal", "bold", "italic", "light"] {
                    if let Some(path) = hash_get_string(&hash, style_name) {
                        let font_name = self.embed_font(path)?;
                        styles.insert(style_name.to_string(), font_name);
                    }
                }
            }
        }

        self.font_families.borrow_mut().register(&name, styles);
        Ok(())
    }

    // define_grid(columns:, rows:, gutter:)
    fn define_grid(&self, kwargs: RHash) -> Result<(), Error> {
        let columns = hash_get_f64(&kwargs, "columns").unwrap_or(4.0) as usize;
        let rows = hash_get_f64(&kwargs, "rows").unwrap_or(4.0) as usize;
        let gutter = hash_get_f64(&kwargs, "gutter").unwrap_or(10.0);

        self.with_layout(|layout| {
            let opts = RustGridOptions::new(rows, columns).gutter(gutter);
            layout.define_grid(opts);
        })
    }

    // grid(row, col) or grid([r1,c1], [r2,c2])
    fn grid(&self, args: &[Value]) -> Result<GridProxy, Error> {
        if args.len() == 2 {
            if let (Ok(row), Ok(col)) = (usize::try_convert(args[0]), usize::try_convert(args[1])) {
                return Ok(GridProxy::Single { row, col });
            }
            let (r1, c1) = extract_point(args[0])?;
            let (r2, c2) = extract_point(args[1])?;
            return Ok(GridProxy::Span {
                start: (r1 as usize, c1 as usize),
                end: (r2 as usize, c2 as usize),
            });
        }
        Err(arg_err(
            "grid() requires 2 arguments: (row, col) or ([r1,c1], [r2,c2])",
        ))
    }

    // Grid cell bounding box
    fn grid_cell_bb(&self, row: usize, col: usize) -> Result<Value, Error> {
        let ruby = unsafe { Ruby::get_unchecked() };
        let block = ruby
            .block_proc()
            .map_err(|_| rt_err("bounding_box requires a block"))?;

        let (old_cursor, height) = self.with_layout(|layout| {
            let cell = layout
                .grid(row, col)
                .ok_or_else(|| rt_err("grid cell not found; call define_grid first"));
            match cell {
                Ok(cell) => {
                    let bounds = layout.bounds();
                    let abs_x = bounds.absolute_left() + cell.left;
                    let abs_y = bounds.absolute_top() - (bounds.height() - cell.top);
                    let h = cell.height;
                    let oc =
                        layout.push_bounding_box_absolute(abs_x, abs_y, cell.width, Some(h));
                    Ok((oc, Some(h)))
                }
                Err(e) => Err(e),
            }
        })??;

        let result = block.call::<_, Value>(());

        self.with_layout(|layout| {
            layout.pop_bounding_box(old_cursor, height);
        })?;

        result.map_err(|e| rt_err(format!("grid bounding_box error: {}", e)))
    }

    // Grid span bounding box
    fn grid_span_bb(&self, r1: usize, c1: usize, r2: usize, c2: usize) -> Result<Value, Error> {
        let ruby = unsafe { Ruby::get_unchecked() };
        let block = ruby
            .block_proc()
            .map_err(|_| rt_err("bounding_box requires a block"))?;

        let (old_cursor, height) = self.with_layout(|layout| {
            let multi = layout
                .grid_span((r1, c1), (r2, c2))
                .ok_or_else(|| rt_err("grid span not found; call define_grid first"));
            match multi {
                Ok(multi) => {
                    let bounds = layout.bounds();
                    let abs_x = bounds.absolute_left() + multi.left;
                    let abs_y = bounds.absolute_top() - (bounds.height() - multi.top);
                    let h = multi.height;
                    let oc =
                        layout.push_bounding_box_absolute(abs_x, abs_y, multi.width, Some(h));
                    Ok((oc, Some(h)))
                }
                Err(e) => Err(e),
            }
        })??;

        let result = block.call::<_, Value>(());

        self.with_layout(|layout| {
            layout.pop_bounding_box(old_cursor, height);
        })?;

        result.map_err(|e| rt_err(format!("grid span bounding_box error: {}", e)))
    }
}

#[magnus::wrap(class = "Pdfcrate::Bounds")]
struct BoundsProxy {
    top: f64,
    width: f64,
    height: f64,
}

impl BoundsProxy {
    fn top(&self) -> f64 {
        self.top
    }
    fn width(&self) -> f64 {
        self.width
    }
    fn height(&self) -> f64 {
        self.height
    }
}

#[magnus::wrap(class = "Pdfcrate::FontFamilies")]
struct FontFamiliesProxy;

#[magnus::wrap(class = "Pdfcrate::GridProxy")]
enum GridProxy {
    Single {
        row: usize,
        col: usize,
    },
    Span {
        start: (usize, usize),
        end: (usize, usize),
    },
}

impl GridProxy {
    fn row(&self) -> usize {
        match self {
            GridProxy::Single { row, .. } => *row,
            GridProxy::Span { start, .. } => start.0,
        }
    }

    fn col(&self) -> usize {
        match self {
            GridProxy::Single { col, .. } => *col,
            GridProxy::Span { start, .. } => start.1,
        }
    }

    fn is_span(&self) -> bool {
        matches!(self, GridProxy::Span { .. })
    }

    fn end_row(&self) -> usize {
        match self {
            GridProxy::Single { row, .. } => *row,
            GridProxy::Span { end, .. } => end.0,
        }
    }

    fn end_col(&self) -> usize {
        match self {
            GridProxy::Single { col, .. } => *col,
            GridProxy::Span { end, .. } => end.1,
        }
    }
}

#[magnus::init(name = "pdfcrate")]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Pdfcrate")?;

    // Document class
    let doc_class = module.define_class("Document", ruby.class_object())?;
    doc_class.define_singleton_method("new", function!(Document::ruby_new, -1))?;
    doc_class.define_method("save_as", method!(Document::save_as, 1))?;
    doc_class.define_method("render", method!(Document::render_bytes, 0))?;
    doc_class.define_method("start_new_page", method!(Document::start_new_page, 0))?;

    // Color/line state
    doc_class.define_method("fill_color", method!(Document::set_fill_color, 1))?;
    doc_class.define_method("stroke_color", method!(Document::set_stroke_color, 1))?;
    doc_class.define_method("line_width", method!(Document::set_line_width, 1))?;
    doc_class.define_method("dash", method!(Document::set_dash, 2))?;
    doc_class.define_method("undash", method!(Document::undash, 0))?;

    // Drawing primitives
    doc_class.define_method("fill_rectangle", method!(Document::fill_rectangle, 3))?;
    doc_class.define_method("stroke_rectangle", method!(Document::stroke_rectangle, 3))?;
    doc_class.define_method(
        "fill_rounded_rectangle",
        method!(Document::fill_rounded_rectangle, 4),
    )?;
    doc_class.define_method(
        "stroke_rounded_rectangle",
        method!(Document::stroke_rounded_rectangle, 4),
    )?;
    doc_class.define_method("fill_circle", method!(Document::fill_circle, 2))?;
    doc_class.define_method("stroke_circle", method!(Document::stroke_circle, 2))?;
    doc_class.define_method("fill_ellipse", method!(Document::fill_ellipse, 3))?;
    doc_class.define_method("stroke_ellipse", method!(Document::stroke_ellipse, 3))?;
    doc_class.define_method("fill_polygon", method!(Document::fill_polygon, -1))?;
    doc_class.define_method("stroke_polygon", method!(Document::stroke_polygon, -1))?;
    doc_class.define_method(
        "stroke_horizontal_line",
        method!(Document::stroke_horizontal_line, -1),
    )?;
    doc_class.define_method("stroke_line", method!(Document::stroke_line, 2))?;
    doc_class.define_method("stroke_axis", method!(Document::stroke_axis, 1))?;

    // Font
    doc_class.define_method("font", method!(Document::set_font, -1))?;
    doc_class.define_method("width_of", method!(Document::width_of, -1))?;
    doc_class.define_method("embed_font", method!(Document::embed_font, 1))?;
    doc_class.define_method(
        "_font_families_raw",
        method!(Document::font_families_proxy, 0),
    )?;
    doc_class.define_method(
        "register_font_family",
        method!(Document::register_font_family, -1),
    )?;

    // Text
    doc_class.define_method("draw_text", method!(Document::draw_text, -1))?;
    doc_class.define_method("text", method!(Document::text_flow, -1))?;
    doc_class.define_method("text_box", method!(Document::text_box_method, -1))?;
    doc_class.define_method("formatted_text", method!(Document::formatted_text, 1))?;

    // Layout
    doc_class.define_method("canvas", method!(Document::canvas, 0))?;
    doc_class.define_method("cursor", method!(Document::cursor, 0))?;
    doc_class.define_method("move_cursor_to", method!(Document::move_cursor_to, 1))?;
    doc_class.define_method("move_down", method!(Document::move_down, 1))?;
    doc_class.define_method("bounds", method!(Document::bounds, 0))?;
    doc_class.define_method("bounding_box", method!(Document::bounding_box, -1))?;
    doc_class.define_method("stroke_bounds", method!(Document::stroke_bounds, 0))?;
    doc_class.define_method("indent", method!(Document::indent, 1))?;
    doc_class.define_method("float", method!(Document::float, 0))?;
    doc_class.define_method("transparent", method!(Document::transparent, 1))?;

    // Image
    doc_class.define_method("image", method!(Document::image, -1))?;

    // Grid
    doc_class.define_method("define_grid", method!(Document::define_grid, 1))?;
    doc_class.define_method("grid", method!(Document::grid, -1))?;
    doc_class.define_method("_grid_cell_bb", method!(Document::grid_cell_bb, 2))?;
    doc_class.define_method("_grid_span_bb", method!(Document::grid_span_bb, 4))?;

    // Bounds class
    let bounds_class = module.define_class("Bounds", ruby.class_object())?;
    bounds_class.define_method("top", method!(BoundsProxy::top, 0))?;
    bounds_class.define_method("width", method!(BoundsProxy::width, 0))?;
    bounds_class.define_method("height", method!(BoundsProxy::height, 0))?;

    // GridProxy class
    let grid_class = module.define_class("GridProxy", ruby.class_object())?;
    grid_class.define_method("row", method!(GridProxy::row, 0))?;
    grid_class.define_method("col", method!(GridProxy::col, 0))?;
    grid_class.define_method("is_span", method!(GridProxy::is_span, 0))?;
    grid_class.define_method("end_row", method!(GridProxy::end_row, 0))?;
    grid_class.define_method("end_col", method!(GridProxy::end_col, 0))?;

    Ok(())
}
