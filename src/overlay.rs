//! 2D drawing surface (top-left origin) that becomes a premultiplied rgba8 texture. Replaces the
//! CoreGraphics/CoreText `OverlayCanvas` of the macOS app with tiny-skia + bundled fonts, so the
//! result is identical on every platform.

use std::sync::OnceLock;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use tiny_skia::{FillRule, LineCap, Paint, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform};

use crate::gpu::T;
use crate::model::Rgba;

// MARK: - Fonts

pub const INTER: [&[u8]; 5] = [
    include_bytes!("../assets/fonts/Inter-400.ttf"),
    include_bytes!("../assets/fonts/Inter-500.ttf"),
    include_bytes!("../assets/fonts/Inter-600.ttf"),
    include_bytes!("../assets/fonts/Inter-700.ttf"),
    include_bytes!("../assets/fonts/Inter-800.ttf"),
];
pub const MONO: [&[u8]; 5] = [
    include_bytes!("../assets/fonts/JetBrainsMono-400.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-500.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-600.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-700.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-800.ttf"),
];
pub const HAND: &[u8] = include_bytes!("../assets/fonts/Caveat-700.ttf");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Medium,
    Semibold,
    Bold,
    Heavy,
}

impl Weight {
    fn index(self) -> usize {
        match self {
            Weight::Regular => 0,
            Weight::Medium => 1,
            Weight::Semibold => 2,
            Weight::Bold => 3,
            Weight::Heavy => 4,
        }
    }
}

struct Fonts {
    inter: Vec<FontRef<'static>>,
    mono: Vec<FontRef<'static>>,
    hand: FontRef<'static>,
}

fn fonts() -> &'static Fonts {
    static F: OnceLock<Fonts> = OnceLock::new();
    F.get_or_init(|| Fonts {
        inter: INTER.iter().map(|d| FontRef::try_from_slice(d).unwrap()).collect(),
        mono: MONO.iter().map(|d| FontRef::try_from_slice(d).unwrap()).collect(),
        hand: FontRef::try_from_slice(HAND).unwrap(),
    })
}

/// Picks the bundled font for a CoreText-style request (`fontName` wins over mono/weight).
fn pick(mono: bool, weight: Weight, font_name: Option<&str>) -> &'static FontRef<'static> {
    let f = fonts();
    match font_name {
        Some(n) if n.starts_with("Menlo") || n.contains("Mono") || n.starts_with("Courier") => {
            return &f.mono[if n.contains("Bold") { 3 } else { 0 }];
        }
        Some(n) if n.starts_with("Noteworthy") || n.starts_with("Marker") || n.contains("Hand") => return &f.hand,
        _ => {}
    }
    if mono { &f.mono[weight.index()] } else { &f.inter[weight.index()] }
}

/// Glyph lookup with fallback across the bundled fonts (so ▶, ●, blocks, braille... always draw).
fn glyph_font(primary: &'static FontRef<'static>, c: char) -> &'static FontRef<'static> {
    if primary.glyph_id(c).0 != 0 { return primary; }
    let f = fonts();
    for cand in [&f.mono[0], &f.inter[0]] {
        if cand.glyph_id(c).0 != 0 { return cand; }
    }
    primary
}

/// Text metrics for `s` (width, ascent, descent) without drawing.
pub fn measure(s: &str, size: f32, mono: bool, weight: Weight, font_name: Option<&str>, stretch: f32, kern: f32) -> (f32, f32, f32) {
    let font = pick(mono, weight, font_name);
    let sf = font.as_scaled(PxScale::from(size));
    let mut w = 0.0;
    let mut prev: Option<(ab_glyph::GlyphId, &FontRef)> = None;
    for c in s.chars() {
        let f = glyph_font(font, c);
        let sc = f.as_scaled(PxScale::from(size));
        let id = f.glyph_id(c);
        if let Some((pid, pf)) = prev {
            if std::ptr::eq(pf, f) { w += sc.kern(pid, id); }
        }
        w += sc.h_advance(id) + kern * size;
        prev = Some((id, f));
    }
    (w * stretch, sf.ascent(), -sf.descent())
}

