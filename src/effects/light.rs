use super::*;

/// Shared glow pipeline used by Glow, Bloom, Halation and Film.
#[allow(clippy::too_many_arguments)]
pub fn glow_pipeline(input: &T, ctx: &mut Ctx, threshold: f32, knee: f32, radius: f64, intensity: f32, tint: Rgba, colorize: bool,
                     saturation: f32, mode: f32, multi_scale: bool, neutral: bool) -> T {
    let r = ctx.px(radius).max(0.5);
    // work on a downscaled copy for speed
    let f = ((r / 6.0) as usize).max(1);
    let small = ctx.resize(input, (input.width() / f).max(1), (input.height() / f).max(1), false);
    let bright = ctx.apply("k_bright", &[&small], &[threshold, knee, tint.r as f32, tint.g as f32, tint.b as f32, if colorize { 1.0 } else { 0.0 }, saturation]);
    let sr = r / f as f64;
    let b1 = ctx.blur(&bright, sr * 0.5);
    let b2 = if multi_scale { ctx.blur(&bright, sr * 1.2) } else { b1.clone() };
    let b3 = if multi_scale { ctx.blur(&bright, sr * 3.0) } else { b1.clone() };
    let w: [f32; 3] = if multi_scale { [0.5, 0.35, 0.3] } else { [1.0, 0.0, 0.0] };
    let t: [f32; 3] = if colorize { [1.0, 1.0, 1.0] } else { tint.floats() };
    let mut u = vec![intensity, mode];
    u.extend(w);
    u.extend(t);
    u.push(if neutral { 1.0 } else { 0.0 });
    ctx.apply("k_addglow", &[input, &b1, &b2, &b3], &u)
}

pub fn glow() -> Effect {
    Effect::new("glow", "Glow", Category::Film, |input, p, ctx| {
        let mode = match p.choice_or("mode", "screen").as_str() { "add" => 1.0, "soft" => 2.0, _ => 0.0 };
        glow_pipeline(input, ctx, p.fd("threshold", 30.0) / 100.0, 0.25, p.num("radius", 20.0), p.fd("intensity", 60.0) / 100.0,
            p.color_or("tint", Rgba::WHITE), false, p.fd("saturation", 100.0) / 100.0, mode, false, false)
    })
    .symbol("sun.max")
    .summary("Soft diffusion: the whole image (above a low threshold) gets a dreamy haze.")
    .params(vec![
        s("intensity", "Intensity", 0.0, 200.0, 60.0).step(1.0).unit("%"),
        s("radius", "Radius", 1.0, 200.0, 20.0).step(1.0).curve(2.0),
        s("threshold", "Threshold", 0.0, 100.0, 30.0).step(1.0).unit("%"),
        s("saturation", "Saturation", 0.0, 200.0, 100.0).step(1.0).unit("%"),
        ParamDef::choice("mode", "Blend", &[("screen", "Screen"), ("add", "Add"), ("soft", "Soft Light")], "screen"),
        ParamDef::color("tint", "Tint", Rgba::WHITE),
    ])
    .depth_bias()
}

pub fn bloom() -> Effect {
    Effect::new("bloom", "Bloom", Category::Film, |input, p, ctx| {
        glow_pipeline(input, ctx, p.fd("threshold", 70.0) / 100.0, p.fd("knee", 15.0) / 100.0, p.num("radius", 30.0),
            p.fd("intensity", 100.0) / 100.0, p.color_or("tint", Rgba::WHITE), false, p.fd("saturation", 100.0) / 100.0, 1.0, true, false)
    })
    .symbol("sparkles")
    .summary("Only the brightest highlights spill light, in several sizes, like a lens.")
    .params(vec![
        s("intensity", "Intensity", 0.0, 300.0, 100.0).step(1.0).unit("%"),
        s("radius", "Radius", 1.0, 200.0, 30.0).step(1.0).curve(2.0),
        s("threshold", "Threshold", 0.0, 100.0, 70.0).step(1.0).unit("%"),
        s("knee", "Softness", 0.0, 50.0, 15.0).step(1.0).unit("%"),
        s("saturation", "Saturation", 0.0, 200.0, 100.0).step(1.0).unit("%"),
        ParamDef::color("tint", "Tint", Rgba::WHITE),
    ])
    .depth_bias()
}

/// Film halation: red-channel driven, added in linear light (white cores stay white, the fringe goes
/// orange then deep red), with a tight / wide balance and an optional neutral bloom.
pub fn halation_apply(input: &T, p: &Params, ctx: &mut Ctx) -> T {
    let r = ctx.px(p.num("radius", 18.0)).max(0.5);
    let f = ((r / 8.0) as usize).max(1);
    let small = ctx.resize(input, (input.width() / f).max(1), (input.height() / f).max(1), false);
    let source = if p.choice_or("source", "red") == "luma" { 1.0 } else { 0.0 };
    let src = ctx.apply("k_halo_src", &[&small], &[p.fd("threshold", 84.0) / 100.0, p.fd("softness", 15.0) / 100.0 * 0.5, 0.0, source]);
    let sr = r / f as f64;
    let b1 = ctx.blur(&src, sr * 0.3);
    let b2 = ctx.blur(&src, sr * 1.0);
    let b3 = ctx.blur(&src, sr * 2.6);
    let mut u = vec![p.fd("intensity", 100.0) / 100.0 * 1.8, p.fd("spread", 50.0) / 100.0];
    u.extend(linear_rgb(p.color_or("color", hex("#FF2A0A"))));
    u.extend(linear_rgb(p.color_or("inner", hex("#FF7A2E"))));
    u.push(p.f("bloom") / 100.0 * 0.8);
    ctx.apply("k_halation", &[input, &b1, &b2, &b3], &u)
}

