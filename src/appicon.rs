//! The EFFECTOR app icon (a dithered yellow "E" on black), drawn in code so the window icon,
//! the Windows .ico and the macOS .icns all come from one source (port of scripts/make_icon.swift).

use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Shader, Stroke, Transform};

use crate::overlay::{OverlayCanvas, Weight};

fn rounded_rect(r: Rect, radius: f32) -> tiny_skia::Path {
    let mut pb = PathBuilder::new();
    let (x0, y0, x1, y1) = (r.left(), r.top(), r.right(), r.bottom());
    let k = radius * 0.5523;
    pb.move_to(x0 + radius, y0);
    pb.line_to(x1 - radius, y0);
    pb.cubic_to(x1 - radius + k, y0, x1, y0 + radius - k, x1, y0 + radius);
    pb.line_to(x1, y1 - radius);
    pb.cubic_to(x1, y1 - radius + k, x1 - radius + k, y1, x1 - radius, y1);
    pb.line_to(x0 + radius, y1);
    pb.cubic_to(x0 + radius - k, y1, x0, y1 - radius + k, x0, y1 - radius);
    pb.line_to(x0, y0 + radius);
    pb.cubic_to(x0, y0 + radius - k, x0 + radius - k, y0, x0 + radius, y0);
    pb.close();
    pb.finish().unwrap()
}

/// Premultiplied pixmap of the icon at `size` px (top-down).
pub fn render(size: u32) -> Pixmap {
    let s = size as f32;
    let mut pm = Pixmap::new(size, size).unwrap();
    // macOS icon grid: 824/1024 body with rounded corners
    let inset = s * 100.0 / 1024.0;
    let body = Rect::from_xywh(inset, inset, s - inset * 2.0, s - inset * 2.0).unwrap();
    let radius = body.width() * 0.225;
    let path = rounded_rect(body, radius);
    // soft shadow
    {
        let mut sh = Pixmap::new(size, size).unwrap();
        let mut p = Paint::default();
        p.set_color_rgba8(0, 0, 0, 115);
        sh.fill_path(&path, &p, FillRule::Winding, Transform::from_translate(0.0, s * 0.01), None);
        crate::overlay::blur_pixmap(&mut sh, s * 0.0125);
        pm.draw_pixmap(0, 0, sh.as_ref(), &Default::default(), Transform::identity(), None);
    }
    // body gradient (lighter at the top)
    let mut p = Paint::default();
    p.shader = tiny_skia::LinearGradient::new(
        tiny_skia::Point::from_xy(0.0, body.top()),
        tiny_skia::Point::from_xy(0.0, body.bottom()),
        vec![
            tiny_skia::GradientStop::new(0.0, tiny_skia::Color::from_rgba8(33, 33, 33, 255)),
            tiny_skia::GradientStop::new(1.0, tiny_skia::Color::from_rgba8(8, 8, 8, 255)),
        ],
        tiny_skia::SpreadMode::Pad,
        Transform::identity(),
    )
    .unwrap_or(Shader::SolidColor(tiny_skia::Color::from_rgba8(13, 13, 13, 255)));
    p.anti_alias = true;
    pm.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
    // glyph mask for "E" (256 px, top-down)
    let ms = 256usize;
    let mut mask = OverlayCanvas::new(ms, ms);
    let (w, asc, desc) = crate::overlay::measure("E", 250.0, false, Weight::Heavy, None, 1.0, 0.0);
    let gh = asc + desc;
    // centre the glyph's ink box (cap height ≈ 0.727 em for Inter)
    let cap = 250.0 * 0.727;
    let top = (ms as f32 - cap) / 2.0 - (asc - cap);
    let _ = gh;
    mask.text("E", ((ms as f32 - w) / 2.0) as f64, top as f64, 250.0, crate::model::Rgba::WHITE, Weight::Heavy, false, false, None, 0.0, None, 1.0, None, 0.0);
    // dither: grid of cells inside the body, Bayer 4×4 against a vertical gradient
    let bayer = [[0.0, 8.0, 2.0, 10.0], [12.0, 4.0, 14.0, 6.0], [3.0, 11.0, 1.0, 9.0], [15.0, 7.0, 13.0, 5.0]];
    let cells = 24usize;
    let grid = Rect::from_xywh(body.left() + body.width() * 0.1, body.top() + body.height() * 0.1, body.width() * 0.8, body.height() * 0.8).unwrap();
    let cw = grid.width() / cells as f32;
    let mut yellow = Paint::default();
    yellow.set_color_rgba8(255, 187, 0, 255);
    yellow.anti_alias = true;
    for gy in 0..cells {
        for gx in 0..cells {
            // the macOS version counts gy from the bottom (CoreGraphics)
            let gy_top = cells - 1 - gy;
            let mx = ((gx as f32 + 0.5) / cells as f32 * ms as f32) as usize;
            let my = ((gy_top as f32 + 0.5) / cells as f32 * ms as f32) as usize;
            if mask.alpha_at(mx, my) < 0.5 {
                continue;
            }
            let t = 0.3 + 1.0 * (gy as f64 / (cells - 1) as f64) - 0.1 * (gx as f64 / (cells - 1) as f64);
            let threshold = (bayer[gy % 4][gx % 4] + 0.5) / 16.0;
            if t < threshold {
                continue;
            }
            let x = grid.left() + gx as f32 * cw + cw * 0.06;
            let y = grid.top() + gy_top as f32 * cw + cw * 0.06;
            if let Some(r) = Rect::from_xywh(x, y, cw * 0.88, cw * 0.88) {
                pm.fill_rect(r, &yellow, Transform::identity(), None);
            }
        }
    }
    // subtle highlight edge
    let mut hl = Paint::default();
    hl.set_color_rgba8(255, 255, 255, 20);
    hl.anti_alias = true;
    pm.stroke_path(&path, &hl, &Stroke { width: s * 0.004, ..Default::default() }, Transform::identity(), None);
    pm
}