// MARK: - Canvas

pub struct OverlayCanvas {
    pub w: usize,
    pub h: usize,
    pub pixmap: Pixmap,
    fill: Rgba,
    fill_a: f64,
    stroke: Rgba,
    stroke_a: f64,
    line_width: f32,
    cap: LineCap,
    dash: Option<Vec<f32>>,
    path: PathBuilder,
}

pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> (Rgba, f64) { (Rgba::new(r, g, b), a) }

fn paint(c: Rgba, a: f64) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(
        (c.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        ((c.a * a).clamp(0.0, 1.0) * 255.0).round() as u8,
    );
    p.anti_alias = true;
    p
}

impl OverlayCanvas {
    pub fn new(w: usize, h: usize) -> OverlayCanvas {
        OverlayCanvas {
            w,
            h,
            pixmap: Pixmap::new(w.max(1) as u32, h.max(1) as u32).unwrap(),
            fill: Rgba::BLACK,
            fill_a: 1.0,
            stroke: Rgba::BLACK,
            stroke_a: 1.0,
            line_width: 1.0,
            cap: LineCap::Butt,
            dash: None,
            path: PathBuilder::new(),
        }
    }

    pub fn set_fill(&mut self, c: Rgba, alpha: f64) { self.fill = c; self.fill_a = alpha; }
    pub fn set_stroke(&mut self, c: Rgba, alpha: f64) { self.stroke = c; self.stroke_a = alpha; }
    pub fn set_line_width(&mut self, w: f64) { self.line_width = w as f32; }
    pub fn set_round_cap(&mut self) { self.cap = LineCap::Round; }
    /// Dash pattern (empty = solid).
    pub fn set_dash(&mut self, lengths: &[f64]) {
        self.dash = if lengths.is_empty() { None } else { Some(lengths.iter().map(|v| *v as f32).collect()) };
    }

