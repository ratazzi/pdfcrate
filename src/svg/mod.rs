//! SVG path rendering (path-only support).

use crate::content::{ContentBuilder, LineCap, LineJoin};
use crate::error::{Error, Result};

use usvg::tiny_skia_path::PathSegment;
use usvg::{FillRule, Node, Paint, Path, Transform, Tree};

/// Renders SVG paths into a PDF content stream.
///
/// This currently supports path-only SVGs (including basic shapes converted to paths).
/// Unsupported paint types (gradients/patterns) will return an error.
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

    // Flip the Y axis to match PDF coordinates and anchor at the given position.
    let base_transform = Transform::from_row(
        scale_x as f32,
        0.0,
        0.0,
        -(scale_y as f32),
        position[0] as f32,
        (position[1] + height) as f32,
    );

    render_group(tree.root(), content, base_transform)
}

fn render_group(group: &usvg::Group, content: &mut ContentBuilder, base: Transform) -> Result<()> {
    for node in group.children() {
        match node {
            Node::Group(group) => render_group(group, content, base)?,
            Node::Path(path) => render_path(path, content, base)?,
            _ => {}
        }
    }

    Ok(())
}

fn render_path(path: &Path, content: &mut ContentBuilder, base: Transform) -> Result<()> {
    if !path.is_visible() {
        return Ok(());
    }

    let transform = base.pre_concat(path.abs_transform());

    match path.paint_order() {
        usvg::PaintOrder::FillAndStroke => {
            render_fill(path, content, transform)?;
            render_stroke(path, content, transform)?;
        }
        usvg::PaintOrder::StrokeAndFill => {
            render_stroke(path, content, transform)?;
            render_fill(path, content, transform)?;
        }
    }

    Ok(())
}

fn render_fill(path: &Path, content: &mut ContentBuilder, transform: Transform) -> Result<()> {
    let fill = match path.fill() {
        Some(fill) => fill,
        None => return Ok(()),
    };

    let color = match fill.paint() {
        Paint::Color(color) => color,
        _ => {
            return Err(Error::Unsupported(
                "SVG fill paint type is not supported".to_string(),
            ))
        }
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

fn render_stroke(path: &Path, content: &mut ContentBuilder, transform: Transform) -> Result<()> {
    let stroke = match path.stroke() {
        Some(stroke) => stroke,
        None => return Ok(()),
    };

    let color = match stroke.paint() {
        Paint::Color(color) => color,
        _ => {
            return Err(Error::Unsupported(
                "SVG stroke paint type is not supported".to_string(),
            ))
        }
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
