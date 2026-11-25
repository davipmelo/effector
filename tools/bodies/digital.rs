//== header
use crate::overlay::{OverlayCanvas, Weight};

/// Encodes the texture as a real JPEG `generations` times and decodes it back.
pub fn jpeg_roundtrip(ctx: &mut Ctx, tex: &T, quality: f64, generations: usize) -> T {
    let (w, h) = (tex.width(), tex.height());
    let f = ctx.read_floats(tex);
    let mut rgb: Vec<u8> = Vec::with_capacity(w * h * 3);
    for i in 0..w * h {
        for c in 0..3 {
            rgb.push((f[i * 4 + c].clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    for g in 0..generations.max(1) {
        // successive generations drift slightly in quality, like re-saved files
        let q = (quality + (g % 3) as f64 * 0.03).clamp(0.0, 1.0);
        let mut out = Vec::new();
        let mut enc = jpeg_encoder::Encoder::new(&mut out, ((q * 100.0).round() as u8).clamp(1, 100));
        enc.set_sampling_factor(jpeg_encoder::SamplingFactor::R_4_2_0);
        if enc.encode(&rgb, w as u16, h as u16, jpeg_encoder::ColorType::Rgb).is_err() {
            break;
        }
        match image::load_from_memory_with_format(&out, image::ImageFormat::Jpeg) {
            Ok(img) => rgb = img.to_rgb8().into_raw(),
            Err(_) => break,
        }
    }
    let mut rgba = Vec::with_capacity(w * h * 4);
    for i in 0..w * h {
        rgba.extend_from_slice(&[rgb[i * 3], rgb[i * 3 + 1], rgb[i * 3 + 2], 255]);
    }
    let up = ctx.gpu().texture_from_rgba8(&rgba, w, h);
    ctx.apply("k_copy", &[&up], &[])
}
//== digicam
        let (w, h) = (input.width(), input.height());
        let res = (p.num("resolution", 45.0) / 100.0).max(0.05);
        let cam_w = ((w as f64 * res) as usize).max(8);
        let cam_h = ((h as f64 * res) as usize).max(8);
        let mut img = ctx.resize(input, cam_w, cam_h, false);
        let unit = (ctx.scale * res) as f32;
        let bm = ctx.apply("k_brightmask", &[&img], &[]);
        let bright = ctx.blur(&bm, (unit as f64 * 3.0).max(1.0));
        let noise_on = p.bool_or("noiseOn", true);
        img = ctx.apply("k_digicam", &[&img, &bright], &[
            p.fd("seed", 1.0) * 4.1, unit, p.fd("wb", -25.0) / 100.0, p.fd("saturation", 115.0) / 100.0,
            p.fd("contrast", 118.0) / 100.0, p.fd("clip", 45.0) / 100.0,
            if p.bool_or("fringeOn", true) { p.fd("fringe", 40.0) / 100.0 } else { 0.0 },
            if noise_on { p.fd("noise", 35.0) / 100.0 } else { 0.0 },
            if noise_on { p.fd("chromaNoise", 40.0) / 100.0 } else { 0.0 }, p.fd("exposure", 0.2),
        ]);
        let sh = if p.bool_or("sharpOn", true) { p.num("sharpen", 120.0) / 100.0 } else { 0.0 };
        if sh > 0.01 {
            let b = ctx.blur(&img, (unit as f64 * 1.2).max(0.6));
            img = ctx.apply("k_unsharp", &[&img, &b], &[sh as f32, 0.0, 1.0]);
        }
        img = ctx.resize(&img, w, h, true);
        if p.bool_or("date", true) {
            let mut ov = OverlayCanvas::new(w, h);
            let size = ctx.px(p.num("dateSize", 28.0));
            let text = p.text_or("dateText", "'03 8 14");
            let color = p.color_or("dateColor", hex("#FF8A1E"));
            let margin = w.min(h) as f64 * 0.055;
            let est = text.chars().count() as f64 * size * 0.62;
            let (wf, hf) = (w as f64, h as f64);
            let pos = match p.choice_or("datePos", "br").as_str() {
                "bl" => (margin, hf - margin - size),
                "tr" => (wf - margin - est, margin),
                "tl" => (margin, margin),
                _ => (wf - margin - est, hf - margin - size),
            };
            ov.text(&text, pos.0, pos.1, size, color, Weight::Bold, true, false, None, 0.0, Some("Menlo-Bold"), 1.0, None, 0.06);
            // a soft glow like the LED imprint
            let t = ov.texture();
            let mut glow = ctx.apply("k_copy", &[&t], &[]);
            glow = ctx.blur(&glow, size * 0.12);
            img = ctx.apply("k_over_pm_op", &[&img, &glow], &[0.6]);
            img = ctx.apply("k_over_pm_op", &[&img, &t], &[0.95]);
        }
        let q = p.num("quality", 55.0) / 100.0;
        if q < 0.99 && p.bool_or("jpegOn", true) {
            // compress at camera resolution so the blocks have the right size
            let small = ctx.resize(&img, cam_w, cam_h, false);
            let j = jpeg_roundtrip(ctx, &small, q, 1);
            img = ctx.resize(&j, w, h, true);
        }
        img
