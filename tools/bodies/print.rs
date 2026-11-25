//== halftone
        let cell = ctx.px(p.num("cell", 8.0)).max(2.0);
        let toned = ctx.apply("k_dither_pre", &[input], &[p.f("brightness"), p.f("contrast"), 0.0, 1.0]);
        let smoothed = ctx.blur(&toned, cell * 0.5 * p.num("smooth", 40.0) / 100.0);
        let mode = match p.choice_or("mode", "bw").as_str() { "cmyk" => 1.0, "color" => 2.0, _ => 0.0 };
        let shape = match p.choice_or("shape", "circle").as_str() {
            "square" => 1.0, "diamond" => 2.0, "line" => 3.0, "ellipse" => 4.0, "cross" => 5.0, _ => 0.0,
        };
        let mut u = vec![cell as f32, p.fd("angle", 45.0), shape, mode, p.fd("dotScale", 100.0) / 100.0, p.fd("softness", 10.0) / 100.0 * 0.5];
        u.extend(p.color_or("paper", hex("#FFFFFF")).floats());
        u.extend(p.color_or("ink", hex("#111111")).floats());
        u.extend([p.fd("angC", 15.0), p.fd("angM", 75.0), p.fd("angY", 0.0), p.fd("angK", 45.0), p.fd("gcr", 80.0) / 100.0]);
        ctx.apply("k_halftone", &[&smoothed], &u)
