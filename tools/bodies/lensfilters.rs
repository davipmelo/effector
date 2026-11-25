//== header
/// Downscale factor and bright-pass of the input.
fn brights(input: &T, ctx: &mut Ctx, threshold: f32, knee: f32, saturation: f32, factor: usize) -> T {
    let small = ctx.resize(input, (input.width() / factor).max(1), (input.height() / factor).max(1), false);
    ctx.apply("k_bright", &[&small], &[threshold, knee, 1.0, 1.0, 1.0, 0.0, saturation])
}

/// Suppresses big bright regions so only point-like lights produce streaks.
fn points_only(b: &T, ctx: &mut Ctx, amount: f32) -> T {
    if amount <= 0.001 {
        return b.clone();
    }
    let area = ctx.blur(b, b.width().max(b.height()) as f64 * 0.012);
    ctx.apply("k_points", &[b, &area], &[amount])
}

/// Exponential streak along `angle` (degrees), `length` in working-texture pixels.
fn streak(t: &T, ctx: &mut Ctx, angle: f64, length: f64) -> T {
    let a = angle * std::f64::consts::PI / 180.0;
    let (dx, dy) = (a.cos() as f32, (-a.sin()) as f32);
    let mut out = t.clone();
    let mut step: f32 = 1.0;
    let l = length.max(1.0) as f32;
    while step <= l * 1.2 {
        out = ctx.apply("k_streak", &[&out], &[dx, dy, step, l, 0.0]);
        step *= 9.0;
    }
    // a final averaging pass smooths the tap pattern into a clean taper
    ctx.apply("k_streak", &[&out], &[dx, dy, (l / 14.0).max(1.0), l * 0.6, 1.0])
}

fn composite(input: &T, glow: &T, ctx: &mut Ctx, intensity: f32, tint: Rgba, add: bool) -> T {
    let mut u = vec![intensity, if add { 1.0 } else { 0.0 }, 1.0, 0.0, 0.0];
    u.extend(tint.floats());
    u.push(0.0);
    ctx.apply("k_addglow", &[input, glow, glow, glow], &u)
}
//== anamorphic
        let f = 4;
        let mix = p.fd("colorMix", 85.0) / 100.0;
        let mut b = brights(input, ctx, p.fd("threshold", 72.0) / 100.0, 0.15, 1.0 - mix, f);
        b = points_only(&b, ctx, p.fd("smallLights", 60.0) / 100.0);
        let th = ctx.px(p.num("thickness", 1.5)) / f as f64;
        if th > 0.3 {
            b = ctx.blur(&b, th);
        }
        let l = p.num("length", 60.0) / 100.0 * b.width() as f64 * 0.35;
        let ang = p.n("angle");
        let mut s = if p.bool("spectral") {
            let r = streak(&b, ctx, ang, l * 1.15);
            let g = streak(&b, ctx, ang, l);
            let bl = streak(&b, ctx, ang, l * 0.85);
            ctx.apply("k_rgbsplit", &[&r, &g, &bl], &[])
        } else {
            streak(&b, ctx, ang, l)
        };
        let gw = if p.bool_or("glowOn", true) { p.fd("glow", 25.0) / 100.0 } else { 0.0 };
        if gw > 0.001 {
            let bb = ctx.blur(&b, b.width() as f64 * 0.02);
            s = ctx.apply("k_addtex", &[&s, &bb], &[gw * 2.0]);
        }
        let c = p.color_or("color", hex("#3D7CFF"));
        let m = mix as f64;
        let tint = Rgba::new(1.0 + (c.r * 1.6 - 1.0) * m, 1.0 + (c.g * 1.6 - 1.0) * m, 1.0 + (c.b * 1.6 - 1.0) * m);
        composite(input, &s, ctx, p.fd("intensity", 120.0) / 100.0 * 0.8, tint, true)
//== star_filter -> star
        let f = 4;
        let mut b = brights(input, ctx, p.fd("threshold", 86.0) / 100.0, 0.08, 1.0, f);
        b = points_only(&b, ctx, p.fd("smallLights", 85.0) / 100.0);
        let th = ctx.px(p.num("thickness", 0.6)) / f as f64;
        if th > 0.3 {
            b = ctx.blur(&b, th);
        }
        let n: usize = p.choice_or("points", "4").parse().unwrap_or(4);
        let l = p.num("length", 14.0) / 100.0 * b.width().max(b.height()) as f64 * 0.25;
        let mut acc: Option<T> = None;
        for i in 0..(n / 2) {
            let ang = p.num("angle", 45.0) + i as f64 * 180.0 / (n / 2) as f64;
            let s = if p.bool("spectral") {
                let r = streak(&b, ctx, ang, l * 1.2);
                let g = streak(&b, ctx, ang, l);
                let bl = streak(&b, ctx, ang, l * 0.8);
                ctx.apply("k_rgbsplit", &[&r, &g, &bl], &[])
            } else {
                streak(&b, ctx, ang, l)
            };
            acc = Some(match acc {
                Some(a) => ctx.apply("k_addtex", &[&a, &s], &[1.0]),
                None => s,
            });
        }
        let Some(glow) = acc else { return input.clone() };
        composite(input, &glow, ctx, p.fd("intensity", 110.0) / 100.0 * 0.8, p.color_or("tint", Rgba::WHITE), true)
