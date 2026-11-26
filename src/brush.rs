//! Brush masks: strokes (image coordinates) rasterised on the CPU into a soft coverage map at a
//! reduced size, cached, then uploaded and smoothly upscaled on the GPU. Lightroom-style dabs:
//! each dab adds `flow` toward full coverage (paint) or removes it (erase).


use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

use crate::model::{BrushStroke, CropRect};

/// Longest side of the raster (brush masks are soft; the GPU upscales them smoothly).
pub const MAX_SIDE: usize = 1280;

/// Raster size for a render of `w`×`h`.
pub fn raster_size(w: usize, h: usize) -> (usize, usize) {
    let s = (MAX_SIDE as f64 / w.max(h) as f64).min(1.0);
    (((w as f64 * s).round() as usize).max(1), ((h as f64 * s).round() as usize).max(1))
}

fn cache() -> &'static Mutex<Vec<(u64, Arc<Vec<f32>>)>> {
    static C: OnceLock<Mutex<Vec<(u64, Arc<Vec<f32>>)>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(vec![]))
}

/// Coverage (row-major, 0..1) of `strokes` over the cropped image at `w`×`h`.
/// `aspect` = image width and height divided by its long side.
pub fn raster(strokes: &[BrushStroke], w: usize, h: usize, crop: &CropRect, aspect: (f64, f64)) -> Arc<Vec<f32>> {
    let mut hs = std::collections::hash_map::DefaultHasher::new();
    (w, h).hash(&mut hs);
    for v in [crop.x, crop.y, crop.w, crop.h, aspect.0, aspect.1] {
        v.to_bits().hash(&mut hs);
    }
    crate::model::MaskShape::Brush { strokes: strokes.to_vec() }.hash_into(&mut hs);
    let key = hs.finish();
    if let Some(r) = cache().lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(k, _)| *k == key) {
        return r.1.clone();
    }
    let mut buf = vec![0f32; w * h];
    // render px per long-side unit, and the mapping image -> raster
    let ppu = w as f64 / (crop.w * aspect.0).max(1e-6);
    let to_px = |p: &[f32; 2]| -> (f64, f64) { ((p[0] as f64 - crop.x) / crop.w * w as f64, (p[1] as f64 - crop.y) / crop.h * h as f64) };
    for s in strokes {
        let r = (s.size as f64 * ppu).max(0.5);
        let spacing = (r * 0.18).max(0.5);
        let mut dabs: Vec<(f64, f64)> = vec![];
        let mut prev: Option<(f64, f64)> = None;
        let mut carry = 0.0;
        for p in &s.points {
            let q = to_px(p);
            match prev {
                None => dabs.push(q),
                Some(a) => {
                    let (dx, dy) = (q.0 - a.0, q.1 - a.1);
                    let len = (dx * dx + dy * dy).sqrt();
                    let mut t = spacing - carry;
                    while t <= len {
                        dabs.push((a.0 + dx * t / len, a.1 + dy * t / len));
                        t += spacing;
                    }
                    carry = len - (t - spacing);
                }
            }
            prev = Some(q);
        }
        // flow per dab, so overlapping dabs along the stroke build up like Lightroom's
        let flow = (s.flow as f64).clamp(0.01, 1.0) as f32;
        let hard = (1.0 - s.feather as f64).clamp(0.0, 1.0);
        // one coverage map per stroke (max of its dabs), then composited: a stroke never darkens itself
        let x0 = dabs.iter().map(|d| d.0).fold(f64::MAX, f64::min) - r - 1.0;
        let x1 = dabs.iter().map(|d| d.0).fold(f64::MIN, f64::max) + r + 1.0;
        let y0 = dabs.iter().map(|d| d.1).fold(f64::MAX, f64::min) - r - 1.0;
        let y1 = dabs.iter().map(|d| d.1).fold(f64::MIN, f64::max) + r + 1.0;
        let (bx0, bx1) = (x0.max(0.0) as usize, (x1.max(0.0) as usize).min(w));
        let (by0, by1) = (y0.max(0.0) as usize, (y1.max(0.0) as usize).min(h));
        if bx0 >= bx1 || by0 >= by1 {
            continue;
        }
        let bw = bx1 - bx0;
        let mut cov = vec![0f32; bw * (by1 - by0)];
        for (cx, cy) in &dabs {
            let (dx0, dx1) = (((cx - r).floor().max(bx0 as f64)) as usize, ((cx + r).ceil().max(0.0) as usize + 1).min(bx1));
            let (dy0, dy1) = (((cy - r).floor().max(by0 as f64)) as usize, ((cy + r).ceil().max(0.0) as usize + 1).min(by1));
            for y in dy0..dy1 {
                let fy = y as f64 + 0.5 - cy;
                for x in dx0..dx1 {
                    let fx = x as f64 + 0.5 - cx;
                    let d = (fx * fx + fy * fy).sqrt() / r;
                    if d >= 1.0 {
                        continue;
                    }
                    let a = if d <= hard { 1.0 } else { let t = (d - hard) / (1.0 - hard).max(1e-6); 1.0 - t * t * (3.0 - 2.0 * t) } as f32;
                    let i = (y - by0) * bw + (x - bx0);
                    // dabs of one stroke accumulate with the flow (Lightroom's build-up)
                    let c = &mut cov[i];
                    *c += (1.0 - *c) * a * flow;
                }
            }
        }
        for y in by0..by1 {
            for x in bx0..bx1 {
                let c = cov[(y - by0) * bw + (x - bx0)];
                if c <= 0.0 {
                    continue;
                }
                let v = &mut buf[y * w + x];
                *v = if s.erase { *v * (1.0 - c) } else { *v + (1.0 - *v) * c };
            }
        }
    }
    let out = Arc::new(buf);
    let mut c = cache().lock().unwrap_or_else(|e| e.into_inner());
    c.insert(0, (key, out.clone()));
    c.truncate(6);
    out
}

