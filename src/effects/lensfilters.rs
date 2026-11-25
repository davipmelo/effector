use super::*;
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

pub fn anamorphic() -> Effect {
    Effect::new("anamorphic", "Anamorphic Flare", Category::Lens, |input, p, ctx| {
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
    })
    .symbol("line.3.horizontal")
    .summary("Long horizontal streaks from bright lights, like an anamorphic cinema lens.")
    .params(vec![
        s("intensity", "Intensity", 0.0, 300.0, 120.0).step(1.0).unit("%").sec("Streak"),
        s("length", "Length", 5.0, 150.0, 60.0).step(1.0).unit("%").sec("Streak"),
        s("threshold", "Threshold", 0.0, 100.0, 72.0).step(1.0).unit("%").sec("Streak"),
        s("thickness", "Thickness", 0.0, 10.0, 1.5).step(0.1).sec("Streak"),
        s("angle", "Angle", -45.0, 45.0, 0.0).step(0.5).unit("°").sec("Streak"),
        ParamDef::color("color", "Color", hex("#3D7CFF")).sec("Color"),
        s("colorMix", "Color Amount", 0.0, 100.0, 85.0).step(1.0).unit("%").sec("Color"),
        ParamDef::toggle("spectral", "Spectral Edges", false).sec("Color"),
        s("smallLights", "Small Lights Only", 0.0, 100.0, 60.0).step(1.0).unit("%").sec("Streak"),
        ParamDef::section_switch("glowOn", "Glow", true),
        s("glow", "Wide Glow", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Glow"),
    ])
    .looks(vec![
        look("blue", "Classic Blue Streak", vals![]),
        look("amber", "Amber Streak", vals!["color" => hex("#FF9A3D")]),
        look("clean", "Clean White", vals!["colorMix" => 0.0, "intensity" => 90.0]),
        look("scifi", "Sci-Fi", vals!["intensity" => 220.0, "length" => 120.0, "threshold" => 60.0, "spectral" => true]),
        look("subtle", "Subtle", vals!["intensity" => 60.0, "length" => 35.0, "glow" => 10.0]),
    ])
}

pub fn star() -> Effect {
    Effect::new("star", "Star Filter", Category::Lens, |input, p, ctx| {
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
    })
    .symbol("sparkle")
    .summary("Cross-screen filter: 4, 6 or 8-point stars on every highlight.")
    .params(vec![
        ParamDef::segmented("points", "Points", &[("4", "4"), ("6", "6"), ("8", "8")], "4").sec("Stars"),
        s("intensity", "Intensity", 0.0, 300.0, 110.0).step(1.0).unit("%").sec("Stars"),
        s("length", "Length", 2.0, 100.0, 14.0).step(1.0).unit("%").curve(1.4).sec("Stars"),
        s("threshold", "Threshold", 0.0, 100.0, 86.0).step(1.0).unit("%").sec("Stars"),
        s("angle", "Rotation", 0.0, 90.0, 45.0).step(0.5).unit("°").sec("Stars"),
        s("thickness", "Thickness", 0.0, 6.0, 0.6).step(0.1).sec("Stars"),
        s("smallLights", "Small Lights Only", 0.0, 100.0, 85.0).step(1.0).unit("%").sec("Stars"),
        ParamDef::toggle("spectral", "Rainbow", false).sec("Color"),
        ParamDef::color("tint", "Tint", Rgba::WHITE).sec("Color"),
    ])
}

pub fn prism() -> Effect {
    Effect::new("prism", "Prism & Kaleidoscope", Category::Lens, |input, p, ctx| {
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
    })
    .symbol("triangle.lefthalf.filled")
    .summary("Prism ghosts, a full kaleidoscope or crystal facets — refract, mirror and spin the image.")
    .params(vec![
        ParamDef::segmented("type", "Type", &[("ghosts", "Prism"), ("kaleido", "Kaleidoscope"), ("facets", "Crystal")], "ghosts").sec("Prism"),
        // prism ghosts
        ParamDef::choice("mode", "Placement", &[("side", "One Side"), ("ring", "Around the Edges"), ("full", "Whole Frame")], "side")
            .sec("Prism")
            .vis(when("type", "ghosts")),
        s("angle", "Direction", -180.0, 180.0, 0.0).step(1.0).unit("°").sec("Prism").vis(when("type", "ghosts")),
        s("coverage", "Coverage", 5.0, 100.0, 40.0).step(1.0).unit("%").sec("Prism").vis(Arc::new(|p: &Params| p.choice_or("type", "ghosts") == "ghosts" && p.choice_or("mode", "side") != "full")),
        s("feather", "Feather", 0.0, 50.0, 15.0).step(1.0).unit("%").sec("Prism").vis(Arc::new(|p: &Params| p.choice_or("type", "ghosts") == "ghosts" && p.choice_or("mode", "side") != "full")),
        s("offset", "Offset", 0.0, 200.0, 40.0).step(1.0).curve(1.5).sec("Prism").vis(when("type", "ghosts")),
        s("copies", "Copies", 1.0, 4.0, 2.0).step(1.0).sec("Prism").vis(when("type", "ghosts")),
        ParamDef::choice("blend", "Blend", &[("screen", "Screen"), ("lighten", "Lighten"), ("normal", "Normal")], "screen").sec("Prism").vis(when("type", "ghosts")),
        // kaleidoscope
        s("segments", "Segments", 2.0, 32.0, 6.0).step(1.0).sec("Kaleidoscope").vis(when("type", "kaleido")),
        ParamDef::toggle("mirror", "Mirror Segments", true).sec("Kaleidoscope").vis(when("type", "kaleido")),
        s("rotation", "Rotation", -180.0, 180.0, 0.0).step(1.0).unit("°").sec("Kaleidoscope").vis(when("type", "kaleido")),
        s("zoom", "Zoom", 10.0, 400.0, 100.0).step(1.0).unit("%").curve(1.5).sec("Kaleidoscope").vis(when("type", "kaleido")),
        s("twist", "Twist", -400.0, 400.0, 0.0).step(1.0).unit("°").sec("Kaleidoscope").vis(when("type", "kaleido")),
        s("rings", "Rings", 0.0, 12.0, 0.0).step(1.0).sec("Kaleidoscope").vis(when("type", "kaleido")),
        s("cx", "Center X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Kaleidoscope").vis(when("type", "kaleido")),
        s("cy", "Center Y", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Kaleidoscope").vis(when("type", "kaleido")),
        s("sx", "Source X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Kaleidoscope").vis(when("type", "kaleido")),
        s("sy", "Source Y", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Kaleidoscope").vis(when("type", "kaleido")),
        // facets
        s("cell", "Facet Size", 10.0, 400.0, 90.0).step(1.0).curve(1.6).sec("Crystal").vis(when("type", "facets")),
        s("refract", "Refraction", 0.0, 200.0, 40.0).step(1.0).curve(1.4).sec("Crystal").vis(when("type", "facets")),
        s("edges", "Edge Shine", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Crystal").vis(when("type", "facets")),
        s("stretch", "Stretch", -90.0, 90.0, 0.0).step(1.0).unit("%").sec("Crystal").vis(when("type", "facets")),
        ParamDef::seed("seed").sec("Crystal"),
        // shared
        s("dispersion", "Rainbow Dispersion", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Color & Mix"),
        s("softness", "Softness", 0.0, 30.0, 3.0).step(0.5).sec("Color & Mix"),
        s("intensity", "Amount", 0.0, 150.0, 70.0).step(1.0).unit("%").sec("Color & Mix"),
    ])
    .looks(vec![
        look_g("side", "Rainbow Side", "Prism", vals![]),
        look_g("kaleido", "Edge Ghosts", "Prism", vals!["mode" => "ring", "coverage" => 45.0, "copies" => 3.0, "offset" => 60.0]),
        look_g("double", "Double Exposure", "Prism", vals!["mode" => "full", "offset" => 120.0, "copies" => 1.0, "dispersion" => 15.0, "intensity" => 50.0, "blend" => "lighten"]),
        look_g("dreamy", "Dreamy Refraction", "Prism", vals!["softness" => 14.0, "dispersion" => 70.0, "offset" => 25.0, "copies" => 3.0]),
        look_g("k6", "Classic Kaleidoscope", "Kaleidoscope", vals!["type" => "kaleido", "segments" => 6.0, "intensity" => 100.0, "dispersion" => 10.0]),
        look_g("mandala", "Mandala", "Kaleidoscope", vals!["type" => "kaleido", "segments" => 12.0, "rings" => 3.0, "intensity" => 100.0, "zoom" => 140.0, "dispersion" => 25.0]),
        look_g("vortex", "Vortex", "Kaleidoscope", vals!["type" => "kaleido", "segments" => 8.0, "twist" => 220.0, "intensity" => 100.0, "dispersion" => 40.0]),
        look_g("mirror2", "Mirror World", "Kaleidoscope", vals!["type" => "kaleido", "segments" => 2.0, "intensity" => 100.0, "dispersion" => 0.0]),
        look_g("trip", "Psychedelic Trip", "Kaleidoscope", vals!["type" => "kaleido", "segments" => 16.0, "twist" => -160.0, "rings" => 5.0, "zoom" => 70.0, "dispersion" => 90.0, "intensity" => 100.0]),
        look_g("crystal", "Crystal Lens", "Crystal", vals!["type" => "facets", "intensity" => 100.0]),
        look_g("shards", "Glass Shards", "Crystal", vals!["type" => "facets", "cell" => 160.0, "refract" => 90.0, "edges" => 60.0, "stretch" => 60.0, "intensity" => 100.0]),
        look_g("mosaic", "Fine Mosaic", "Crystal", vals!["type" => "facets", "cell" => 25.0, "refract" => 15.0, "edges" => 20.0, "intensity" => 100.0, "dispersion" => 10.0]),
    ])
}

pub fn halo() -> Effect {
    Effect::new("halo", "Halo", Category::Lens, |input, p, ctx| {
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
    })
    .symbol("circle.circle")
    .summary("Rings of light around bright sources, optionally with rainbow edges.")
    .params(vec![
        s("intensity", "Intensity", 0.0, 300.0, 160.0).step(1.0).unit("%"),
        s("radius", "Radius", 5.0, 300.0, 60.0).step(1.0).curve(1.6),
        s("width", "Ring Width", 5.0, 95.0, 45.0).step(1.0).unit("%"),
        s("threshold", "Threshold", 0.0, 100.0, 75.0).step(1.0).unit("%"),
        ParamDef::toggle("spectral", "Rainbow", true),
        ParamDef::color("tint", "Tint", Rgba::WHITE),
    ])
}

pub fn lens_blur() -> Effect {
    Effect::new("lensblur", "Lens Blur & Tilt-Shift", Category::Blur, |input, p, ctx| {
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
    })
    .symbol("camera.metering.center.weighted")
    .summary("Shallow focus: a sharp band (tilt-shift miniature) or a sharp centre with bright bokeh.")
    .params(vec![
        ParamDef::segmented("mode", "Focus", &[("band", "Band"), ("radial", "Radial"), ("full", "Whole")], "band").sec("Focus"),
        s("x", "Focus X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Focus").vis(when("mode", "radial")),
        s("y", "Focus Y", 0.0, 100.0, 55.0).step(1.0).unit("%").sec("Focus").vis(when_not("mode", "full")),
        s("size", "Sharp Area", 0.0, 100.0, 20.0).step(1.0).unit("%").sec("Focus").vis(when_not("mode", "full")),
        s("falloff", "Transition", 1.0, 100.0, 25.0).step(1.0).unit("%").sec("Focus").vis(when_not("mode", "full")),
        s("angle", "Angle", -90.0, 90.0, 0.0).step(1.0).unit("°").sec("Focus").vis(when("mode", "band")),
        s("amount", "Blur", 0.0, 100.0, 18.0).step(0.5).curve(1.6).sec("Blur"),
        s("bokeh", "Bokeh Brightness", 0.0, 100.0, 40.0).step(1.0).unit("%").sec("Blur"),
        s("saturation", "Saturation", 0.0, 200.0, 100.0).step(1.0).unit("%").sec("Blur"),
    ])
    .looks(vec![
        look("mini", "Miniature (Tilt-Shift)", vals!["mode" => "band", "saturation" => 140.0, "amount" => 14.0]),
        look("portrait", "Portrait f/1.4", vals!["mode" => "radial", "size" => 30.0, "falloff" => 35.0, "amount" => 22.0, "y" => 45.0]),
        look("dream", "Dreamy Bokeh", vals!["mode" => "full", "amount" => 30.0, "bokeh" => 80.0]),
    ])
}
