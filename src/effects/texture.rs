use super::*;
use crate::overlay::OverlayCanvas;
use crate::swatches::SplitMix;

pub fn dust() -> Effect {
    Effect::new("dust", "Dust & Scratches", Category::Overlay, |input, p, ctx| {
        let (w, h) = (input.width() as f64, input.height() as f64);
        let s = ctx.scale;
        let mut rng = SplitMix::new((p.num("seed", 1.0).max(1.0) as u64).wrapping_mul(2654435761));
        let mut r = move || rng.unit();
        let style = p.choice_or("style", "light");
        let size = p.num("size", 1.0);
        let mut ov = OverlayCanvas::new(input.width(), input.height());
        let mut color = |r: &mut dyn FnMut() -> f64| -> (Rgba, f64) {
            let dark = style == "dark" || (style == "mixed" && r() < 0.5);
            let a = 0.55 + r() * 0.45;
            if dark { (Rgba::new(0.06, 0.06, 0.06), a) } else { (Rgba::new(0.96, 0.96, 0.96), a) }
        };
        // specks: many tiny, a few larger clumps
        let ref_area = (w / s) * (h / s);
        let n = if p.bool_or("dustOn", true) { (p.num("amount", 35.0) / 100.0 * ref_area * 0.0012) as i64 } else { 0 };
        for _ in 0..n.min(40000) {
            let x = r() * w;
            let y = r() * h;
            let rad = (0.35 + r().powi(5) * 5.5) * size * s;
            let (c, a) = color(&mut r);
            ov.set_fill(c, a);
            let parts = if rad > 2.0 * s { 3 } else { 1 };
            for _ in 0..parts {
                let ox = (r() - 0.5) * rad;
                let oy = (r() - 0.5) * rad;
                let rw = rad * (0.5 + r() * 0.8);
                let rh = rad * (0.5 + r() * 0.8);
                ov.fill_ellipse(x + ox - rw / 2.0, y + oy - rh / 2.0, rw, rh);
            }
        }
        // hairs / fibres
        ov.set_round_cap();
        let hairs = if p.bool_or("hairsOn", true) { p.num("hairs", 5.0) as i64 } else { 0 };
        for _ in 0..hairs.max(0) {
            let (c, a) = color(&mut r);
            ov.set_stroke(c, a);
            ov.set_line_width((0.5 + r() * 0.8) * s * size);
            let px = r() * w;
            let py = r() * h;
            let mut pt = (px, py);
            ov.move_to(pt.0, pt.1);
            let mut ang = r() * std::f64::consts::PI * 2.0;
            let segs = 3 + (r() * 4.0) as i64;
            let len = (15.0 + r() * 70.0) * s * size;
            for _ in 0..segs {
                ang += (r() - 0.5) * 1.6;
                let to = (pt.0 + ang.cos() * len / segs as f64, pt.1 + ang.sin() * len / segs as f64);
                let cx = (pt.0 + to.0) / 2.0 + (r() - 0.5) * len * 0.3;
                let cy = (pt.1 + to.1) / 2.0 + (r() - 0.5) * len * 0.3;
                ov.quad_to(cx, cy, to.0, to.1);
                pt = to;
            }
            ov.stroke_path();
        }
        // scratches run along the film (vertical)
        let len_f = p.num("scratchLen", 55.0) / 100.0;
        let scratches = if p.bool_or("scratchesOn", true) { p.num("scratches", 3.0) as i64 } else { 0 };
        for _ in 0..scratches.max(0) {
            let light = style != "dark" && (style == "light" || r() < 0.6);
            let a = 0.25 + r() * 0.5;
            if light { ov.set_stroke(Rgba::new(0.95, 0.95, 0.95), a) } else { ov.set_stroke(Rgba::new(0.05, 0.05, 0.05), a) }
            ov.set_line_width((0.35 + r() * 0.9) * s);
            let x = r() * w;
            let len = h * len_f * (0.5 + r() * 0.5);
            let y0 = r() * (h - len).max(1.0);
            ov.move_to(x, y0);
            let steps = 8;
            for i in 1..=steps {
                ov.line_to(x + (r() - 0.5) * 1.5 * s, y0 + len * i as f64 / steps as f64);
            }
            ov.stroke_path();
        }
        let otex = ov.texture();
        let mut tex = ctx.apply("k_copy", &[&otex], &[]);
        let soft = p.num("softness", 0.5);
        if soft > 0.01 {
            tex = ctx.blur(&tex, ctx.px(soft));
        }
        ctx.apply("k_over_pm_op", &[input, &tex], &[p.fd("opacity", 80.0) / 100.0])
    })
    .symbol("aqi.low")
    .summary("Specks, hairs and film scratches, like a dusty scan or an old print.")
    .params(vec![
        ParamDef::segmented("style", "Style", &[("dark", "Dark"), ("light", "Light"), ("mixed", "Mixed")], "light").sec("Look"),
        ParamDef::section_switch("dustOn", "Dust", true),
        s("amount", "Density", 0.0, 100.0, 35.0).step(1.0).unit("%").curve(1.5).sec("Dust"),
        s("size", "Size", 0.2, 5.0, 1.0).step(0.05).curve(1.8).sec("Dust"),
        ParamDef::section_switch("hairsOn", "Hairs", true),
        s("hairs", "Hairs", 0.0, 40.0, 5.0).step(1.0).sec("Hairs"),
        ParamDef::section_switch("scratchesOn", "Scratches", true),
        s("scratches", "Scratches", 0.0, 40.0, 3.0).step(1.0).sec("Scratches"),
        s("scratchLen", "Scratch Length", 5.0, 100.0, 55.0).step(1.0).unit("%").sec("Scratches"),
        s("opacity", "Opacity", 0.0, 100.0, 80.0).step(1.0).unit("%").sec("Look"),
        s("softness", "Softness", 0.0, 3.0, 0.5).step(0.05).sec("Look"),
        ParamDef::seed("seed").sec("Look"),
    ])
    .looks(vec![
        look("scan", "Dusty Negative Scan", vals!["style" => "light", "amount" => 45.0, "hairs" => 8.0, "scratches" => 2.0]),
        look("print", "Old Print", vals!["style" => "dark", "amount" => 30.0, "hairs" => 4.0, "scratches" => 5.0, "opacity" => 65.0]),
        look("cinema", "Projected Film Reel", vals!["style" => "mixed", "amount" => 25.0, "hairs" => 3.0, "scratches" => 14.0, "scratchLen" => 100.0, "size" => 1.4]),
        look("subtle", "Subtle Specks", vals!["style" => "light", "amount" => 15.0, "hairs" => 1.0, "scratches" => 0.0, "opacity" => 60.0]),
    ])
}