/// sRGB colour -> linear floats.
pub fn linear_rgb(c: Rgba) -> [f32; 3] {
    let f = |v: f64| -> f32 {
        let v = v.clamp(0.0, 1.0);
        (if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }) as f32
    };
    [f(c.r), f(c.g), f(c.b)]
}

pub fn halation() -> Effect {
    Effect::new("halation", "Halation", Category::Film, halation_apply)
        .symbol("circle.dotted.and.circle")
        .summary("Film halation: lights get an orange-to-red fringe while their cores stay white.")
        .params(vec![
            s("intensity", "Intensity", 0.0, 300.0, 100.0).step(1.0).unit("%").sec("Halation"),
            s("radius", "Radius", 1.0, 200.0, 18.0).step(1.0).curve(2.0).sec("Halation"),
            s("spread", "Spread", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Halation"),
            s("threshold", "Threshold", 0.0, 100.0, 84.0).step(1.0).unit("%").sec("Halation"),
            s("softness", "Softness", 0.0, 100.0, 15.0).step(1.0).unit("%").sec("Halation"),
            ParamDef::choice("source", "Source", &[("red", "Red Layer (film)"), ("luma", "All Bright Lights")], "red").sec("Halation"),
            ParamDef::color("inner", "Inner Color", hex("#FF7A2E")).sec("Color"),
            ParamDef::color("color", "Outer Color", hex("#FF2A0A")).sec("Color"),
            s("bloom", "Bloom", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Color"),
        ])
    .depth_bias()
}

pub fn trail() -> Effect {
    Effect::new("trail", "Trail & Motion Blur", Category::Glitch, |input, p, ctx| {
        let mode = match p.choice_or("mode", "dir").as_str() {
            "zoom" => 1.0,
            "spin" => 2.0,
            "arc" => 3.0,
            "wave" => 4.0,
            "shake" => 5.0,
            "spiral" => 6.0,
            _ => 0.0,
        };
        let blend = match p.choice_or("blend", "lighten").as_str() {
            "screen" => 1.0,
            "normal" => 2.0,
            "difference" => 3.0,
            "darken" => 4.0,
            "add" => 5.0,
            _ => 0.0,
        };
        let cm = match p.choice_or("colorMode", "off").as_str() { "rainbow" => 1.0, "gradient" => 2.0, _ => 0.0 };
        let (ca, cb) = (p.color_or("colorA", hex("#FF3D7F")), p.color_or("colorB", hex("#3DA9FF")));
        let thr = if p.bool("lightsOnly") { p.fd("lightThreshold", 80.0) / 100.0 } else { 0.0 };
        ctx.apply("k_trail", &[input], &[
            mode, p.f("angle"), ctx.px(p.num("distance", 60.0)) as f32, p.fd("copies", 8.0), p.fd("decay", 75.0) / 100.0, blend,
            p.f("rgbShift") / 100.0 * 0.5, p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0, p.fd("opacity", 60.0) / 100.0,
            if p.choice_or("style", "copies") == "smooth" { 1.0 } else { 0.0 }, p.b("centered"), p.fd("bend", 60.0),
            ctx.px(p.num("amplitude", 20.0)) as f32, p.fd("waves", 2.0), p.fd("seed", 1.0) * 3.1, cm,
            ca.r as f32, ca.g as f32, ca.b as f32, cb.r as f32, cb.g as f32, cb.b as f32, thr, p.fd("hueCycles", 1.0),
            p.fd("turn", 90.0), p.fd("colorAmount", 70.0) / 100.0,
        ])
    })
    .symbol("wind")
    .summary("Ghost copies or a smooth trail along a line, zoom, spin, arc, wave, hand shake or spiral — with rainbow copies and light-only trails.")
    .params(vec![
        ParamDef::segmented("style", "Style", &[("copies", "Copies"), ("smooth", "Smooth")], "copies").sec("Trail"),
        ParamDef::choice("mode", "Path", &[
            ("dir", "Straight"), ("zoom", "Zoom"), ("spin", "Spin"), ("arc", "Arc"), ("wave", "Wave"), ("shake", "Hand Shake"), ("spiral", "Spiral"),
        ], "dir").sec("Trail"),
        ParamDef::toggle("centered", "Both Directions", false).sec("Trail"),
        s("angle", "Angle", -180.0, 180.0, 0.0).step(1.0).unit("°").sec("Trail").vis(when_not("mode", "zoom")),
        s("distance", "Distance", 0.0, 400.0, 60.0).step(1.0).curve(1.8).sec("Trail"),
        s("copies", "Copies", 1.0, 64.0, 8.0).step(1.0).curve(1.4).sec("Trail").vis(when_not("style", "smooth")),
        s("decay", "Decay", 0.0, 100.0, 75.0).step(1.0).unit("%").sec("Trail"),
        s("opacity", "Ghost Opacity", 0.0, 100.0, 60.0).step(1.0).unit("%").sec("Trail"),
        ParamDef::choice("blend", "Ghost Blend", &[("lighten", "Lighten"), ("screen", "Screen"), ("add", "Add"), ("normal", "Normal"), ("difference", "Difference"), ("darken", "Darken")], "lighten").sec("Trail"),
        // path shape
        s("bend", "Bend", -360.0, 360.0, 60.0).step(1.0).unit("°").sec("Path").vis(when("mode", "arc")),
        s("amplitude", "Amplitude", 0.0, 200.0, 20.0).step(1.0).curve(1.5).sec("Path").vis(when_any("mode", &["wave", "shake"])),
        s("waves", "Waves", 0.5, 12.0, 2.0).step(0.5).sec("Path").vis(when("mode", "wave")),
        s("turn", "Turn", -720.0, 720.0, 90.0).step(1.0).unit("°").sec("Path").vis(when("mode", "spiral")),
        ParamDef::seed("seed").sec("Path").vis(when("mode", "shake")),
        s("cx", "Center X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Path").vis(when_any("mode", &["zoom", "spin", "spiral"])),
        s("cy", "Center Y", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Path").vis(when_any("mode", &["zoom", "spin", "spiral"])),
        // colour
        ParamDef::segmented("colorMode", "Copy Color", &[("off", "Off"), ("rainbow", "Rainbow"), ("gradient", "Gradient")], "off").sec("Color"),
        s("colorAmount", "Color Strength", 0.0, 100.0, 70.0).step(1.0).unit("%").sec("Color").vis(when_not("colorMode", "off")),
        s("hueCycles", "Rainbow Cycles", 0.2, 6.0, 1.0).step(0.1).sec("Color").vis(when("colorMode", "rainbow")),
        ParamDef::color("colorA", "Start Color", hex("#FF3D7F")).sec("Color").vis(when("colorMode", "gradient")),
        ParamDef::color("colorB", "End Color", hex("#3DA9FF")).sec("Color").vis(when("colorMode", "gradient")),
        s("rgbShift", "RGB Split", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Color"),
        // lights
        ParamDef::section_switch("lightsOnly", "Light Trails Only", false),
        s("lightThreshold", "Brightness", 0.0, 100.0, 80.0).step(1.0).unit("%").sec("Light Trails Only"),
    ])
    .depth_bias()
    .looks(vec![
        look_g("ghosts", "Ghost Copies", "Copies", vals![]),
        look_g("prism", "Prismatic Echo", "Copies", vals!["copies" => 6.0, "distance" => 50.0, "colorMode" => "rainbow", "colorAmount" => 80.0, "blend" => "screen", "opacity" => 55.0, "decay" => 80.0]),
        look_g("stutter", "Stutter", "Copies", vals!["copies" => 4.0, "distance" => 120.0, "decay" => 60.0, "opacity" => 80.0, "blend" => "normal"]),
        look_g("duo", "Duotone Echo", "Copies", vals!["copies" => 10.0, "colorMode" => "gradient", "blend" => "screen", "opacity" => 50.0]),
        look_g("shake", "Hand Shake", "Copies", vals!["mode" => "shake", "copies" => 12.0, "distance" => 10.0, "amplitude" => 18.0, "opacity" => 45.0, "centered" => true]),
        look_g("motion", "Motion Blur", "Smooth", vals!["style" => "smooth", "centered" => true, "opacity" => 100.0, "blend" => "normal", "decay" => 100.0]),
        look_g("swing", "Camera Swing", "Smooth", vals!["style" => "smooth", "mode" => "arc", "bend" => 90.0, "distance" => 140.0, "opacity" => 85.0, "blend" => "normal"]),
        look_g("zoomburst", "Zoom Burst", "Smooth", vals!["style" => "smooth", "mode" => "zoom", "distance" => 120.0, "opacity" => 90.0, "blend" => "normal", "decay" => 90.0]),
        look_g("vortex", "Spiral Vortex", "Smooth", vals!["style" => "smooth", "mode" => "spiral", "distance" => 60.0, "turn" => 120.0, "opacity" => 90.0, "blend" => "normal"]),
        look_g("light", "Light Painting", "Lights", vals!["style" => "smooth", "mode" => "wave", "distance" => 160.0, "amplitude" => 30.0, "waves" => 3.0, "lightsOnly" => true, "blend" => "add", "opacity" => 100.0, "decay" => 90.0]),
        look_g("rainbowlights", "Rainbow Light Trails", "Lights", vals!["copies" => 24.0, "distance" => 140.0, "lightsOnly" => true, "lightThreshold" => 82.0, "colorMode" => "rainbow", "blend" => "add", "opacity" => 30.0, "decay" => 90.0]),
    ])
}
