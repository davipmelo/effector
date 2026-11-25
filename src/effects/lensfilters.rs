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

/// Keeps only some highlights: `selectivity` (0..1) favours the brightest, `density` (0..1) drops
/// random lights so the effect can be sprinkled subtly.
fn select(b: &T, ctx: &mut Ctx, selectivity: f32, density: f32, seed: f32) -> T {
    if selectivity <= 0.001 && density >= 0.999 {
        return b.clone();
    }
    let cell = if density < 0.999 { (b.width().max(b.height()) as f32 * 0.045).max(3.0) } else { 0.0 };
    ctx.apply("k_select", &[b], &[1.0 + selectivity * 7.0, cell, density, seed])
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
        let mut b = brights(input, ctx, p.fd("threshold", 80.0) / 100.0, p.fd("knee", 10.0) / 100.0, 1.0 - mix, f);
        b = points_only(&b, ctx, p.fd("smallLights", 70.0) / 100.0);
        b = select(&b, ctx, p.fd("selectivity", 30.0) / 100.0, p.fd("density", 100.0) / 100.0, p.fd("seed", 1.0));
        let th = ctx.px(p.num("thickness", 1.0)) / f as f64;
        if th > 0.3 {
            b = ctx.blur(&b, th);
        }
        let l = p.num("length", 45.0) / 100.0 * b.width() as f64 * 0.35;
        let ang = p.n("angle");
        let mut s = if p.bool("spectral") {
            let r = streak(&b, ctx, ang, l * 1.15);
            let g = streak(&b, ctx, ang, l);
            let bl = streak(&b, ctx, ang, l * 0.85);
            ctx.apply("k_rgbsplit", &[&r, &g, &bl], &[])
        } else {
            streak(&b, ctx, ang, l)
        };
        let gw = if p.bool_or("glowOn", true) { p.fd("glow", 15.0) / 100.0 } else { 0.0 };
        if gw > 0.001 {
            let bb = ctx.blur(&b, b.width() as f64 * 0.02);
            s = ctx.apply("k_addtex", &[&s, &bb], &[gw * 2.0]);
        }
        let core = p.fd("core", 0.0) / 100.0;
        if core > 0.001 {
            let cb = ctx.blur(&b, b.width() as f64 * 0.004);
            s = ctx.apply("k_addtex", &[&s, &cb], &[core * 3.0]);
        }
        let c = p.color_or("color", hex("#3D7CFF"));
        let m = mix as f64;
        let tint = Rgba::new(1.0 + (c.r * 1.6 - 1.0) * m, 1.0 + (c.g * 1.6 - 1.0) * m, 1.0 + (c.b * 1.6 - 1.0) * m);
        composite(input, &s, ctx, p.fd("intensity", 70.0) / 100.0 * 0.8, tint, true)
    })
    .symbol("line.3.horizontal")
    .summary("Long horizontal streaks from bright lights, like an anamorphic cinema lens.")
    .params(vec![
        s("intensity", "Intensity", 0.0, 300.0, 70.0).step(1.0).unit("%").curve(1.8).sec("Streak"),
        s("length", "Length", 2.0, 150.0, 45.0).step(1.0).unit("%").curve(1.4).sec("Streak"),
        s("thickness", "Thickness", 0.0, 10.0, 1.0).step(0.1).curve(1.5).sec("Streak"),
        s("angle", "Angle", -45.0, 45.0, 0.0).step(0.5).unit("°").sec("Streak"),
        s("threshold", "Threshold", 0.0, 100.0, 80.0).step(1.0).unit("%").sec("Which Lights"),
        s("knee", "Threshold Softness", 0.0, 40.0, 10.0).step(1.0).unit("%").sec("Which Lights"),
        s("selectivity", "Brightest Only", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Which Lights"),
        s("smallLights", "Small Lights Only", 0.0, 100.0, 70.0).step(1.0).unit("%").sec("Which Lights"),
        s("density", "Density", 0.0, 100.0, 100.0).step(1.0).unit("%").sec("Which Lights"),
        ParamDef::seed("seed").sec("Which Lights").vis(Arc::new(|p: &Params| p.num("density", 100.0) < 99.5)),
        ParamDef::color("color", "Color", hex("#3D7CFF")).sec("Color"),
        s("colorMix", "Color Amount", 0.0, 100.0, 85.0).step(1.0).unit("%").sec("Color"),
        ParamDef::toggle("spectral", "Spectral Edges", false).sec("Color"),
        ParamDef::section_switch("glowOn", "Glow", true),
        s("glow", "Wide Glow", 0.0, 100.0, 15.0).step(1.0).unit("%").sec("Glow"),
        s("core", "Hot Core", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Glow"),
    ])
    .depth_bias()
    .looks(vec![
        look("blue", "Classic Blue Streak", vals![]),
        look("subtle", "Subtle Kiss", vals!["intensity" => 30.0, "length" => 30.0, "glow" => 6.0, "selectivity" => 55.0, "threshold" => 86.0]),
        look("few", "Few Lights Only", vals!["intensity" => 60.0, "density" => 35.0, "selectivity" => 60.0]),
        look("amber", "Amber Streak", vals!["color" => hex("#FF9A3D")]),
        look("clean", "Clean White", vals!["colorMix" => 0.0, "intensity" => 55.0]),
        look("scifi", "Sci-Fi", vals!["intensity" => 200.0, "length" => 120.0, "threshold" => 60.0, "spectral" => true, "selectivity" => 0.0, "core" => 40.0]),
        look("night", "Night Street", vals!["intensity" => 90.0, "length" => 70.0, "threshold" => 74.0, "thickness" => 0.6, "smallLights" => 90.0]),
    ])
}

pub fn star() -> Effect {
    Effect::new("star", "Star Filter", Category::Lens, |input, p, ctx| {
        let f = 4;
        let mut b = brights(input, ctx, p.fd("threshold", 86.0) / 100.0, p.fd("knee", 6.0) / 100.0, 1.0, f);
        b = points_only(&b, ctx, p.fd("smallLights", 90.0) / 100.0);
        b = select(&b, ctx, p.fd("selectivity", 35.0) / 100.0, p.fd("density", 100.0) / 100.0, p.fd("seed", 1.0));
        let th = ctx.px(p.num("thickness", 0.5)) / f as f64;
        if th > 0.3 {
            b = ctx.blur(&b, th);
        }
        let kind = p.choice_or("points", "4");
        let l = p.num("length", 10.0) / 100.0 * b.width().max(b.height()) as f64 * 0.25;
        let base = p.num("angle", 45.0);
        // (angle, length factor, weight)
        let rays: Vec<(f64, f64, f32)> = if kind == "scratch" {
            // scratch / "SFX" filter: many short streaks at random angles
            let n = p.num("rays", 12.0).round().max(3.0) as usize;
            let mut rng = crate::swatches::SplitMix::new(p.num("seed", 1.0) as u64 * 7919 + 3);
            (0..n)
                .map(|_| {
                    let a = (rng.next() % 3600) as f64 / 10.0;
                    let lf = 0.35 + (rng.next() % 650) as f64 / 1000.0;
                    (a, lf, 1.6 / (n as f32).sqrt())
                })
                .collect()
        } else {
            let n: usize = kind.parse().unwrap_or(4);
            let var = p.num("variation", 0.0) / 100.0;
            (0..(n / 2)).map(|i| (base + i as f64 * 180.0 / (n / 2) as f64, if i % 2 == 1 { 1.0 - var * 0.6 } else { 1.0 }, 1.0)).collect()
        };
        let mut acc: Option<T> = None;
        for (ang, lf, w) in rays {
            let ll = l * lf;
            let s = if p.bool("spectral") {
                let r = streak(&b, ctx, ang, ll * 1.2);
                let g = streak(&b, ctx, ang, ll);
                let bl = streak(&b, ctx, ang, ll * 0.8);
                ctx.apply("k_rgbsplit", &[&r, &g, &bl], &[])
            } else {
                streak(&b, ctx, ang, ll)
            };
            acc = Some(match acc {
                Some(a) => ctx.apply("k_addtex", &[&a, &s], &[w]),
                None if w != 1.0 => ctx.apply("k_addtex", &[&s, &s], &[w - 1.0]),
                None => s,
            });
        }
        let Some(mut glow) = acc else { return input.clone() };
        let core = p.fd("core", 20.0) / 100.0;
        if core > 0.001 {
            let cb = ctx.blur(&b, b.width().max(b.height()) as f64 * 0.005);
            glow = ctx.apply("k_addtex", &[&glow, &cb], &[core * 2.5]);
        }
        composite(input, &glow, ctx, p.fd("intensity", 60.0) / 100.0 * 0.8, p.color_or("tint", Rgba::WHITE), true)
    })
    .symbol("sparkle")
    .summary("Cross-screen filter: 4, 6 or 8-point stars, or a scratch filter, on the brightest highlights.")
    .params(vec![
        ParamDef::segmented("points", "Points", &[("4", "4"), ("6", "6"), ("8", "8"), ("scratch", "Scratch")], "4").sec("Stars"),
        s("rays", "Rays", 3.0, 32.0, 12.0).step(1.0).sec("Stars").vis(when("points", "scratch")),
        s("intensity", "Intensity", 0.0, 300.0, 60.0).step(1.0).unit("%").curve(1.8).sec("Stars"),
        s("length", "Length", 1.0, 100.0, 10.0).step(0.5).unit("%").curve(1.8).sec("Stars"),
        s("variation", "Alternate Length", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Stars").vis(when_not("points", "scratch")),
        s("angle", "Rotation", 0.0, 90.0, 45.0).step(0.5).unit("°").sec("Stars").vis(when_not("points", "scratch")),
        s("thickness", "Thickness", 0.0, 6.0, 0.5).step(0.1).curve(1.5).sec("Stars"),
        s("core", "Hot Core", 0.0, 100.0, 20.0).step(1.0).unit("%").sec("Stars"),
        s("threshold", "Threshold", 0.0, 100.0, 86.0).step(0.5).unit("%").sec("Which Lights"),
        s("knee", "Threshold Softness", 0.0, 30.0, 6.0).step(1.0).unit("%").sec("Which Lights"),
        s("selectivity", "Brightest Only", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Which Lights"),
        s("smallLights", "Small Lights Only", 0.0, 100.0, 90.0).step(1.0).unit("%").sec("Which Lights"),
        s("density", "Density", 0.0, 100.0, 100.0).step(1.0).unit("%").sec("Which Lights"),
        ParamDef::seed("seed").sec("Which Lights"),
        ParamDef::toggle("spectral", "Rainbow", false).sec("Color"),
        ParamDef::color("tint", "Tint", Rgba::WHITE).sec("Color"),
    ])
    .depth_bias()
    .looks(vec![
        look("classic", "Classic Cross", vals![]),
        look("whisper", "Whisper", vals!["intensity" => 25.0, "length" => 6.0, "selectivity" => 70.0, "core" => 10.0]),
        look("sparse", "Sparse Twinkle", vals!["density" => 30.0, "selectivity" => 60.0, "intensity" => 70.0, "length" => 12.0]),
        look("six", "Six Point", vals!["points" => "6", "length" => 12.0]),
        look("eight", "Eight Point Sparkle", vals!["points" => "8", "variation" => 50.0, "length" => 14.0]),
        look("scratch", "Scratch Filter", vals!["points" => "scratch", "rays" => 14.0, "length" => 22.0, "intensity" => 90.0, "selectivity" => 30.0, "core" => 35.0]),
        look("rainbow", "Rainbow Burst", vals!["spectral" => true, "length" => 20.0, "intensity" => 110.0]),
        look("party", "Disco Night", vals!["points" => "8", "threshold" => 80.0, "selectivity" => 10.0, "length" => 24.0, "intensity" => 140.0, "spectral" => true]),
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
            "facets" if p.choice_or("crystalStyle", "gem") == "shards" => ctx.apply("k_facets", &[input, &soft], &[
                ctx.scale as f32, p.fd("cell", 90.0), ctx.px(p.num("refract", 40.0)) as f32,
                disp, p.fd("edges", 35.0) / 100.0, p.fd("seed", 1.0) * 3.7, amt, p.f("stretch") / 100.0,
            ]),
            "facets" => ctx.apply("k_crystal", &[input, &soft], &[
                p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0, p.fd("gemRings", 2.0), p.fd("gemFacets", 6.0),
                p.fd("clear", 22.0) / 100.0, p.fd("gemRefract", 45.0) / 100.0, p.fd("seamSoft", 6.0) / 100.0,
                disp, p.fd("edges", 25.0) / 100.0, p.f("rotation") * deg, p.fd("seed", 1.0) * 3.7, amt,
                p.fd("jitter", 20.0) / 100.0, p.fd("shade", 10.0) / 100.0,
            ]),
            "linear" => ctx.apply("k_linprism", &[input, &soft], &[
                p.f("angle"), ctx.px(p.num("spacing", 70.0)) as f32, p.fd("copies", 4.0), p.fd("decay", 30.0) / 100.0,
                if p.choice_or("linStyle", "bands") == "echo" { 1.0 } else { 0.0 }, p.fd("start", 40.0) / 100.0,
                p.fd("feather", 15.0) / 100.0, disp, amt,
                match p.choice_or("blend", "screen").as_str() { "lighten" => 1.0, "normal" => 2.0, _ => 0.0 },
            ]),
            "split" => ctx.apply("k_splitprism", &[input, &soft], &[
                p.f("angle"), p.f("position") / 100.0, ctx.px(p.num("offset", 40.0)) as f32, p.b("mirror"),
                p.fd("zoom", 100.0) / 100.0, p.fd("seam", 3.0) / 100.0, p.fd("seamRainbow", 60.0) / 100.0, disp, amt,
                p.fd("feather", 2.0) / 100.0,
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
    .summary("Prism ghosts, linear and half prisms, a kaleidoscope or a faceted crystal lens — refract, repeat and spin the image.")
    .params({
        let t = |v: &'static [&'static str]| -> Visible { when_any("type", v) };
        vec![
            ParamDef::choice("type", "Type", &[("ghosts", "Prism Ghosts"), ("linear", "Linear Prism"), ("split", "Half Prism"), ("kaleido", "Kaleidoscope"), ("facets", "Crystal")], "ghosts").sec("Prism"),
            // prism ghosts
            ParamDef::choice("mode", "Placement", &[("side", "One Side"), ("ring", "Around the Edges"), ("full", "Whole Frame")], "side").sec("Prism").vis(t(&["ghosts"])),
            ParamDef::segmented("linStyle", "Style", &[("bands", "Bands"), ("echo", "Echoes")], "bands").sec("Prism").vis(t(&["linear"])),
            s("angle", "Direction", -180.0, 180.0, 0.0).step(1.0).unit("°").sec("Prism").vis(t(&["ghosts", "linear", "split"])),
            s("coverage", "Coverage", 5.0, 100.0, 40.0).step(1.0).unit("%").sec("Prism").vis(Arc::new(|p: &Params| p.choice_or("type", "ghosts") == "ghosts" && p.choice_or("mode", "side") != "full")),
            s("start", "Starts At", 0.0, 95.0, 40.0).step(1.0).unit("%").sec("Prism").vis(Arc::new(|p: &Params| p.is("type", "linear") && p.choice_or("linStyle", "bands") == "bands")),
            s("position", "Cut Position", -100.0, 100.0, 0.0).step(1.0).unit("%").sec("Prism").vis(t(&["split"])),
            s("feather", "Feather", 0.0, 50.0, 15.0).step(1.0).unit("%").sec("Prism").vis(Arc::new(|p: &Params| match p.choice_or("type", "ghosts").as_str() {
                "ghosts" => p.choice_or("mode", "side") != "full",
                "linear" => p.choice_or("linStyle", "bands") == "bands",
                "split" => true,
                _ => false,
            })),
            s("offset", "Offset", 0.0, 300.0, 40.0).step(1.0).curve(1.5).sec("Prism").vis(t(&["ghosts", "split"])),
            s("spacing", "Spacing", 2.0, 300.0, 70.0).step(1.0).curve(1.5).sec("Prism").vis(t(&["linear"])),
            s("copies", "Copies", 1.0, 10.0, 2.0).step(1.0).sec("Prism").vis(t(&["ghosts", "linear"])),
            s("decay", "Fade per Copy", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Prism").vis(t(&["linear"])),
            ParamDef::toggle("mirror", "Mirror", true).sec("Prism").vis(t(&["kaleido", "split"])),
            s("zoom", "Zoom", 10.0, 400.0, 100.0).step(1.0).unit("%").curve(1.5).sec("Prism").vis(t(&["split", "kaleido"])),
            s("seam", "Seam Width", 0.0, 20.0, 3.0).step(0.5).unit("%").sec("Prism").vis(t(&["split"])),
            s("seamRainbow", "Seam Rainbow", 0.0, 100.0, 60.0).step(1.0).unit("%").sec("Prism").vis(t(&["split"])),
            ParamDef::choice("blend", "Blend", &[("screen", "Screen"), ("lighten", "Lighten"), ("normal", "Normal")], "screen").sec("Prism").vis(Arc::new(|p: &Params| p.choice_or("type", "ghosts") == "ghosts" || (p.is("type", "linear") && p.is("linStyle", "echo")))),
            // kaleidoscope
            s("segments", "Segments", 2.0, 32.0, 6.0).step(1.0).sec("Kaleidoscope").vis(t(&["kaleido"])),
            s("rotation", "Rotation", -180.0, 180.0, 0.0).step(1.0).unit("°").sec("Kaleidoscope").vis(t(&["kaleido", "facets"])),
            s("twist", "Twist", -400.0, 400.0, 0.0).step(1.0).unit("°").sec("Kaleidoscope").vis(t(&["kaleido"])),
            s("rings", "Rings", 0.0, 12.0, 0.0).step(1.0).sec("Kaleidoscope").vis(t(&["kaleido"])),
            s("cx", "Center X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Kaleidoscope").vis(t(&["kaleido", "facets"])),
            s("cy", "Center Y", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Kaleidoscope").vis(t(&["kaleido", "facets"])),
            s("sx", "Source X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Kaleidoscope").vis(t(&["kaleido"])),
            s("sy", "Source Y", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Kaleidoscope").vis(t(&["kaleido"])),
            // crystal
            ParamDef::segmented("crystalStyle", "Cut", &[("gem", "Faceted Lens"), ("shards", "Random Shards")], "gem").sec("Crystal").vis(t(&["facets"])),
            s("gemRings", "Rings", 1.0, 5.0, 2.0).step(1.0).sec("Crystal").vis(Arc::new(|p: &Params| p.is("type", "facets") && p.choice_or("crystalStyle", "gem") == "gem")),
            s("gemFacets", "Facets", 3.0, 16.0, 6.0).step(1.0).sec("Crystal").vis(Arc::new(|p: &Params| p.is("type", "facets") && p.choice_or("crystalStyle", "gem") == "gem")),
            s("clear", "Clear Center", 0.0, 80.0, 22.0).step(1.0).unit("%").sec("Crystal").vis(Arc::new(|p: &Params| p.is("type", "facets") && p.choice_or("crystalStyle", "gem") == "gem")),
            s("gemRefract", "Repeat Strength", 0.0, 100.0, 45.0).step(1.0).unit("%").sec("Crystal").vis(Arc::new(|p: &Params| p.is("type", "facets") && p.choice_or("crystalStyle", "gem") == "gem")),
            s("seamSoft", "Seam Softness", 0.0, 30.0, 6.0).step(0.5).unit("%").sec("Crystal").vis(Arc::new(|p: &Params| p.is("type", "facets") && p.choice_or("crystalStyle", "gem") == "gem")),
            s("jitter", "Irregular Cut", 0.0, 100.0, 20.0).step(1.0).unit("%").sec("Crystal").vis(Arc::new(|p: &Params| p.is("type", "facets") && p.choice_or("crystalStyle", "gem") == "gem")),
            s("shade", "Facet Shading", 0.0, 50.0, 10.0).step(1.0).unit("%").sec("Crystal").vis(Arc::new(|p: &Params| p.is("type", "facets") && p.choice_or("crystalStyle", "gem") == "gem")),
            s("cell", "Shard Size", 10.0, 400.0, 90.0).step(1.0).curve(1.6).sec("Crystal").vis(Arc::new(|p: &Params| p.is("type", "facets") && p.is("crystalStyle", "shards"))),
            s("refract", "Refraction", 0.0, 200.0, 40.0).step(1.0).curve(1.4).sec("Crystal").vis(Arc::new(|p: &Params| p.is("type", "facets") && p.is("crystalStyle", "shards"))),
            s("stretch", "Stretch", -90.0, 90.0, 0.0).step(1.0).unit("%").sec("Crystal").vis(Arc::new(|p: &Params| p.is("type", "facets") && p.is("crystalStyle", "shards"))),
            s("edges", "Edge Shine", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Crystal").vis(t(&["facets"])),
            ParamDef::seed("seed").sec("Crystal").vis(t(&["facets"])),
            // shared
            s("dispersion", "Rainbow Dispersion", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Color & Mix"),
            s("softness", "Softness", 0.0, 30.0, 3.0).step(0.5).curve(1.4).sec("Color & Mix"),
            s("intensity", "Amount", 0.0, 150.0, 70.0).step(1.0).unit("%").sec("Color & Mix"),
        ]
    })
    .depth_bias()
    .looks(vec![
        look_g("side", "Rainbow Side", "Prism", vals![]),
        look_g("kaleido", "Edge Ghosts", "Prism", vals!["mode" => "ring", "coverage" => 45.0, "copies" => 3.0, "offset" => 60.0]),
        look_g("double", "Double Exposure", "Prism", vals!["mode" => "full", "offset" => 120.0, "copies" => 1.0, "dispersion" => 15.0, "intensity" => 50.0, "blend" => "lighten"]),
        look_g("dreamy", "Dreamy Refraction", "Prism", vals!["softness" => 14.0, "dispersion" => 70.0, "offset" => 25.0, "copies" => 3.0]),
        look_g("steps", "Stepped Prism", "Linear Prism", vals!["type" => "linear", "linStyle" => "bands", "angle" => 180.0, "spacing" => 90.0, "copies" => 5.0, "start" => 35.0, "intensity" => 100.0, "dispersion" => 15.0]),
        look_g("echoes", "Motion Echoes", "Linear Prism", vals!["type" => "linear", "linStyle" => "echo", "angle" => 180.0, "spacing" => 60.0, "copies" => 5.0, "decay" => 35.0, "blend" => "lighten", "intensity" => 100.0, "dispersion" => 0.0]),
        look_g("rainsteps", "Rainbow Steps", "Linear Prism", vals!["type" => "linear", "spacing" => 40.0, "copies" => 6.0, "start" => 20.0, "dispersion" => 80.0, "intensity" => 80.0]),
        look_g("split", "Split Diopter", "Half Prism", vals!["type" => "split", "angle" => 35.0, "offset" => 60.0, "mirror" => false, "intensity" => 100.0, "softness" => 6.0]),
        look_g("mirrorhalf", "Mirror Half", "Half Prism", vals!["type" => "split", "angle" => 0.0, "mirror" => true, "offset" => 0.0, "intensity" => 100.0, "seamRainbow" => 80.0]),
        look_g("k6", "Classic Kaleidoscope", "Kaleidoscope", vals!["type" => "kaleido", "segments" => 6.0, "intensity" => 100.0, "dispersion" => 10.0]),
        look_g("mandala", "Mandala", "Kaleidoscope", vals!["type" => "kaleido", "segments" => 12.0, "rings" => 3.0, "intensity" => 100.0, "zoom" => 140.0, "dispersion" => 25.0]),
        look_g("vortex", "Vortex", "Kaleidoscope", vals!["type" => "kaleido", "segments" => 8.0, "twist" => 220.0, "intensity" => 100.0, "dispersion" => 40.0]),
        look_g("mirror2", "Mirror World", "Kaleidoscope", vals!["type" => "kaleido", "segments" => 2.0, "intensity" => 100.0, "dispersion" => 0.0]),
        look_g("trip", "Psychedelic Trip", "Kaleidoscope", vals!["type" => "kaleido", "segments" => 16.0, "twist" => -160.0, "rings" => 5.0, "zoom" => 70.0, "dispersion" => 90.0, "intensity" => 100.0]),
        look_g("crystal", "Kaleido Filter", "Crystal", vals!["type" => "facets", "intensity" => 100.0]),
        look_g("overlay", "Soft Crystal Overlay", "Crystal", vals!["type" => "facets", "intensity" => 22.0, "softness" => 8.0, "seamSoft" => 12.0, "dispersion" => 60.0, "edges" => 20.0]),
        look_g("gem", "Cut Gem", "Crystal", vals!["type" => "facets", "gemRings" => 3.0, "gemFacets" => 7.0, "clear" => 15.0, "gemRefract" => 65.0, "edges" => 45.0, "intensity" => 100.0]),
        look_g("halfgem", "Edge Facets", "Crystal", vals!["type" => "facets", "gemRings" => 1.0, "gemFacets" => 10.0, "clear" => 55.0, "gemRefract" => 35.0, "intensity" => 85.0]),
        look_g("shards", "Glass Shards", "Crystal", vals!["type" => "facets", "crystalStyle" => "shards", "cell" => 160.0, "refract" => 90.0, "edges" => 60.0, "stretch" => 60.0, "intensity" => 100.0]),
        look_g("mosaic", "Fine Mosaic", "Crystal", vals!["type" => "facets", "crystalStyle" => "shards", "cell" => 25.0, "refract" => 15.0, "edges" => 20.0, "intensity" => 100.0, "dispersion" => 10.0]),
    ])
}

/// Removed in 0.8.0: kept hidden so older projects still render.
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
    .hidden()
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
