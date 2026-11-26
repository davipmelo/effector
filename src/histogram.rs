//! Histogram and vectorscope of the rendered image (what is on screen), computed on the render
//! thread from a small copy of the output. 256 levels per channel on the sRGB-encoded values, like
//! Lightroom; the vectorscope bins Rec. 709 Cb/Cr and keeps the average colour of each bin (for a
//! colourised scope, like Resolve).

use crate::gpu::{Ctx, T};

#[derive(Clone)]
pub struct Histogram {
    pub r: Vec<u32>,
    pub g: Vec<u32>,
    pub b: Vec<u32>,
    /// Rec. 709 luminance of the encoded values.
    pub l: Vec<u32>,
    /// Pixels counted (fully transparent ones are left out).
    pub total: u32,
    /// Vectorscope: SCOPE×SCOPE bins over Cb (x, −0.5..0.5) and Cr (y, +0.5 at the top).
    pub scope: Vec<u32>,
    /// Sum of the colours that fell in each bin.
    pub scope_rgb: Vec<[f32; 3]>,
}

/// Vectorscope resolution (bins per side).
pub const SCOPE: usize = 160;

/// Rec. 709 chroma of an sRGB-encoded colour (Cb, Cr in −0.5..0.5).
pub fn cbcr(r: f32, g: f32, b: f32) -> (f32, f32) {
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    ((b - y) / 1.8556, (r - y) / 1.5748)
}

/// Longest side of the copy the histogram is measured on.
const SIZE: f64 = 640.0;

impl Histogram {
    pub fn of(ctx: &mut Ctx, out: &T) -> Histogram {
        let f = (SIZE / out.width().max(out.height()) as f64).min(1.0);
        let (w, h) = (((out.width() as f64 * f) as usize).max(1), ((out.height() as f64 * f) as usize).max(1));
        // bilinear: close to point sampling, so clipped pixels stay clipped (an averaging filter would hide them)
        let small = if f < 1.0 { ctx.resize(out, w, h, true) } else { out.clone() };
        let px = ctx.read_floats(&small);
        Histogram::from_rgba(&px)
    }

    pub fn from_rgba(px: &[f32]) -> Histogram {
        let mut hst = Histogram { r: vec![0; 256], g: vec![0; 256], b: vec![0; 256], l: vec![0; 256], total: 0, scope: vec![0; SCOPE * SCOPE], scope_rgb: vec![[0.0; 3]; SCOPE * SCOPE] };
        let level = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as usize;
        for p in px.chunks_exact(4) {
            if p[3] < 0.5 {
                continue;
            }
            let (r, g, b) = (p[0].clamp(0.0, 1.0), p[1].clamp(0.0, 1.0), p[2].clamp(0.0, 1.0));
            hst.r[level(r)] += 1;
            hst.g[level(g)] += 1;
            hst.b[level(b)] += 1;
            hst.l[level(0.2126 * r + 0.7152 * g + 0.0722 * b)] += 1;
            hst.total += 1;
            let (cb, cr) = cbcr(r, g, b);
            let x = (((cb + 0.5) * SCOPE as f32) as usize).min(SCOPE - 1);
            let y = (((0.5 - cr) * SCOPE as f32) as usize).min(SCOPE - 1);
            let i = y * SCOPE + x;
            hst.scope[i] += 1;
            let s = &mut hst.scope_rgb[i];
            s[0] += r;
            s[1] += g;
            s[2] += b;
        }
        hst
    }

    /// Share of pixels (0..1) at level 0 / 255 per channel: [r, g, b, luminance].
    pub fn clipped(&self, high: bool) -> [f32; 4] {
        let i = if high { 255 } else { 0 };
        let t = self.total.max(1) as f32;
        [self.r[i] as f32 / t, self.g[i] as f32 / t, self.b[i] as f32 / t, self.l[i] as f32 / t]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_levels_and_clipping() {
        // one black, one white, one mid-grey, one transparent pixel
        let px = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 0.5, 0.5, 1.0, 1.0, 1.0, 1.0, 0.0];
        let h = Histogram::from_rgba(&px);
        assert_eq!(h.total, 3);
        assert_eq!(h.r[0], 1);
        assert_eq!(h.l[255], 1);
        assert_eq!(h.g[128], 1);
        assert!((h.clipped(true)[3] - 1.0 / 3.0).abs() < 1e-6);
        // greys sit in the centre of the vectorscope
        let c = (SCOPE / 2) * SCOPE + SCOPE / 2;
        assert_eq!(h.scope[c], 3);
        // pure red is up and to the left (Cr > 0, Cb < 0)
        let (cb, cr) = cbcr(1.0, 0.0, 0.0);
        assert!(cb < 0.0 && cr > 0.4);
    }
}
