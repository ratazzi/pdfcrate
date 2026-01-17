//! SVG rendering support.
//!
//! This module provides SVG rendering with support for:
//! - Paths and basic shapes
//! - Solid color fill and stroke
//! - Linear and radial gradients
//! - Transparency (opacity)
//! - Clipping paths
//! - Text (rendered as real PDF text with embedded fonts)

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::content::{ContentBuilder, LineCap, LineJoin};
use crate::document::PdfContext;
use crate::error::{Error, Result};
use crate::objects::{PdfArray, PdfDict, PdfName, PdfObject, PdfRef};

use usvg::fontdb;
use usvg::tiny_skia_path::PathSegment;
use usvg::{FillRule, Node, Paint, Path, Transform, Tree};

/// Sanitizes a font name to be a valid PDF name
fn sanitize_font_name(name: &str) -> String {
    // PDF names can contain most ASCII characters except whitespace and delimiters
    // Replace problematic characters with underscores
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '+' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Options for SVG rendering
#[derive(Debug, Clone)]
pub struct SvgOptions {
    /// Custom font data to load (in addition to system fonts)
    pub fonts: Vec<Vec<u8>>,
    /// Whether to load system fonts (default: true)
    pub load_system_fonts: bool,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            fonts: Vec::new(),
            load_system_fonts: true, // System fonts enabled by default
        }
    }
}

impl SvgOptions {
    /// Creates new options with default settings (system fonts enabled)
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a custom font from bytes
    pub fn font(mut self, data: Vec<u8>) -> Self {
        self.fonts.push(data);
        self
    }

    /// Adds multiple custom fonts
    pub fn fonts(mut self, fonts: Vec<Vec<u8>>) -> Self {
        self.fonts.extend(fonts);
        self
    }

    /// Disables loading system fonts (only use custom fonts)
    pub fn no_system_fonts(mut self) -> Self {
        self.load_system_fonts = false;
        self
    }
}

/// Font data collected from SVG for embedding
#[derive(Debug, Clone)]
pub struct SvgFontData {
    /// PDF font resource name (e.g., "F1")
    pub pdf_name: String,
    /// Font data bytes
    pub data: Arc<Vec<u8>>,
    /// Face index in font collection
    pub face_index: u32,
    /// Units per em
    pub units_per_em: u16,
    /// Glyph set: glyph ID -> unicode text
    pub glyph_set: BTreeMap<u16, String>,
}

/// Resources collected during SVG rendering
#[derive(Debug, Default)]
pub struct SvgResources {
    /// Shading resources (for gradients)
    pub shadings: Vec<(String, PdfRef)>,
    /// ExtGState resources (for transparency)
    pub ext_gstates: Vec<(String, PdfRef)>,
    /// Font data for embedding (pdf_font_name -> font_data)
    pub fonts: Vec<SvgFontData>,
}

/// SVG renderer that can create PDF resources
pub struct SvgRenderer<'a> {
    content: &'a mut ContentBuilder,
    context: &'a mut PdfContext,
    resources: SvgResources,
    /// Cache for gradient shadings (gradient id -> shading name)
    gradient_cache: HashMap<String, String>,
    /// Cache for opacity ExtGStates (opacity key -> gs name)
    opacity_cache: HashMap<String, String>,
    /// Cache for fonts (fontdb::ID -> pdf_font_name)
    font_cache: HashMap<fontdb::ID, String>,
    /// Font data cache (fontdb::ID -> (data, face_index, units_per_em, font_key))
    font_data_cache: HashMap<fontdb::ID, (Arc<Vec<u8>>, u32, u16, String)>,
    /// Glyph usage per font (fontdb::ID -> glyph_id -> unicode text)
    font_glyphs: HashMap<fontdb::ID, BTreeMap<u16, String>>,
}

impl<'a> SvgRenderer<'a> {
    /// Creates a new SVG renderer
    pub fn new(content: &'a mut ContentBuilder, context: &'a mut PdfContext) -> Self {
        SvgRenderer {
            content,
            context,
            resources: SvgResources::default(),
            gradient_cache: HashMap::new(),
            opacity_cache: HashMap::new(),
            font_cache: HashMap::new(),
            font_data_cache: HashMap::new(),
            font_glyphs: HashMap::new(),
        }
    }

    /// Renders SVG and returns collected resources
    pub fn render(
        self,
        svg: &str,
        position: [f64; 2],
        width: f64,
        height: f64,
    ) -> Result<SvgResources> {
        self.render_with_options(svg, position, width, height, &SvgOptions::default())
    }

    /// Renders SVG with custom options (including custom fonts)
    pub fn render_with_options(
        mut self,
        svg: &str,
        position: [f64; 2],
        width: f64,
        height: f64,
        svg_options: &SvgOptions,
    ) -> Result<SvgResources> {
        if width <= 0.0 || height <= 0.0 {
            return Err(Error::Svg("SVG target size must be positive".to_string()));
        }

        let mut options = usvg::Options::default();

        // Load custom fonts first (higher priority)
        for font_data in &svg_options.fonts {
            options.fontdb_mut().load_font_data(font_data.clone());
        }

        // Optionally load system fonts
        if svg_options.load_system_fonts {
            options.fontdb_mut().load_system_fonts();
        }

        let tree = Tree::from_str(svg, &options)
            .map_err(|e| Error::Svg(format!("Failed to parse SVG: {e}")))?;

        let size = tree.size();
        let svg_width = size.width() as f64;
        let svg_height = size.height() as f64;

        if svg_width <= 0.0 || svg_height <= 0.0 {
            return Err(Error::Svg("SVG has invalid size".to_string()));
        }

        let scale_x = width / svg_width;
        let scale_y = height / svg_height;

        // Flip the Y axis to match PDF coordinates and anchor at the given position.
        let base_transform = Transform::from_row(
            scale_x as f32,
            0.0,
            0.0,
            -(scale_y as f32),
            position[0] as f32,
            (position[1] + height) as f32,
        );

        // Pre-collect font data from all text nodes (requires fonts feature)
        #[cfg(feature = "fonts")]
        self.collect_fonts_from_tree(tree.root(), tree.fontdb());

        // Render the tree
        self.render_group(tree.root(), base_transform, tree.fontdb())?;

        // Build font resources from collected data (requires fonts feature)
        #[cfg(feature = "fonts")]
        for (font_id, pdf_name) in &self.font_cache {
            if let Some((data, face_index, units_per_em, _)) = self.font_data_cache.get(font_id) {
                let glyph_set = self.font_glyphs.get(font_id).cloned().unwrap_or_default();
                self.resources.fonts.push(SvgFontData {
                    pdf_name: pdf_name.clone(),
                    data: data.clone(),
                    face_index: *face_index,
                    units_per_em: *units_per_em,
                    glyph_set,
                });
            }
        }

        Ok(self.resources)
    }