/// Straight-alpha RGBA bytes.
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let pm = render(size);
    pm.data()
        .chunks(4)
        .flat_map(|p| {
            let a = p[3] as u32;
            if a == 0 {
                [0, 0, 0, 0]
            } else {
                [(p[0] as u32 * 255 / a) as u8, (p[1] as u32 * 255 / a) as u8, (p[2] as u32 * 255 / a) as u8, p[3]]
            }
        })
        .collect()
}

/// PNG bytes of the icon.
pub fn icon_png(size: u32) -> Vec<u8> {
    let rgba = icon_rgba(size);
    crate::imageio::encode(&rgba, size as usize, size as usize, crate::imageio::Format::Png, 1.0, true).unwrap_or_default()
}

/// Windows .ico with the standard sizes (PNG-compressed entries).
pub fn write_ico(path: &std::path::Path) -> std::io::Result<()> {
    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let pngs: Vec<Vec<u8>> = sizes.iter().map(|s| icon_png(*s)).collect();
    let mut out: Vec<u8> = vec![0, 0, 1, 0, sizes.len() as u8, 0];
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (s, png) in sizes.iter().zip(&pngs) {
        let d = if *s >= 256 { 0 } else { *s as u8 };
        out.extend_from_slice(&[d, d, 0, 0, 1, 0, 32, 0]);
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for png in pngs {
        out.extend_from_slice(&png);
    }
    std::fs::write(path, out)
}

/// macOS .icns (PNG entries ic07…ic14).
pub fn write_icns(path: &std::path::Path) -> std::io::Result<()> {
    let entries: [(&[u8; 4], u32); 7] = [(b"ic07", 128), (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024), (b"ic11", 32), (b"ic12", 64), (b"ic13", 256)];
    let mut body: Vec<u8> = vec![];
    for (tag, s) in entries {
        let png = icon_png(s);
        body.extend_from_slice(tag);
        body.extend_from_slice(&((png.len() + 8) as u32).to_be_bytes());
        body.extend_from_slice(&png);
    }
    let mut out: Vec<u8> = b"icns".to_vec();
    out.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
    out.extend(body);
    std::fs::write(path, out)
}