    pub fn move_to(&mut self, x: f64, y: f64) { self.path.move_to(x as f32, y as f32); }
    pub fn line_to(&mut self, x: f64, y: f64) { self.path.line_to(x as f32, y as f32); }
    pub fn quad_to(&mut self, cx: f64, cy: f64, x: f64, y: f64) { self.path.quad_to(cx as f32, cy as f32, x as f32, y as f32); }
    pub fn add_lines(&mut self, pts: &[(f64, f64)]) {
        if let Some(f) = pts.first() {
            self.move_to(f.0, f.1);
            for p in &pts[1..] { self.line_to(p.0, p.1); }
        }
    }
    pub fn add_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        if let Some(r) = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) { self.path.push_rect(r); }
    }
    pub fn add_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64) {
        if let Some(r) = Rect::from_xywh(x as f32, y as f32, w.max(0.01) as f32, h.max(0.01) as f32) { self.path.push_oval(r); }
    }

    fn take_path(&mut self) -> Option<tiny_skia::Path> {
        std::mem::replace(&mut self.path, PathBuilder::new()).finish()
    }

    fn stroke_style(&self) -> Stroke {
        let mut s = Stroke { width: self.line_width.max(0.01), line_cap: self.cap, ..Default::default() };
        if let Some(d) = &self.dash {
            let mut d = d.clone();
            if d.len() % 2 == 1 { d.extend(d.clone()); }
            s.dash = StrokeDash::new(d, 0.0);
        }
        s
    }

    pub fn stroke_path(&mut self) {
        if let Some(p) = self.take_path() {
            let st = self.stroke_style();
            self.pixmap.stroke_path(&p, &paint(self.stroke, self.stroke_a), &st, Transform::identity(), None);
        }
    }

    pub fn fill_path(&mut self) {
        if let Some(p) = self.take_path() {
            self.pixmap.fill_path(&p, &paint(self.fill, self.fill_a), FillRule::Winding, Transform::identity(), None);
        }
    }

    pub fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        if let Some(r) = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) {
            self.pixmap.fill_rect(r, &paint(self.fill, self.fill_a), Transform::identity(), None);
        }
    }

    pub fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.add_rect(x, y, w, h);
        self.stroke_path();
    }

    pub fn fill_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.add_ellipse(x, y, w, h);
        self.fill_path();
    }

    pub fn stroke_ellipse(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.add_ellipse(x, y, w, h);
        self.stroke_path();
    }

    fn glyph_path(&self, s: &str, size: f32, font: &'static FontRef<'static>, kern: f32) -> Option<tiny_skia::Path> {
        let mut pb = PathBuilder::new();
        let mut x = 0.0f32;
        let mut prev: Option<(ab_glyph::GlyphId, &FontRef)> = None;
        let scale = PxScale::from(size);
        for c in s.chars() {
            let f = glyph_font(font, c);
            let sc = f.as_scaled(scale);
            let id = f.glyph_id(c);
            if let Some((pid, pf)) = prev {
                if std::ptr::eq(pf, f) { x += sc.kern(pid, id); }
            }
            if let Some(outline) = f.outline(id) {
                let k = sc.h_scale_factor();
                let ky = sc.v_scale_factor();
                let mut last: Option<ab_glyph::Point> = None;
                for curve in &outline.curves {
                    use ab_glyph::OutlineCurve::*;
                    let (p0, p_end) = match curve {
                        Line(a, b) => (*a, *b),
                        Quad(a, _, b) => (*a, *b),
                        Cubic(a, _, _, b) => (*a, *b),
                    };
                    let tx = |p: ab_glyph::Point| (x + p.x * k, -p.y * ky);
                    if last.map(|l| (l.x - p0.x).abs() > 1e-3 || (l.y - p0.y).abs() > 1e-3).unwrap_or(true) {
                        if last.is_some() { pb.close(); }
                        let (a, b) = tx(p0);
                        pb.move_to(a, b);
                    }
                    match curve {
                        Line(_, b) => { let (a, bb) = tx(*b); pb.line_to(a, bb); }
                        Quad(_, c1, b) => { let (c1x, c1y) = tx(*c1); let (a, bb) = tx(*b); pb.quad_to(c1x, c1y, a, bb); }
                        Cubic(_, c1, c2, b) => {
                            let (c1x, c1y) = tx(*c1);
                            let (c2x, c2y) = tx(*c2);
                            let (a, bb) = tx(*b);
                            pb.cubic_to(c1x, c1y, c2x, c2y, a, bb);
                        }
                    }
                    last = Some(p_end);
                }
                if last.is_some() { pb.close(); }
            }
            x += sc.h_advance(id) + kern * size;
            prev = Some((id, f));
        }
        pb.finish()
    }

    /// Draws text with its top-left at (x, y). Returns the text size.
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &mut self, s: &str, x: f64, y: f64, size: f64, color: Rgba, weight: Weight, mono: bool, shadow: bool, background: Option<Rgba>,
        padding: f64, font_name: Option<&str>, stretch: f64, outline: Option<Rgba>, kern: f64,
    ) -> (f64, f64) {
        let size_f = size as f32;
        let font = pick(mono, weight, font_name);
        let (width, ascent, descent) = measure(s, size_f, mono, weight, font_name, stretch as f32, kern as f32);
        let (width, height) = (width as f64, (ascent + descent) as f64);
        if let Some(bg) = background {
            self.set_fill(bg, 1.0);
            self.fill_rect(x - padding, y - padding, width + padding * 2.0, height + padding * 2.0);
        }
        let Some(path) = self.glyph_path(s, size_f, font, kern as f32) else { return (width, height) };
        let tr = Transform::from_row(stretch as f32, 0.0, 0.0, 1.0, x as f32, (y + ascent as f64) as f32);
        if shadow {
            // soft drop shadow (down-right), like the CoreGraphics shadow of the macOS app
            let mut m = Pixmap::new(self.pixmap.width(), self.pixmap.height()).unwrap();
            let off = (size * 0.08) as f32;
            let tr2 = tr.post_translate(off, off);
            m.fill_path(&path, &paint(Rgba::BLACK, 0.8), FillRule::Winding, tr2, None);
            if let Some(o) = outline {
                let st = Stroke { width: (size * 0.18) as f32, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
                let _ = o;
                m.stroke_path(&path, &paint(Rgba::BLACK, 0.8), &st, tr2, None);
            }
            blur_pixmap(&mut m, (size * 0.15 / 2.0) as f32);
            self.pixmap.draw_pixmap(0, 0, m.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        }
        if let Some(o) = outline {
            // stroked copy underneath for the classic outlined OSD look
            let st = Stroke { width: (size * 0.18) as f32, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
            self.pixmap.stroke_path(&path, &paint(o, 1.0), &st, tr, None);
        }
        self.pixmap.fill_path(&path, &paint(color, 1.0), FillRule::Winding, tr, None);
        (width, height)
    }

    /// White text with its em box scaled by (sx, sy) from the top-left corner (used for block glyphs that fill a cell).
    pub fn text_scaled(&mut self, s: &str, x: f64, y: f64, size: f64, sx: f64, sy: f64, weight: Weight) {
        let font = pick(true, weight, None);
        let (_, ascent, _) = measure(s, size as f32, true, weight, None, 1.0, 0.0);
        if let Some(path) = self.glyph_path(s, size as f32, font, 0.0) {
            let tr = Transform::from_row(sx as f32, 0.0, 0.0, sy as f32, x as f32, (y + ascent as f64 * sy) as f32);
            self.pixmap.fill_path(&path, &paint(Rgba::WHITE, 1.0), FillRule::Winding, tr, None);
        }
    }

    /// Simple text call with the common defaults (semibold mono, no extras).
    pub fn text_simple(&mut self, s: &str, x: f64, y: f64, size: f64, color: Rgba) -> (f64, f64) {
        self.text(s, x, y, size, color, Weight::Semibold, true, false, None, 0.0, None, 1.0, None, 0.0)
    }

    /// Premultiplied rgba8 bytes.
    pub fn data(&self) -> &[u8] { self.pixmap.data() }

    /// Alpha (0…1) at a pixel.
    pub fn alpha_at(&self, x: usize, y: usize) -> f32 {
        if x >= self.w || y >= self.h { return 0.0; }
        self.pixmap.data()[(y * self.w + x) * 4 + 3] as f32 / 255.0
    }

    /// Uploads as an rgba8 texture (premultiplied; shaders unpremultiply).
    pub fn texture(&self) -> T { crate::gpu::gpu().texture_from_rgba8(self.pixmap.data(), self.w, self.h) }
}

/// Approximate gaussian blur (3 box passes) of a premultiplied pixmap.
pub fn blur_pixmap(p: &mut Pixmap, sigma: f32) {
    if sigma < 0.5 { return; }
    let (w, h) = (p.width() as usize, p.height() as usize);
    let r = ((sigma * 1.7) as usize).max(1);
    let data = p.data_mut();
    let mut tmp = vec![0u8; data.len()];
    for _ in 0..3 {
        box_pass(data, &mut tmp, w, h, r, true);
        box_pass(&tmp, data, w, h, r, false);
    }
}

fn box_pass(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, horiz: bool) {
    let (n, m) = if horiz { (w, h) } else { (h, w) };
    let idx = |i: usize, j: usize| if horiz { (j * w + i) * 4 } else { (i * w + j) * 4 };
    let div = (2 * r + 1) as u32;
    for j in 0..m {
        let mut acc = [0u32; 4];
        for k in 0..=r.min(n - 1) {
            for c in 0..4 { acc[c] += src[idx(k, j) + c] as u32; }
        }
        for c in 0..4 { acc[c] += src[idx(0, j) + c] as u32 * r as u32; }
        for i in 0..n {
            for c in 0..4 { dst[idx(i, j) + c] = (acc[c] / div) as u8; }
            let add = (i + r + 1).min(n - 1);
            let sub = i.saturating_sub(r);
            for c in 0..4 {
                acc[c] += src[idx(add, j) + c] as u32;
                acc[c] -= src[idx(sub, j) + c] as u32;
            }
        }
    }
}
