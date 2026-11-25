//== fisheye
        let k = (p.fd("amount", 45.0) / 100.0 * 0.9) as f64;
        let asp = input.width() as f64 / input.height() as f64;
        let true_fish = p.choice_or("projection", "barrel") == "fisheye";
        let f = |r: f64| 1.0 + k * r * r + k * 0.5 * r * r * r * r;
        let mut auto = 1.0;
        if !true_fish && p.bool_or("autoZoom", true) {
            if p.bool_or("circle", false) {
                auto = f(0.5 * asp.min(1.0));
            } else if k > 0.0 {
                auto = f((asp * asp / 4.0 + 0.25).sqrt());
            }
        }
        let zoom = (p.num("zoom", 100.0) / 100.0 * auto) as f32;
        let fringe = if p.bool_or("fringeOn", true) { p.fd("fringe", 10.0) / 100.0 * 0.08 } else { 0.0 };
        let out = ctx.apply("k_fisheye", &[input], &[k as f32, zoom, p.b("circle"), fringe, p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0,
            if true_fish { 1.0 } else { 0.0 }, p.fd("fov", 180.0) * std::f32::consts::PI / 180.0,
            if p.choice_or("edge", "mirror") == "mirror" { 1.0 } else { 0.0 }]);
        if !p.bool_or("edgeBlurOn", false) {
            return out;
        }
        let b = ctx.blur(&out, ctx.px(p.num("edgeBlurRadius", 14.0)));
        ctx.apply("k_maskmix", &[&out, &b], &[1.0, p.fd("edgeBlurCenter", 40.0) / 100.0, 0.5, 0.0, p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0,
            p.fd("edgeBlur", 70.0) / 100.0])
//== edge_distort
        let mode = match p.choice_or("mode", "bend").as_str() { "stretch" => 1.0, "swirl" => 2.0, "ripple" => 3.0, "liquid" => 4.0, "squeeze" => 5.0, _ => 0.0 };
        let blur_on = p.bool_or("blurOn", false);
        let blurred = if blur_on { ctx.blur(input, ctx.px(p.num("blurRadius", 12.0))) } else { input.clone() };
        let u = [
            mode, p.fd("strength", 50.0) / 100.0, p.fd("protect", 35.0) / 100.0, p.fd("softness", 45.0) / 100.0,
            if p.choice_or("shape", "round") == "frame" { 1.0 } else { 0.0 }, p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0,
            if p.bool_or("fringeOn", true) { p.fd("fringe", 30.0) / 100.0 * 0.08 } else { 0.0 }, p.fd("seed", 1.0) * 3.1, ctx.scale as f32,
            p.fd("scale", 60.0), if blur_on { p.fd("blur", 50.0) / 100.0 } else { 0.0 },
            if p.bool_or("darkOn", false) { p.fd("darken", 35.0) / 100.0 } else { 0.0 },
        ];
        ctx.apply("k_edgewarp", &[input, &blurred], &u)
//== mirror
        let mode = match p.choice_or("mode", "lr").as_str() { "rl" => 1.0, "tb" => 2.0, "bt" => 3.0, "quad" => 4.0, "tilesm" => 5.0, "tiles" => 6.0, _ => 0.0 };
        ctx.apply("k_mirror", &[input], &[mode, p.fd("tiles", 3.0), p.fd("position", 50.0) / 100.0])
//== stretch
        let dir = match p.choice_or("dir", "right").as_str() { "left" => 1.0, "down" => 2.0, "up" => 3.0, _ => 0.0 };
        ctx.apply("k_stretch", &[input], &[dir, p.fd("position", 60.0) / 100.0, p.fd("jitter", 15.0) / 100.0, p.fd("seed", 1.0) * 2.9,
            ctx.px(p.num("lane", 3.0)) as f32])