pub fn light_leak() -> Effect {
    Effect::new("leak", "Light Leak", Category::Film, |input, p, ctx| {
        let edge = match p.choice_or("edge", "right").as_str() {
            "left" => 0.0, "right" => 1.0, "top" => 2.0, "bottom" => 3.0, "tl" => 4.0, "br" => 5.0, "random" => 6.0, _ => 1.0,
        };
        let blend = match p.choice_or("blend", "screen").as_str() { "add" => 1.0, "lighten" => 2.0, _ => 0.0 };
        let mut u = vec![p.fd("seed", 1.0) * 2.7, ctx.scale as f32, edge, p.fd("size", 45.0) / 100.0, p.fd("intensity", 90.0) / 100.0];
        u.extend(p.color_or("colorA", hex("#FF4A1C")).floats());
        u.extend(p.color_or("colorB", hex("#FFC23A")).floats());
        u.extend([p.fd("warp", 45.0) / 100.0 * 0.6, blend]);
        ctx.apply("k_leak", &[input], &u)
    })
    .symbol("sun.haze")
    .summary("Warm film-camera light leaks bleeding in from an edge.")
    .params(vec![
        ParamDef::choice("edge", "From", &[("left", "Left Edge"), ("right", "Right Edge"), ("top", "Top Edge"), ("bottom", "Bottom Edge"), ("tl", "Top-Left Corner"), ("br", "Bottom-Right Corner"), ("random", "Random Spot")], "right"),
        s("size", "Size", 5.0, 100.0, 45.0).step(1.0).unit("%"),
        s("intensity", "Intensity", 0.0, 200.0, 90.0).step(1.0).unit("%"),
        s("warp", "Irregularity", 0.0, 100.0, 45.0).step(1.0).unit("%"),
        ParamDef::color("colorA", "Color A", hex("#FF4A1C")),
        ParamDef::color("colorB", "Color B", hex("#FFC23A")),
        ParamDef::choice("blend", "Blend", &[("screen", "Screen"), ("add", "Add"), ("lighten", "Lighten")], "screen"),
        ParamDef::seed("seed"),
    ])
    .looks(vec![
        look("amber", "Amber Leak", vals!["colorA" => hex("#FF4A1C"), "colorB" => hex("#FFC23A")]),
        look("magenta", "Magenta Burn", vals!["colorA" => hex("#FF1E6E"), "colorB" => hex("#FF8A3D"), "edge" => "left"]),
        look("red", "Red Flare", vals!["colorA" => hex("#E0100F"), "colorB" => hex("#FF5A1A"), "edge" => "tl", "size" => 45.0]),
        look("cool", "Cool Haze", vals!["colorA" => hex("#3AA0FF"), "colorB" => hex("#C6E8FF"), "edge" => "top", "intensity" => 70.0]),
        look("end", "End of Roll", vals!["colorA" => hex("#FF3A12"), "colorB" => hex("#FFE27A"), "edge" => "right", "size" => 70.0, "intensity" => 160.0, "warp" => 70.0]),
    ])
}