//== prism
        let soft = ctx.blur(input, ctx.px(p.num("softness", 3.0)));
        let amt = p.fd("intensity", 70.0) / 100.0;
        let disp = p.fd("dispersion", 35.0) / 100.0;
        let deg = std::f32::consts::PI / 180.0;
        match p.choice_or("type", "ghosts").as_str() {
            "kaleido" => ctx.apply("k_kaleido", &[input, &soft], &[
                p.fd("segments", 6.0), p.f("rotation") * deg, p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0,
                p.fd("zoom", 100.0) / 100.0, p.f("twist") * deg, if p.bool_or("mirror", true) { 1.0 } else { 0.0 },
                disp, amt, p.f("rings"), p.fd("sx", 50.0) / 100.0, p.fd("sy", 50.0) / 100.0,
            ]),
            "facets" => ctx.apply("k_facets", &[input, &soft], &[
                ctx.scale as f32, p.fd("cell", 90.0), ctx.px(p.num("refract", 40.0)) as f32,
                disp, p.fd("edges", 35.0) / 100.0, p.fd("seed", 1.0) * 3.7, amt, p.f("stretch") / 100.0,
            ]),
            _ => {
                let mode = match p.choice_or("mode", "side").as_str() { "ring" => 1.0, "full" => 2.0, _ => 0.0 };
                let blend = match p.choice_or("blend", "screen").as_str() { "lighten" => 1.0, "normal" => 2.0, _ => 0.0 };
                ctx.apply("k_prism", &[input, &soft], &[
                    mode, p.f("angle"), ctx.px(p.num("offset", 40.0)) as f32, disp * 0.6,
                    amt, p.fd("coverage", 40.0) / 100.0, p.fd("feather", 15.0) / 100.0, p.fd("copies", 2.0), blend,
                ])
            }
        }
//== halo
        let r = ctx.px(p.num("radius", 60.0)).max(2.0);
        let f = ((r / 8.0) as usize).max(1);
        let b = brights(input, ctx, p.fd("threshold", 75.0) / 100.0, 0.1, 1.0, f);
        let sr = r / f as f64 / 2.2;
        let inner = 1.0 - p.num("width", 45.0) / 100.0;
        let mut ring = |ctx: &mut Ctx, k: f64| -> T {
            let a = ctx.blur(&b, sr * k);
            let c = ctx.blur(&b, sr * k * inner.max(0.05));
            ctx.apply("k_ring", &[&a, &c], &[0.95])
        };
        let glow = if p.bool_or("spectral", true) {
            let a = ring(ctx, 1.12);
            let bb = ring(ctx, 1.0);
            let c = ring(ctx, 0.88);
            ctx.apply("k_rgbsplit", &[&a, &bb, &c], &[])
        } else {
            ring(ctx, 1.0)
        };
        composite(input, &glow, ctx, p.fd("intensity", 160.0) / 100.0, p.color_or("tint", Rgba::WHITE), false)
//== lens_blur
        let amt = ctx.px(p.num("amount", 18.0));
        let boost = p.fd("bokeh", 40.0) / 100.0;
        let pre = ctx.apply("k_hlboost", &[input], &[boost]);
        let b1 = ctx.blur(&pre, (amt * 0.35).max(0.3));
        let b2 = ctx.blur(&pre, amt.max(0.3));
        let mode = match p.choice_or("mode", "band").as_str() { "radial" => 1.0, "full" => 2.0, _ => 0.0 };
        let mixed = ctx.apply("k_focusmix", &[input, &b1, &b2], &[
            mode, p.fd("x", 50.0) / 100.0, p.fd("y", 55.0) / 100.0, p.fd("size", 20.0) / 100.0,
            p.fd("falloff", 25.0) / 100.0, p.f("angle"), p.fd("saturation", 100.0) / 100.0,
        ]);
        ctx.apply("k_clamp", &[&mixed], &[])