    /// Pre-collects font data from all text nodes in the tree
    #[cfg(feature = "fonts")]
    fn collect_fonts_from_tree(&mut self, group: &usvg::Group, fontdb: &fontdb::Database) {
        for node in group.children() {
            match node {
                Node::Group(g) => self.collect_fonts_from_tree(g, fontdb),
                Node::Text(text) => {
                    for span in text.layouted() {
                        for glyph in &span.positioned_glyphs {
                            let font_id = glyph.font;

                            // Cache font data if not already cached
                            if !self.font_data_cache.contains_key(&font_id) {
                                fontdb.with_face_data(font_id, |data, face_index| {
                                    if let Ok(face) = ttf_parser::Face::parse(data, face_index) {
                                        let units_per_em = face.units_per_em();
                                        // Use font's PostScript name or hash as PDF font name
                                        // to ensure uniqueness across multiple draw_svg calls
                                        let font_key = face
                                            .names()
                                            .into_iter()
                                            .find(|n| {
                                                n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME
                                            })
                                            .and_then(|n| n.to_string())
                                            .unwrap_or_else(|| {
                                                // Use hash of font data as fallback identifier
                                                format!("Font{:08X}", {
                                                    let mut h: u32 = 0;
                                                    for (i, &b) in
                                                        data.iter().take(1000).enumerate()
                                                    {
                                                        h = h.wrapping_add(
                                                            (b as u32).wrapping_mul((i + 1) as u32),
                                                        );
                                                    }
                                                    h
                                                })
                                            });
                                        self.font_data_cache.insert(
                                            font_id,
                                            (
                                                Arc::new(data.to_vec()),
                                                face_index,
                                                units_per_em,
                                                font_key,
                                            ),
                                        );
                                    }
                                });
                            }

                            // Allocate PDF font name based on font_key (not counter) for uniqueness
                            if !self.font_cache.contains_key(&font_id) {
                                // Use the font_key from font_data_cache with "SVG_" prefix
                                // to avoid conflicts with standard PDF fonts (Helvetica, Times, etc.)
                                if let Some((_, _, _, font_key)) =
                                    self.font_data_cache.get(&font_id)
                                {
                                    let pdf_name = format!("SVG_{}", sanitize_font_name(font_key));
                                    self.font_cache.insert(font_id, pdf_name);
                                }
                            }

                            // Record glyph usage
                            self.font_glyphs
                                .entry(font_id)
                                .or_default()
                                .insert(glyph.id.0, glyph.text.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Generates a unique resource name using the document-level counter
    fn next_name(&mut self, prefix: &str) -> String {
        self.context.next_svg_resource_name(prefix)
    }

    /// Renders a group node
    fn render_group(
        &mut self,
        group: &usvg::Group,
        base: Transform,
        fontdb: &fontdb::Database,
    ) -> Result<()> {
        let opacity = group.opacity().get();
        let has_opacity = opacity < 1.0;
        let has_clip = group.clip_path().is_some();

        // Apply clipping path if present
        if let Some(clip_path) = group.clip_path() {
            self.content.save_state();
            self.apply_clip_path(clip_path, base)?;
        }

        // Apply opacity if present
        if has_opacity {
            self.content.save_state();
            self.apply_opacity(opacity as f64)?;
        }

        // Render children
        for node in group.children() {
            match node {
                Node::Group(g) => self.render_group(g, base, fontdb)?,
                Node::Path(path) => self.render_path(path, base)?,
                Node::Text(text) => {
                    // Text rendering requires the 'fonts' feature for embedded font support
                    #[cfg(feature = "fonts")]
                    {
                        self.render_text(text, base)?;
                    }
                    #[cfg(not(feature = "fonts"))]
                    {
                        // Without fonts feature, SVG text cannot be rendered
                        log::warn!(
                            "SVG text element skipped: 'fonts' feature required for text rendering"
                        );
                        let _ = text; // silence unused warning
                    }
                }
                Node::Image(_) => {
                    // Image embedding not yet supported
                }
            }
        }

        // Restore states
        if has_opacity {
            self.content.restore_state();
        }
        if has_clip {
            self.content.restore_state();
        }

        Ok(())
    }

    /// Applies a clipping path
    fn apply_clip_path(&mut self, clip: &usvg::ClipPath, base: Transform) -> Result<()> {
        // The clip path's transform positions the clip in SVG coordinates
        // We need to combine: base (SVG->PDF) with clip.transform()
        let clip_transform = base.pre_concat(clip.transform());

        // Render all clip path contents as a single path
        for node in clip.root().children() {
            if let Node::Path(path) = node {
                // For paths inside clip path, combine:
                // 1. clip_transform (base + clip's transform)
                // 2. path's own transform
                let path_transform = clip_transform.pre_concat(path.abs_transform());
                draw_path_segments_transformed(
                    path.data().segments(),
                    self.content,
                    path_transform,
                );
            }
        }

        // Apply clip - W sets the clipping path, n ends without fill/stroke
        self.content.line("W n");
        Ok(())
    }

    /// Applies opacity using ExtGState
    fn apply_opacity(&mut self, opacity: f64) -> Result<()> {
        // Check cache
        let key = format!("{:.3}", opacity);
        if let Some(name) = self.opacity_cache.get(&key) {
            self.content.set_graphics_state(name);
            return Ok(());
        }

        // Create new ExtGState
        let gs_ref = self.context.alloc_ref();
        let gs_name = self.next_name("sGS"); // "s" prefix to avoid conflict with Document::transparent

        let mut gs_dict = PdfDict::new();
        gs_dict.set("Type", PdfObject::Name(PdfName::new("ExtGState")));
        gs_dict.set("CA", PdfObject::Real(opacity)); // Stroke alpha
        gs_dict.set("ca", PdfObject::Real(opacity)); // Fill alpha

        self.context.assign(gs_ref, PdfObject::Dict(gs_dict));
        self.resources.ext_gstates.push((gs_name.clone(), gs_ref));
        self.opacity_cache.insert(key, gs_name.clone());

        self.content.set_graphics_state(&gs_name);
        Ok(())
    }

    /// Renders text as real PDF text with embedded fonts
    #[cfg(feature = "fonts")]
    fn render_text(&mut self, text: &usvg::Text, base: Transform) -> Result<()> {
        for span in text.layouted() {
            if !span.visible {
                continue;
            }

            // Get fill color (default to black if none)
            let (r, g, b) = if let Some(fill) = &span.fill {
                match fill.paint() {
                    Paint::Color(color) => (
                        color.red as f64 / 255.0,
                        color.green as f64 / 255.0,
                        color.blue as f64 / 255.0,
                    ),
                    _ => (0.0, 0.0, 0.0),
                }
            } else {
                (0.0, 0.0, 0.0)
            };

            // Handle fill opacity
            let opacity = span.fill.as_ref().map(|f| f.opacity().get()).unwrap_or(1.0);
            let has_opacity = opacity < 1.0;

            if has_opacity {
                self.content.save_state();
                self.apply_opacity(opacity as f64)?;
            }

            // Render each glyph with its transform
            for glyph in &span.positioned_glyphs {
                // Get PDF font name for this glyph's font
                let pdf_font_name = match self.font_cache.get(&glyph.font) {
                    Some(name) => name.clone(),
                    None => continue, // Font not found, skip
                };

                // Get units per em for scaling
                let units_per_em = self
                    .font_data_cache
                    .get(&glyph.font)
                    .map(|(_, _, u, _)| *u)
                    .unwrap_or(1000) as f32;

                // Get glyph transform and scale
                // The glyph transform includes the font size scaling
                // We need to rescale to use font size 1.0 in PDF and let the matrix handle it
                let glyph_transform = glyph
                    .outline_transform()
                    .pre_scale(units_per_em, units_per_em)
                    .pre_scale(1.0 / span.font_size.get(), 1.0 / span.font_size.get());

                let combined = base.pre_concat(glyph_transform);

                self.content.save_state();
                self.content.set_fill_color_rgb(r, g, b);

                // Begin text
                self.content.begin_text();

                // Set text matrix (combines position and scale)
                self.content.set_text_matrix(
                    combined.sx as f64,
                    combined.ky as f64,
                    combined.kx as f64,
                    combined.sy as f64,
                    combined.tx as f64,
                    combined.ty as f64,
                );

                // Set font and size
                self.content
                    .set_font(&pdf_font_name, span.font_size.get() as f64);

                // Show glyph as hex-encoded CID
                // For CID fonts, we use glyph ID directly as the CID
                let hex = format!("{:04X}", glyph.id.0);
                self.content.show_text_hex(&hex);

                self.content.end_text();
                self.content.restore_state();
            }

            if has_opacity {
                self.content.restore_state();
            }
        }

        Ok(())
    }

    /// Renders a path node
    fn render_path(&mut self, path: &Path, base: Transform) -> Result<()> {
        if !path.is_visible() {
            return Ok(());
        }

        let transform = base.pre_concat(path.abs_transform());

        match path.paint_order() {
            usvg::PaintOrder::FillAndStroke => {
                self.render_fill(path, transform)?;
                self.render_stroke(path, transform)?;
            }
            usvg::PaintOrder::StrokeAndFill => {
                self.render_stroke(path, transform)?;
                self.render_fill(path, transform)?;
            }
        }

        Ok(())
    }

    /// Renders fill
    fn render_fill(&mut self, path: &Path, transform: Transform) -> Result<()> {
        let fill = match path.fill() {
            Some(fill) => fill,
            None => return Ok(()),
        };

        // Handle fill opacity
        let opacity = fill.opacity().get();
        let has_opacity = opacity < 1.0;

        if has_opacity {
            self.content.save_state();
            self.apply_opacity(opacity as f64)?;
        }

        self.content.save_state();
        apply_transform(self.content, transform);

        match fill.paint() {
            Paint::Color(color) => {
                self.content.set_fill_color_rgb(
                    color.red as f64 / 255.0,
                    color.green as f64 / 255.0,
                    color.blue as f64 / 255.0,
                );
                draw_path_segments(path.data().segments(), self.content);
                match fill.rule() {
                    FillRule::EvenOdd => self.content.fill_even_odd(),
                    FillRule::NonZero => self.content.fill(),
                };
            }
            Paint::LinearGradient(grad) => {
                self.fill_with_linear_gradient(path, grad, fill.rule())?;
            }
            Paint::RadialGradient(grad) => {
                self.fill_with_radial_gradient(path, grad, fill.rule())?;
            }
            Paint::Pattern(_) => {
                // Pattern not supported, skip silently
            }
        }

        self.content.restore_state();

        if has_opacity {
            self.content.restore_state();
        }

        Ok(())
    }

    /// Renders stroke
    fn render_stroke(&mut self, path: &Path, transform: Transform) -> Result<()> {
        let stroke = match path.stroke() {
            Some(stroke) => stroke,
            None => return Ok(()),
        };

        // Handle stroke opacity
        let opacity = stroke.opacity().get();
        let has_opacity = opacity < 1.0;

        if has_opacity {
            self.content.save_state();
            self.apply_opacity(opacity as f64)?;
        }

        self.content.save_state();
        apply_transform(self.content, transform);

        self.content.set_line_width(stroke.width().get() as f64);
        self.content.set_line_cap(map_line_cap(stroke.linecap()));
        self.content.set_line_join(map_line_join(stroke.linejoin()));

        if let Some(pattern) = stroke.dasharray() {
            let pattern: Vec<f64> = pattern.iter().map(|value| *value as f64).collect();
            self.content.set_dash(&pattern, stroke.dashoffset() as f64);
        }

        match stroke.paint() {
            Paint::Color(color) => {
                self.content.set_stroke_color_rgb(
                    color.red as f64 / 255.0,
                    color.green as f64 / 255.0,
                    color.blue as f64 / 255.0,
                );
                draw_path_segments(path.data().segments(), self.content);
                self.content.stroke();
            }
            Paint::LinearGradient(_) | Paint::RadialGradient(_) => {
                // Gradient strokes are complex - fall back to first stop color
                if let Paint::LinearGradient(grad) = stroke.paint() {
                    if let Some(stop) = grad.stops().first() {
                        self.content.set_stroke_color_rgb(
                            stop.color().red as f64 / 255.0,
                            stop.color().green as f64 / 255.0,
                            stop.color().blue as f64 / 255.0,
                        );
                    }
                } else if let Paint::RadialGradient(grad) = stroke.paint() {
                    if let Some(stop) = grad.stops().first() {
                        self.content.set_stroke_color_rgb(
                            stop.color().red as f64 / 255.0,
                            stop.color().green as f64 / 255.0,
                            stop.color().blue as f64 / 255.0,
                        );
                    }
                }
                draw_path_segments(path.data().segments(), self.content);
                self.content.stroke();
            }
            Paint::Pattern(_) => {
                // Pattern not supported, skip silently
            }
        }

        self.content.restore_state();

        if has_opacity {
            self.content.restore_state();
        }

        Ok(())
    }

    /// Fills with linear gradient using PDF Shading
    fn fill_with_linear_gradient(
        &mut self,
        path: &Path,
        grad: &usvg::LinearGradient,
        fill_rule: FillRule,
    ) -> Result<()> {
        // Create shading pattern
        let shading_name = self.create_linear_gradient_shading(grad)?;

        // Define clipping path first
        draw_path_segments(path.data().segments(), self.content);
        match fill_rule {
            FillRule::EvenOdd => self.content.line("W* n"),
            FillRule::NonZero => self.content.line("W n"),
        };

        // Apply gradient transform and fill with shading
        let grad_transform = grad.transform();
        self.content.save_state();
        self.content.concat_matrix(
            grad_transform.sx as f64,
            grad_transform.ky as f64,
            grad_transform.kx as f64,
            grad_transform.sy as f64,
            grad_transform.tx as f64,
            grad_transform.ty as f64,
        );
        self.content.line(&format!("/{} sh", shading_name));
        self.content.restore_state();

        Ok(())
    }

    /// Creates a linear gradient shading object
    fn create_linear_gradient_shading(&mut self, grad: &usvg::LinearGradient) -> Result<String> {
        // Check cache
        let grad_id = grad.id().to_string();
        if let Some(name) = self.gradient_cache.get(&grad_id) {
            return Ok(name.clone());
        }

        let shading_ref = self.context.alloc_ref();
        let shading_name = self.next_name("sSh"); // "s" prefix for SVG resources

        // Create Function for color interpolation
        let function = self.create_gradient_function(grad.stops())?;
        let func_ref = self.context.register(PdfObject::Dict(function));

        // Create Shading dictionary (Type 2 - Axial)
        let mut shading = PdfDict::new();
        shading.set("ShadingType", PdfObject::Integer(2)); // Axial
        shading.set("ColorSpace", PdfObject::Name(PdfName::new("DeviceRGB")));
        shading.set(
            "Coords",
            PdfObject::Array(PdfArray::from(vec![
                PdfObject::Real(grad.x1() as f64),
                PdfObject::Real(grad.y1() as f64),
                PdfObject::Real(grad.x2() as f64),
                PdfObject::Real(grad.y2() as f64),
            ])),
        );
        shading.set("Function", PdfObject::Reference(func_ref));

        // Handle spread method
        match grad.spread_method() {
            usvg::SpreadMethod::Pad => {
                shading.set(
                    "Extend",
                    PdfObject::Array(PdfArray::from(vec![
                        PdfObject::Bool(true),
                        PdfObject::Bool(true),
                    ])),
                );
            }
            _ => {
                // Repeat and Reflect are complex, fall back to Pad
                shading.set(
                    "Extend",
                    PdfObject::Array(PdfArray::from(vec![
                        PdfObject::Bool(true),
                        PdfObject::Bool(true),
                    ])),
                );
            }
        }

        self.context.assign(shading_ref, PdfObject::Dict(shading));
        self.resources
            .shadings
            .push((shading_name.clone(), shading_ref));
        self.gradient_cache.insert(grad_id, shading_name.clone());

        Ok(shading_name)
    }

    /// Fills with radial gradient using PDF Shading
    fn fill_with_radial_gradient(
        &mut self,
        path: &Path,
        grad: &usvg::RadialGradient,
        fill_rule: FillRule,
    ) -> Result<()> {
        // Create shading pattern
        let shading_name = self.create_radial_gradient_shading(grad)?;

        // Define clipping path first
        draw_path_segments(path.data().segments(), self.content);
        match fill_rule {
            FillRule::EvenOdd => self.content.line("W* n"),
            FillRule::NonZero => self.content.line("W n"),
        };

        // Apply gradient transform and fill with shading
        let grad_transform = grad.transform();
        self.content.save_state();
        self.content.concat_matrix(
            grad_transform.sx as f64,
            grad_transform.ky as f64,
            grad_transform.kx as f64,
            grad_transform.sy as f64,
            grad_transform.tx as f64,
            grad_transform.ty as f64,
        );
        self.content.line(&format!("/{} sh", shading_name));
        self.content.restore_state();

        Ok(())
    }

    /// Creates a radial gradient shading object
    fn create_radial_gradient_shading(&mut self, grad: &usvg::RadialGradient) -> Result<String> {
        // Check cache
        let grad_id = grad.id().to_string();
        if let Some(name) = self.gradient_cache.get(&grad_id) {
            return Ok(name.clone());
        }

        let shading_ref = self.context.alloc_ref();
        let shading_name = self.next_name("sSh"); // "s" prefix for SVG resources

        // Create Function for color interpolation
        let function = self.create_gradient_function(grad.stops())?;
        let func_ref = self.context.register(PdfObject::Dict(function));

        // Create Shading dictionary (Type 3 - Radial)
        let mut shading = PdfDict::new();
        shading.set("ShadingType", PdfObject::Integer(3)); // Radial
        shading.set("ColorSpace", PdfObject::Name(PdfName::new("DeviceRGB")));
        shading.set(
            "Coords",
            PdfObject::Array(PdfArray::from(vec![
                PdfObject::Real(grad.fx() as f64), // Start circle center
                PdfObject::Real(grad.fy() as f64),
                PdfObject::Real(0.0), // Start circle radius (usually 0)
                PdfObject::Real(grad.cx() as f64), // End circle center
                PdfObject::Real(grad.cy() as f64),
                PdfObject::Real(grad.r().get() as f64), // End circle radius
            ])),
        );
        shading.set("Function", PdfObject::Reference(func_ref));
        shading.set(
            "Extend",
            PdfObject::Array(PdfArray::from(vec![
                PdfObject::Bool(true),
                PdfObject::Bool(true),
            ])),
        );

        self.context.assign(shading_ref, PdfObject::Dict(shading));
        self.resources
            .shadings
            .push((shading_name.clone(), shading_ref));
        self.gradient_cache.insert(grad_id, shading_name.clone());

        Ok(shading_name)
    }

    /// Creates a PDF Function for gradient color interpolation
    fn create_gradient_function(&self, stops: &[usvg::Stop]) -> Result<PdfDict> {
        if stops.len() < 2 {
            return Err(Error::Svg(
                "Gradient must have at least 2 stops".to_string(),
            ));
        }

        if stops.len() == 2 {
            // Simple case: Type 2 exponential interpolation
            let mut func = PdfDict::new();
            func.set("FunctionType", PdfObject::Integer(2));
            func.set(
                "Domain",
                PdfObject::Array(PdfArray::from(vec![
                    PdfObject::Real(0.0),
                    PdfObject::Real(1.0),
                ])),
            );
            func.set("N", PdfObject::Real(1.0)); // Linear interpolation

            let c0 = &stops[0];
            let c1 = &stops[1];

            func.set(
                "C0",
                PdfObject::Array(PdfArray::from(vec![
                    PdfObject::Real(c0.color().red as f64 / 255.0),
                    PdfObject::Real(c0.color().green as f64 / 255.0),
                    PdfObject::Real(c0.color().blue as f64 / 255.0),
                ])),
            );
            func.set(
                "C1",
                PdfObject::Array(PdfArray::from(vec![
                    PdfObject::Real(c1.color().red as f64 / 255.0),
                    PdfObject::Real(c1.color().green as f64 / 255.0),
                    PdfObject::Real(c1.color().blue as f64 / 255.0),
                ])),
            );

            Ok(func)
        } else {
            // Multiple stops: Type 3 stitching function
            let mut func = PdfDict::new();
            func.set("FunctionType", PdfObject::Integer(3));
            func.set(
                "Domain",
                PdfObject::Array(PdfArray::from(vec![
                    PdfObject::Real(0.0),
                    PdfObject::Real(1.0),
                ])),
            );

            // Create subfunctions for each segment
            let mut functions = PdfArray::new();
            let mut bounds = PdfArray::new();
            let mut encode = PdfArray::new();

            for i in 0..stops.len() - 1 {
                let c0 = &stops[i];
                let c1 = &stops[i + 1];

                let mut subfunc = PdfDict::new();
                subfunc.set("FunctionType", PdfObject::Integer(2));
                subfunc.set(
                    "Domain",
                    PdfObject::Array(PdfArray::from(vec![
                        PdfObject::Real(0.0),
                        PdfObject::Real(1.0),
                    ])),
                );
                subfunc.set("N", PdfObject::Real(1.0));
                subfunc.set(
                    "C0",
                    PdfObject::Array(PdfArray::from(vec![
                        PdfObject::Real(c0.color().red as f64 / 255.0),
                        PdfObject::Real(c0.color().green as f64 / 255.0),
                        PdfObject::Real(c0.color().blue as f64 / 255.0),
                    ])),
                );
                subfunc.set(
                    "C1",
                    PdfObject::Array(PdfArray::from(vec![
                        PdfObject::Real(c1.color().red as f64 / 255.0),
                        PdfObject::Real(c1.color().green as f64 / 255.0),
                        PdfObject::Real(c1.color().blue as f64 / 255.0),
                    ])),
                );

                functions.push(PdfObject::Dict(subfunc));

                if i < stops.len() - 2 {
                    bounds.push(PdfObject::Real(c1.offset().get() as f64));
                }

                encode.push(PdfObject::Real(0.0));
                encode.push(PdfObject::Real(1.0));
            }

            func.set("Functions", PdfObject::Array(functions));
            func.set("Bounds", PdfObject::Array(bounds));
            func.set("Encode", PdfObject::Array(encode));

            Ok(func)
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Renders SVG into PDF content stream with full feature support.
///
/// This function supports:
/// - Paths and shapes
/// - Solid colors
/// - Linear and radial gradients
/// - Transparency (opacity)
/// - Clipping paths
/// - Text (rendered as real PDF text with embedded fonts)
///
/// Returns resources that need to be added to the page.
pub fn render_svg(
    content: &mut ContentBuilder,
    context: &mut PdfContext,
    svg: &str,
    position: [f64; 2],
    width: f64,
    height: f64,
) -> Result<SvgResources> {
    let renderer = SvgRenderer::new(content, context);
    renderer.render(svg, position, width, height)
}

/// Renders SVG into PDF content stream with custom options.
///
/// This allows specifying custom fonts and controlling system font loading.
///
/// # Example
///
/// ```rust,ignore
/// use pdfcrate::svg::{render_svg_with_options, SvgOptions};
///
/// // Load custom font
/// let font_data = std::fs::read("custom_font.ttf")?;
/// let options = SvgOptions::new()
///     .font(font_data)
///     .no_system_fonts();  // Only use custom fonts
///
/// let resources = render_svg_with_options(
///     &mut content, &mut context, svg_str,
///     [72.0, 500.0], 200.0, 100.0, &options
/// )?;
/// ```
pub fn render_svg_with_options(
    content: &mut ContentBuilder,
    context: &mut PdfContext,
    svg: &str,
    position: [f64; 2],
    width: f64,
    height: f64,
    options: &SvgOptions,
) -> Result<SvgResources> {
    let renderer = SvgRenderer::new(content, context);
    renderer.render_with_options(svg, position, width, height, options)
}

/// Renders SVG paths into a PDF content stream (legacy API).
///
/// This is kept for backward compatibility. For full feature support,
/// use [`render_svg`] instead.
pub fn render_svg_paths(
    content: &mut ContentBuilder,
    svg: &str,
    position: [f64; 2],
    width: f64,
    height: f64,
) -> Result<()> {
    if width <= 0.0 || height <= 0.0 {
        return Err(Error::Svg("SVG target size must be positive".to_string()));
    }

    let options = usvg::Options::default();
    let tree = Tree::from_str(svg, &options)
        .map_err(|e| Error::Svg(format!("Failed to parse SVG: {e}")))?;

    let size = tree.size();
    let svg_width = size.width() as f64;
    let svg_height = size.height() as f64;

    if svg_width <= 0.0 || svg_height <= 0.0 {
        return Err(Error::Svg("SVG has invalid size".to_string()));
    }

    let scale_x = width / svg_width;
    let scale_y = height / svg_height;

    let base_transform = Transform::from_row(
        scale_x as f32,
        0.0,
        0.0,
        -(scale_y as f32),
        position[0] as f32,
        (position[1] + height) as f32,
    );

    render_group_legacy(tree.root(), content, base_transform)
}

// ============================================================================
// Helper functions
// ============================================================================

fn apply_transform(content: &mut ContentBuilder, transform: Transform) {
    content.concat_matrix(
        transform.sx as f64,
        transform.ky as f64,
        transform.kx as f64,
        transform.sy as f64,
        transform.tx as f64,
        transform.ty as f64,
    );
}

fn draw_path_segments(segments: impl Iterator<Item = PathSegment>, content: &mut ContentBuilder) {
    fn calc(n1: f32, n2: f32) -> f32 {
        (n1 + n2 * 2.0) / 3.0
    }

    let mut prev = None;

    for segment in segments {
        match segment {
            PathSegment::MoveTo(p) => {
                content.move_to(p.x as f64, p.y as f64);
                prev = Some(p);
            }
            PathSegment::LineTo(p) => {
                content.line_to(p.x as f64, p.y as f64);
                prev = Some(p);
            }
            PathSegment::QuadTo(p1, p2) => {
                if let Some(prev) = prev {
                    content.curve_to(
                        calc(prev.x, p1.x) as f64,
                        calc(prev.y, p1.y) as f64,
                        calc(p2.x, p1.x) as f64,
                        calc(p2.y, p1.y) as f64,
                        p2.x as f64,
                        p2.y as f64,
                    );
                }
                prev = Some(p2);
            }
            PathSegment::CubicTo(p1, p2, p3) => {
                content.curve_to(
                    p1.x as f64,
                    p1.y as f64,
                    p2.x as f64,
                    p2.y as f64,
                    p3.x as f64,
                    p3.y as f64,
                );
                prev = Some(p3);
            }
            PathSegment::Close => {
                content.close_path();
            }
        }
    }
}

/// Draws path segments with coordinates transformed by the given transform
fn draw_path_segments_transformed(
    segments: impl Iterator<Item = PathSegment>,
    content: &mut ContentBuilder,
    transform: Transform,
) {
    fn calc(n1: f32, n2: f32) -> f32 {
        (n1 + n2 * 2.0) / 3.0
    }

    // Helper to transform a point
    let tx = |x: f32, y: f32| -> (f64, f64) {
        let new_x = transform.sx * x + transform.kx * y + transform.tx;
        let new_y = transform.ky * x + transform.sy * y + transform.ty;
        (new_x as f64, new_y as f64)
    };

    let mut prev = None;

    for segment in segments {
        match segment {
            PathSegment::MoveTo(p) => {
                let (x, y) = tx(p.x, p.y);
                content.move_to(x, y);
                prev = Some(p);
            }
            PathSegment::LineTo(p) => {
                let (x, y) = tx(p.x, p.y);
                content.line_to(x, y);
                prev = Some(p);
            }
            PathSegment::QuadTo(p1, p2) => {
                if let Some(prev_pt) = prev {
                    let (x1, y1) = tx(calc(prev_pt.x, p1.x), calc(prev_pt.y, p1.y));
                    let (x2, y2) = tx(calc(p2.x, p1.x), calc(p2.y, p1.y));
                    let (x3, y3) = tx(p2.x, p2.y);
                    content.curve_to(x1, y1, x2, y2, x3, y3);
                }
                prev = Some(p2);
            }
            PathSegment::CubicTo(p1, p2, p3) => {
                let (x1, y1) = tx(p1.x, p1.y);
                let (x2, y2) = tx(p2.x, p2.y);
                let (x3, y3) = tx(p3.x, p3.y);
                content.curve_to(x1, y1, x2, y2, x3, y3);
                prev = Some(p3);
            }
            PathSegment::Close => {
                content.close_path();
            }
        }
    }
}

fn map_line_cap(cap: usvg::LineCap) -> LineCap {
    match cap {
        usvg::LineCap::Butt => LineCap::Butt,
        usvg::LineCap::Round => LineCap::Round,
        usvg::LineCap::Square => LineCap::Square,
    }
}

fn map_line_join(join: usvg::LineJoin) -> LineJoin {
    match join {
        usvg::LineJoin::Round => LineJoin::Round,
        usvg::LineJoin::Bevel => LineJoin::Bevel,
        usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => LineJoin::Miter,
    }
}

// ============================================================================
// Legacy implementation (for backward compatibility)
// ============================================================================

fn render_group_legacy(
    group: &usvg::Group,
    content: &mut ContentBuilder,
    base: Transform,
) -> Result<()> {
    for node in group.children() {
        match node {
            Node::Group(group) => render_group_legacy(group, content, base)?,
            Node::Path(path) => render_path_legacy(path, content, base)?,
            Node::Text(text) => {
                // Text is pre-flattened to paths by usvg
                render_group_legacy(text.flattened(), content, base)?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn render_path_legacy(path: &Path, content: &mut ContentBuilder, base: Transform) -> Result<()> {
    if !path.is_visible() {
        return Ok(());
    }

    let transform = base.pre_concat(path.abs_transform());

    match path.paint_order() {
        usvg::PaintOrder::FillAndStroke => {
            render_fill_legacy(path, content, transform)?;
            render_stroke_legacy(path, content, transform)?;
        }
        usvg::PaintOrder::StrokeAndFill => {
            render_stroke_legacy(path, content, transform)?;
            render_fill_legacy(path, content, transform)?;
        }
    }

    Ok(())
}

fn render_fill_legacy(
    path: &Path,
    content: &mut ContentBuilder,
    transform: Transform,
) -> Result<()> {
    let fill = match path.fill() {
        Some(fill) => fill,
        None => return Ok(()),
    };

    let color = match fill.paint() {
        Paint::Color(color) => color,
        _ => return Ok(()), // Skip unsupported paint types silently in legacy mode
    };

    content.save_state();
    apply_transform(content, transform);
    content.set_fill_color_rgb(
        color.red as f64 / 255.0,
        color.green as f64 / 255.0,
        color.blue as f64 / 255.0,
    );

    draw_path_segments(path.data().segments(), content);

    match fill.rule() {
        FillRule::EvenOdd => content.fill_even_odd(),
        FillRule::NonZero => content.fill(),
    };

    content.restore_state();
    Ok(())
}

fn render_stroke_legacy(
    path: &Path,
    content: &mut ContentBuilder,
    transform: Transform,
) -> Result<()> {
    let stroke = match path.stroke() {
        Some(stroke) => stroke,
        None => return Ok(()),
    };

    let color = match stroke.paint() {
        Paint::Color(color) => color,
        _ => return Ok(()), // Skip unsupported paint types silently in legacy mode
    };

    content.save_state();
    apply_transform(content, transform);
    content.set_stroke_color_rgb(
        color.red as f64 / 255.0,
        color.green as f64 / 255.0,
        color.blue as f64 / 255.0,
    );
    content.set_line_width(stroke.width().get() as f64);
    content.set_line_cap(map_line_cap(stroke.linecap()));
    content.set_line_join(map_line_join(stroke.linejoin()));

    if let Some(pattern) = stroke.dasharray() {
        let pattern: Vec<f64> = pattern.iter().map(|value| *value as f64).collect();
        content.set_dash(&pattern, stroke.dashoffset() as f64);
    }

    draw_path_segments(path.data().segments(), content);
    content.stroke();
    content.restore_state();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ContentBuilder;
    use crate::document::PdfContext;

    /// Helper to render SVG and get resources
    fn render_test_svg(svg: &str) -> Result<SvgResources> {
        let mut content = ContentBuilder::new();
        let mut context = PdfContext::new();
        let renderer = SvgRenderer::new(&mut content, &mut context);
        renderer.render(svg, [0.0, 0.0], 100.0, 100.0)
    }

    #[test]
    fn test_basic_rect() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect x="10" y="10" width="80" height="80" fill="red"/>
        </svg>"#;
        let result = render_test_svg(svg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_linear_gradient() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <defs>
                <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" style="stop-color:#ff0000"/>
                    <stop offset="100%" style="stop-color:#0000ff"/>
                </linearGradient>
            </defs>
            <rect x="10" y="10" width="80" height="80" fill="url(#grad1)"/>
        </svg>"##;
        let result = render_test_svg(svg).unwrap();
        // Should have created a shading resource
        assert!(
            !result.shadings.is_empty(),
            "Expected shading resource for gradient"
        );
        // Shading name should have "sSh" prefix
        assert!(
            result.shadings[0].0.starts_with("sSh"),
            "Expected sSh prefix"
        );
    }

    #[test]
    fn test_radial_gradient() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <defs>
                <radialGradient id="grad1" cx="50%" cy="50%" r="50%">
                    <stop offset="0%" style="stop-color:#ff0000"/>
                    <stop offset="100%" style="stop-color:#0000ff"/>
                </radialGradient>
            </defs>
            <circle cx="50" cy="50" r="40" fill="url(#grad1)"/>
        </svg>"##;
        let result = render_test_svg(svg).unwrap();
        assert!(
            !result.shadings.is_empty(),
            "Expected shading resource for radial gradient"
        );
    }

    #[test]
    fn test_opacity() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect x="10" y="10" width="80" height="80" fill="red" opacity="0.5"/>
        </svg>"#;
        let result = render_test_svg(svg).unwrap();
        // Should have created an ExtGState resource for opacity
        assert!(
            !result.ext_gstates.is_empty(),
            "Expected ExtGState for opacity"
        );
        // ExtGState name should have "sGS" prefix
        assert!(
            result.ext_gstates[0].0.starts_with("sGS"),
            "Expected sGS prefix"
        );
    }

    #[test]
    fn test_group_opacity() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <g opacity="0.7">
                <rect x="10" y="10" width="40" height="40" fill="red"/>
                <rect x="50" y="50" width="40" height="40" fill="blue"/>
            </g>
        </svg>"#;
        let result = render_test_svg(svg).unwrap();
        assert!(
            !result.ext_gstates.is_empty(),
            "Expected ExtGState for group opacity"
        );
    }

    #[test]
    fn test_clip_path() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <defs>
                <clipPath id="clip1">
                    <circle cx="50" cy="50" r="40"/>
                </clipPath>
            </defs>
            <rect x="0" y="0" width="100" height="100" fill="red" clip-path="url(#clip1)"/>
        </svg>"##;
        let result = render_test_svg(svg);
        assert!(result.is_ok(), "Clip path should render without error");
    }

    #[test]
    fn test_resource_name_uniqueness() {
        // Multiple draw_svg calls should generate unique resource names
        let mut content = ContentBuilder::new();
        let mut context = PdfContext::new();

        let svg_with_opacity = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect x="10" y="10" width="80" height="80" fill="red" opacity="0.5"/>
        </svg>"#;

        // First render
        let renderer1 = SvgRenderer::new(&mut content, &mut context);
        let result1 = renderer1
            .render(svg_with_opacity, [0.0, 0.0], 100.0, 100.0)
            .unwrap();

        // Second render (same SVG, should get different resource names)
        let renderer2 = SvgRenderer::new(&mut content, &mut context);
        let result2 = renderer2
            .render(svg_with_opacity, [100.0, 0.0], 100.0, 100.0)
            .unwrap();

        // Resource names should be different
        if !result1.ext_gstates.is_empty() && !result2.ext_gstates.is_empty() {
            assert_ne!(
                result1.ext_gstates[0].0, result2.ext_gstates[0].0,
                "Resource names should be unique across draw_svg calls"
            );
        }
    }

    #[test]
    fn test_svg_options_default() {
        let options = SvgOptions::default();
        assert!(
            options.load_system_fonts,
            "System fonts should be enabled by default"
        );
        assert!(options.fonts.is_empty(), "No custom fonts by default");
    }

    #[test]
    fn test_svg_options_no_system_fonts() {
        let options = SvgOptions::new().no_system_fonts();
        assert!(
            !options.load_system_fonts,
            "System fonts should be disabled"
        );
    }

    #[test]
    fn test_svg_options_custom_font() {
        let fake_font_data = vec![0u8; 100]; // Fake font data
        let options = SvgOptions::new().font(fake_font_data.clone());
        assert_eq!(options.fonts.len(), 1, "Should have one custom font");
        assert_eq!(options.fonts[0], fake_font_data);
    }

    #[test]
    fn test_sanitize_font_name() {
        assert_eq!(sanitize_font_name("Helvetica"), "Helvetica");
        assert_eq!(sanitize_font_name("Arial Bold"), "Arial_Bold");
        assert_eq!(sanitize_font_name("Font/Name"), "Font_Name");
        assert_eq!(sanitize_font_name("Test-Font_123"), "Test-Font_123");
    }

    #[test]
    fn test_stroke_styles() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <line x1="10" y1="10" x2="90" y2="90" stroke="black" stroke-width="2" stroke-dasharray="5,3"/>
        </svg>"#;
        let result = render_test_svg(svg);
        assert!(result.is_ok(), "Stroke with dash array should render");
    }

    #[test]
    fn test_nested_groups() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <g>
                <g>
                    <rect x="10" y="10" width="80" height="80" fill="red"/>
                </g>
            </g>
        </svg>"#;
        let result = render_test_svg(svg);
        assert!(result.is_ok(), "Nested groups should render");
    }

    #[test]
    fn test_transforms() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect x="0" y="0" width="50" height="50" fill="red" transform="translate(25, 25) rotate(45)"/>
        </svg>"#;
        let result = render_test_svg(svg);
        assert!(result.is_ok(), "Transforms should render");
    }

    #[test]
    fn test_invalid_svg() {
        let svg = "not valid svg";
        let result = render_test_svg(svg);
        assert!(result.is_err(), "Invalid SVG should return error");
    }

    #[test]
    fn test_empty_svg() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"></svg>"#;
        let result = render_test_svg(svg);
        assert!(result.is_ok(), "Empty SVG should render without error");
    }

    #[test]
    fn test_zero_size_rejected() {
        let mut content = ContentBuilder::new();
        let mut context = PdfContext::new();
        let renderer = SvgRenderer::new(&mut content, &mut context);
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"></svg>"#;

        let result = renderer.render(svg, [0.0, 0.0], 0.0, 100.0);
        assert!(result.is_err(), "Zero width should be rejected");
    }

    #[test]
    fn test_multiple_gradients_same_svg() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
            <defs>
                <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" style="stop-color:#ff0000"/>
                    <stop offset="100%" style="stop-color:#00ff00"/>
                </linearGradient>
                <linearGradient id="grad2" x1="0%" y1="0%" x2="0%" y2="100%">
                    <stop offset="0%" style="stop-color:#0000ff"/>
                    <stop offset="100%" style="stop-color:#ffff00"/>
                </linearGradient>
            </defs>
            <rect x="0" y="0" width="100" height="100" fill="url(#grad1)"/>
            <rect x="100" y="0" width="100" height="100" fill="url(#grad2)"/>
        </svg>"##;
        let result = render_test_svg(svg).unwrap();
        assert_eq!(
            result.shadings.len(),
            2,
            "Should have two shading resources"
        );
    }
}
