//== header
use crate::overlay::OverlayCanvas;
use crate::swatches::SplitMix;
//== dust
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
//== light_leak
        let edge = match p.choice_or("edge", "right").as_str() {
            "left" => 0.0, "right" => 1.0, "top" => 2.0, "bottom" => 3.0, "tl" => 4.0, "br" => 5.0, "random" => 6.0, _ => 1.0,
        };
        let blend = match p.choice_or("blend", "screen").as_str() { "add" => 1.0, "lighten" => 2.0, _ => 0.0 };
        let mut u = vec![p.fd("seed", 1.0) * 2.7, ctx.scale as f32, edge, p.fd("size", 45.0) / 100.0, p.fd("intensity", 90.0) / 100.0];
        u.extend(p.color_or("colorA", hex("#FF4A1C")).floats());
        u.extend(p.color_or("colorB", hex("#FFC23A")).floats());
        u.extend([p.fd("warp", 45.0) / 100.0 * 0.6, blend]);
        ctx.apply("k_leak", &[input], &u)
