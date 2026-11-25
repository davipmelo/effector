//== header
use crate::overlay::{OverlayCanvas, Weight};

/// VCR-style on-screen text: chunky, slightly stretched monospace with an outline.
fn osd(p: &Params, ctx: &Ctx, w: usize, h: usize, camcorder: bool) -> OverlayCanvas {
    let mut ov = OverlayCanvas::new(w, h);
    let size = ctx.px(p.num("osdSize", 30.0));
    let margin = w.min(h) as f64 * 0.07;
    let color = p.color_or("osdColor", Rgba::WHITE);
    let outline = if camcorder { Some(Rgba::new(0.05, 0.05, 0.05)) } else { None };
    let mut draw = |ov: &mut OverlayCanvas, s: &str, x: f64, y: f64| {
        ov.text(s, x, y, size, color, Weight::Heavy, true, !camcorder, None, 0.0, None, 1.18, outline, 0.04);
    };
    let top = p.text_or("osdTop", "PLAY ▶");
    if !top.is_empty() {
        draw(&mut ov, &top, margin, margin);
    }
    let bottom = p.text_or("osdBottom", "AM 10:24\nNOV. 24 1997");
    let lines: Vec<&str> = bottom.split('\n').collect();
    let lh = size * 1.2;
    let mut y = h as f64 - margin - lines.len() as f64 * lh;
    for l in &lines {
        if !(!l.is_empty() || lines.len() > 1) {
            continue;
        }
        if camcorder {
            // camcorders print the date bottom-right
            let est = l.chars().count() as f64 * size * 0.62 * 1.18;
            draw(&mut ov, l, w as f64 - margin - est, y);
        } else {
            draw(&mut ov, l, margin, y);
        }
        y += lh;
    }
    ov
}
//== v_h_s -> vhs
        let on = |k: &str| if p.bool_or(k, true) { 1.0f32 } else { 0.0 };
        let seed = p.fd("seed", 1.0) * 7.31;
        let unit = ctx.scale as f32;
        let line_h = ctx.px(1.5).round().max(1.0) as f32;
        let osd_on = p.bool_or("osd", true);
        let burn_in = p.choice_or("osdStyle", "vcr") == "camcorder";
        let mut src = input.clone();
        // Camcorder date stamps are recorded on tape: draw them before the degradation.
        if osd_on && burn_in {
            let t = osd(p, ctx, input.width(), input.height(), true).texture();
            src = ctx.apply("k_over_pm", &[&src, &t], &[]);
        }
        let geo = ctx.apply("k_vhs_geo", &[&src], &[
            seed, unit, on("tracking"), p.fd("trackAmt", 5.0), p.fd("bandPos", 82.0) / 100.0, p.fd("bandH", 5.0) / 100.0,
            on("wobble"), p.fd("wobbleAmt", 1.5), p.fd("wobbleFreq", 3.0), on("headSwitch"), p.fd("headH", 2.5) / 100.0, line_h,
        ]);
        let soft_on = p.bool_or("soft", true);
        let color_on = p.bool_or("colorOn", true);
        let u = [
            seed, unit, if soft_on { p.fd("softAmt", 3.0) } else { 0.3 }, if soft_on { p.fd("sharpen", 35.0) / 100.0 } else { 0.0 },
            if soft_on { on("chroma") } else { 0.0 }, p.fd("bleed", 10.0), p.fd("shift", 3.0),
            if soft_on { p.fd("chromaNoise", 25.0) / 100.0 } else { 0.0 },
            if color_on { p.fd("saturation", 85.0) / 100.0 * (1.0 - p.fd("desat", 0.0) / 100.0) } else { 1.0 },
            on("noise"), p.fd("noiseAmt", 40.0) / 100.0, on("dropouts"), p.fd("dropAmt", 30.0) / 100.0,
            on("scanlines"), p.fd("scanAmt", 18.0) / 100.0, ctx.px(p.num("scanSpacing", 3.0)).round().max(2.0) as f32,
            on("vignette"), p.fd("vigAmt", 30.0) / 100.0, if color_on { p.fd("blackLift", 6.0) / 100.0 } else { 0.0 },
            if color_on { p.fd("rolloff", 40.0) / 100.0 } else { 0.0 },
            if color_on { p.fd("warmth", 10.0) / 100.0 } else { 0.0 }, if color_on { p.fd("tint", 8.0) / 100.0 } else { 0.0 },
            on("tracking"), p.fd("bandPos", 82.0) / 100.0, p.fd("bandH", 5.0) / 100.0, p.fd("trackAmt", 5.0),
            on("headSwitch"), p.fd("headH", 2.5) / 100.0, line_h, if color_on { p.fd("contrast", 92.0) / 100.0 } else { 1.0 },
        ];
        let mut out = ctx.apply("k_vhs", &[&geo, input], &u);
        if osd_on && !burn_in {
            let t = osd(p, ctx, input.width(), input.height(), false).texture();
            out = ctx.apply("k_over_pm", &[&out, &t], &[]);
        }
        out
