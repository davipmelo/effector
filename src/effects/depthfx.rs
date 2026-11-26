//! Effects that use the AI subject matte (0.8.3; the depth map of 0.8.0–0.8.2 is gone):
//! Background Blur (bokeh behind a sharp subject), Lens Emulation and Fog & Haze. Without the
//! matte they fall back to sensible 2D versions. Parallax & Relief only worked with depth and is
//! kept hidden for old projects.

use super::*;

/// Shared focus + aperture parameters (Depth Blur and Lens Emulation).
fn bokeh_params(sec_focus: &str, sec_shape: &str, def_amount: f64) -> Vec<ParamDef> {
    vec![
        // "depth" (the key of old projects) now means: the AI subject stays sharp
        ParamDef::segmented("focusMode", "Focus", &[("depth", "Subject"), ("radial", "Radial"), ("band", "Tilt-Shift"), ("whole", "Whole")], "depth").sec(sec_focus),
        s("range", "Sharp Zone", 0.0, 60.0, 14.0).step(0.5).unit("%").sec(sec_focus).vis(when_any("focusMode", &["radial", "band"])),
        s("transition", "Transition", 1.0, 60.0, 14.0).step(0.5).unit("%").sec(sec_focus).vis(when_not("focusMode", "whole")),
        s("cx", "Center X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec(sec_focus).vis(when("focusMode", "radial")),
        s("cy", "Center Y", 0.0, 100.0, 50.0).step(1.0).unit("%").sec(sec_focus).vis(when_any("focusMode", &["radial", "band"])),
        s("bandAngle", "Angle", -90.0, 90.0, 0.0).step(1.0).unit("°").sec(sec_focus).vis(when("focusMode", "band")),
        s("amount", "Blur", 0.0, 100.0, def_amount).step(0.5).curve(1.6).sec(sec_shape),
        ParamDef::segmented("blades", "Aperture", &[("0", "Round"), ("5", "5"), ("6", "6"), ("7", "7"), ("9", "9")], "0").sec(sec_shape),
        s("bladeRot", "Blade Rotation", 0.0, 72.0, 0.0).step(1.0).unit("°").sec(sec_shape).vis(when_not("blades", "0")),
        s("squeeze", "Anamorphic Oval", 100.0, 250.0, 100.0).step(1.0).unit("%").sec(sec_shape),
        s("swirl", "Swirl", 0.0, 100.0, 0.0).step(1.0).unit("%").sec(sec_shape),
        s("catEye", "Cat's Eye", 0.0, 100.0, 0.0).step(1.0).unit("%").sec(sec_shape),
        s("bokeh", "Bokeh Brightness", 0.0, 100.0, 40.0).step(1.0).unit("%").sec(sec_shape),
    ]
}

/// Runs the bokeh blur with the shared parameters (falls back to radial without a depth map).
fn bokeh(input: &T, p: &Params, ctx: &mut Ctx) -> T {
    let amt = ctx.px(p.num("amount", 18.0));
    if amt < 0.5 {
        return input.clone();
    }
    let mut mode = match p.choice_or("focusMode", "depth").as_str() { "radial" => 1.0, "band" => 2.0, "whole" => 3.0, _ => 0.0 };
    let d = if mode == 0.0 { ctx.subject() } else { None };
    if mode == 0.0 && d.is_none() {
        mode = 1.0;
    }
    let dt = d.unwrap_or_else(|| input.clone());
    let blades: f32 = p.choice_or("blades", "0").parse().unwrap_or(0.0);
    // subject mode: the matte is the "distance" (1 = subject), in focus at 1, sharp zone fixed
    let (focus, range) = if mode == 0.0 { (1.0, 0.2) } else { (p.fd("focus", 88.0) / 100.0, p.fd("range", 14.0) / 100.0) };
    let out = ctx.apply("k_bokeh", &[input, &dt], &[
        amt as f32, focus, range, mode, p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0,
        p.f("bandAngle"), p.fd("squeeze", 100.0) / 100.0, p.f("swirl") / 100.0, p.f("catEye") / 100.0, blades, p.f("bladeRot"),
        p.fd("bokeh", 40.0) / 100.0, p.fd("seed", 1.0), 0.0, p.fd("transition", 14.0) / 100.0,
    ]);
    ctx.apply("k_clamp", &[&out], &[])
}

fn uses_focus_depth(p: &Params) -> bool { p.choice_or("focusMode", "depth") == "depth" || uses_subject_bias(p) }

pub fn depth_blur() -> Effect {
    Effect::new("depthblur", "Background Blur", Category::Blur, |input, p, ctx| bokeh(input, p, ctx))
        .symbol("camera.aperture")
        .summary("Shallow depth of field: the subject (detected by AI) stays sharp, the background melts into round, bladed, oval or swirly bokeh.")
        .subject(uses_focus_depth)
        .params({
            let mut v = bokeh_params("Focus", "Bokeh", 18.0);
            v.push(ParamDef::seed("seed").sec("Bokeh"));
            v
        })
        .looks(vec![
            look("portrait", "Portrait f/1.4", vals!["amount" => 22.0]),
            look("subtle", "Subtle Background", vals!["amount" => 8.0, "transition" => 25.0]),
            look("creamy", "Creamy f/0.95", vals!["amount" => 40.0, "bokeh" => 60.0, "transition" => 10.0]),
            look("hexagon", "Vintage Hexagons", vals!["blades" => "6", "amount" => 24.0, "bokeh" => 70.0]),
            look("swirl", "Swirly Helios", vals!["swirl" => 70.0, "catEye" => 40.0, "amount" => 26.0, "bokeh" => 55.0]),
            look("oval", "Anamorphic Ovals", vals!["squeeze" => 200.0, "amount" => 24.0, "bokeh" => 60.0]),
            look("mini", "Miniature (Tilt-Shift)", vals!["focusMode" => "band", "cy" => 55.0, "range" => 6.0, "transition" => 18.0, "amount" => 14.0]),
        ])
}

pub fn lens_emulation() -> Effect {
    Effect::new("lensemu", "Lens Emulation", Category::Lens, |input, p, ctx| {
        let mut img = input.clone();
        // optics: distortion, lateral colour, soft corners, vignette
        if p.bool_or("opticsOn", true) {
            let soft = ctx.blur(&img, ctx.px(4.0));
            img = ctx.apply("k_camlens", &[&img, &soft], &[
                p.f("distortion") / 100.0, p.f("ca") / 100.0, p.f("vignette") / 100.0, p.f("cornerSoft") / 100.0, p.fd("vigRound", 80.0) / 100.0,
            ]);
        }
        // focus falloff + bokeh
        if p.bool_or("focusOn", true) {
            img = bokeh(&img, p, ctx);
        }
        // glow / diffusion (bloom around lights, lifted shadows of old or filtered glass)
        let glow = p.f("glow") / 100.0;
        if glow > 0.001 {
            img = crate::effects::light::glow_pipeline(&img, ctx, p.fd("glowThreshold", 55.0) / 100.0, 0.3, p.num("glowRadius", 30.0), glow,
                p.color_or("glowTint", Rgba::WHITE), false, 1.0, 2.0, true, false);
        }
        // veiling flare, colour rendering
        let veil = p.f("veil") / 100.0;
        let tint = p.color_or("rendering", Rgba::WHITE);
        if veil > 0.001 || (tint.r - 1.0).abs() + (tint.g - 1.0).abs() + (tint.b - 1.0).abs() > 0.01 || (p.fd("contrast", 100.0) - 100.0).abs() > 0.5 {
            img = ctx.apply("k_camcolor", &[&img], &[
                0.0, 0.0, 0.0, 0.0, veil * 0.12, 0.0, 0.0, p.fd("saturation", 100.0) / 100.0, p.fd("contrast", 100.0) / 100.0,
                tint.r as f32 * 0.3 + 0.7, tint.g as f32 * 0.3 + 0.7, tint.b as f32 * 0.3 + 0.7,
            ]);
        }
        // anamorphic streaks
        let streak = p.f("streak") / 100.0;
        if streak > 0.001 {
            if let Some(e) = crate::effects::effect("anamorphic") {
                let mut n = LayerNode::effect("anamorphic", None, None);
                n.params.set("intensity", ParamValue::Number(streak as f64 * 90.0));
                n.params.set("length", ParamValue::Number(55.0));
                n.params.set("color", ParamValue::Color(p.color_or("streakColor", hex("#3D7CFF"))));
                img = (e.apply)(&img, &n.params, ctx);
            }
        }
        img
    })
    .symbol("camera.aperture")
    .summary("Emulates whole lenses — cinema primes, anamorphics, vintage Soviet glass, toy cameras: a sharp subject with bokeh behind, distortion, glow, flares.")
    .subject(|p| p.bool_or("focusOn", true) && uses_focus_depth(p))
    .params({
        let mut v = vec![ParamDef::section_switch("focusOn", "Focus", true)];
        v.extend(bokeh_params("Focus", "Focus", 14.0));
        v.extend([
            ParamDef::section_switch("opticsOn", "Optics", true),
            s("distortion", "Distortion", -60.0, 60.0, 0.0).step(1.0).unit("%").sec("Optics"),
            s("ca", "Color Fringing", 0.0, 100.0, 10.0).step(1.0).unit("%").sec("Optics"),
            s("cornerSoft", "Soft Corners", 0.0, 100.0, 15.0).step(1.0).unit("%").sec("Optics"),
            s("vignette", "Vignette", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Optics"),
            s("vigRound", "Vignette Roundness", 0.0, 100.0, 80.0).step(1.0).unit("%").sec("Optics"),
            s("glow", "Glow", 0.0, 150.0, 0.0).step(1.0).unit("%").curve(1.4).sec("Rendering"),
            s("glowRadius", "Glow Radius", 2.0, 120.0, 30.0).step(1.0).curve(1.6).sec("Rendering"),
            s("glowThreshold", "Glow Threshold", 0.0, 100.0, 55.0).step(1.0).unit("%").sec("Rendering"),
            ParamDef::color("glowTint", "Glow Tint", Rgba::WHITE).sec("Rendering"),
            s("veil", "Veiling Flare", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Rendering"),
            s("contrast", "Contrast", 50.0, 150.0, 100.0).step(1.0).unit("%").sec("Rendering"),
            s("saturation", "Saturation", 0.0, 200.0, 100.0).step(1.0).unit("%").sec("Rendering"),
            ParamDef::color("rendering", "Color Rendering", Rgba::WHITE).sec("Rendering"),
            s("streak", "Anamorphic Streaks", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Flare"),
            ParamDef::color("streakColor", "Streak Color", hex("#3D7CFF")).sec("Flare"),
            ParamDef::seed("seed").sec("Focus"),
        ]);
        v
    })
    .looks(vec![
        look_g("cine", "Cinema Prime", "Cinema", vals!["amount" => 14.0, "range" => 7.0, "vignette" => 20.0, "glow" => 15.0, "glowRadius" => 40.0, "ca" => 5.0]),
        look_g("anamorphic", "Anamorphic 2x", "Cinema", vals![
            "amount" => 18.0, "squeeze" => 200.0, "distortion" => 8.0, "vignette" => 35.0, "cornerSoft" => 30.0, "streak" => 45.0, "glow" => 15.0, "bokeh" => 55.0
        ]),
        look_g("vista", "Large Format Vista", "Cinema", vals!["amount" => 26.0, "transition" => 22.0, "vignette" => 15.0, "contrast" => 96.0]),
        look_g("helios", "Helios 44 Swirl", "Vintage", vals![
            "amount" => 24.0, "swirl" => 75.0, "catEye" => 45.0, "bokeh" => 55.0, "vignette" => 40.0, "ca" => 25.0, "glow" => 30.0, "veil" => 25.0,
            "contrast" => 90.0, "rendering" => hex("#FFE3B8")
        ]),
        look_g("petzval", "Petzval Portrait", "Vintage", vals!["amount" => 30.0, "swirl" => 100.0, "catEye" => 60.0, "cornerSoft" => 60.0, "vignette" => 50.0, "glow" => 20.0]),
        look_g("seventies", "70s Zoom", "Vintage", vals!["amount" => 12.0, "ca" => 45.0, "glow" => 45.0, "veil" => 40.0, "contrast" => 85.0, "saturation" => 90.0, "rendering" => hex("#FFD9A8"), "distortion" => -8.0]),
        look_g("promist", "Black Pro-Mist", "Filters", vals!["focusOn" => false, "glow" => 70.0, "glowRadius" => 60.0, "glowThreshold" => 35.0, "veil" => 20.0, "contrast" => 94.0]),
        look_g("toy", "Plastic Toy Camera", "Toy", vals![
            "focusMode" => "radial", "range" => 28.0, "transition" => 30.0, "amount" => 10.0, "vignette" => 80.0, "vigRound" => 100.0, "cornerSoft" => 70.0,
            "ca" => 60.0, "distortion" => 15.0, "saturation" => 120.0, "contrast" => 112.0
        ]),
        look_g("wide", "Cheap Wide Zoom", "Toy", vals!["focusOn" => false, "distortion" => 35.0, "ca" => 70.0, "cornerSoft" => 60.0, "vignette" => 35.0]),
        look_g("macro", "Macro", "Cinema", vals!["amount" => 30.0, "transition" => 8.0, "bokeh" => 30.0]),
    ])
}

pub fn fog() -> Effect {
    Effect::new("fog", "Fog & Haze", Category::Film, |input, p, ctx| {
        let d = ctx.subject();
        let has = d.is_some() as i32 as f32;
        let dt = d.unwrap_or_else(|| input.clone());
        let glow = p.fd("glow", 30.0) / 100.0;
        let g = if glow > 0.001 {
            let b = ctx.apply("k_bright", &[input], &[0.6, 0.2, 1.0, 1.0, 1.0, 0.0, 1.0]);
            ctx.blur(&b, ctx.px(p.num("glowRadius", 40.0)))
        } else {
            input.clone()
        };
        let c = p.color_or("color", hex("#C9D3DC"));
        ctx.apply("k_fog", &[input, &dt, &g], &[
            p.fd("density", 60.0) / 100.0, p.fd("curve", 150.0) / 100.0, c.r as f32, c.g as f32, c.b as f32, p.fd("start", 10.0) / 100.0,
            p.f("height") / 100.0, p.fd("heightPos", 70.0) / 100.0, p.fd("patchy", 25.0) / 100.0, ctx.px(p.num("patchScale", 120.0)) as f32,
            p.fd("seed", 1.0) * 3.3, glow, has, p.fd("lift", 30.0) / 100.0,
        ])
    })
    .symbol("cloud.fog")
    .summary("Atmospheric fog and haze over the background while the subject (detected by AI) stays clear, with glow around lights.")
    .subject(|_| true)
    .params(vec![
        s("density", "Density", 0.0, 100.0, 60.0).step(1.0).unit("%").sec("Fog"),
        s("start", "Starts At", 0.0, 90.0, 10.0).step(1.0).unit("%").sec("Fog"),
        s("curve", "Falloff", 30.0, 400.0, 150.0).step(1.0).unit("%").curve(1.4).sec("Fog"),
        ParamDef::color("color", "Color", hex("#C9D3DC")).sec("Fog"),
        s("lift", "Haze Lift", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Fog"),
        s("height", "Ground Fog", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Shape"),
        s("heightPos", "Ground Level", 0.0, 100.0, 70.0).step(1.0).unit("%").sec("Shape"),
        s("patchy", "Patchiness", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Shape"),
        s("patchScale", "Patch Size", 10.0, 600.0, 120.0).step(1.0).curve(1.6).sec("Shape"),
        ParamDef::seed("seed").sec("Shape"),
        s("glow", "Light Glow", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Glow"),
        s("glowRadius", "Glow Radius", 5.0, 200.0, 40.0).step(1.0).curve(1.6).sec("Glow"),
    ])
    .looks(vec![
        look("morning", "Morning Mist", vals![]),
        look("smog", "City Smog", vals!["color" => hex("#B8AE96"), "density" => 55.0, "curve" => 110.0, "lift" => 45.0]),
        look("blue", "Blue Hour Haze", vals!["color" => hex("#7F97C4"), "density" => 50.0, "glow" => 55.0]),
        look("ground", "Ground Fog", vals!["height" => 80.0, "heightPos" => 75.0, "density" => 80.0, "patchy" => 45.0]),
        look("dense", "Pea Soup", vals!["density" => 92.0, "curve" => 70.0, "start" => 0.0, "lift" => 60.0]),
        look("golden", "Golden Dust", vals!["color" => hex("#E8B77A"), "density" => 45.0, "glow" => 60.0, "patchy" => 40.0]),
        look("subtle", "Aerial Perspective", vals!["density" => 30.0, "curve" => 200.0, "patchy" => 0.0, "lift" => 15.0]),
    ])
}

pub fn relief() -> Effect {
    // Removed in 0.8.3 with the depth map: kept hidden so old projects still open (renders unchanged).
    Effect::new("relief", "Parallax & Relief", Category::Distort, |input, _p, _ctx| input.clone())
    .symbol("cube.transparent")
    .summary("Experimental depth tricks: 3D parallax shift, an embossed relief of the scene, or near/far colour tint.")
    .hidden()
    .params(vec![
        ParamDef::segmented("mode", "Mode", &[("parallax", "Parallax"), ("relief", "Relief"), ("tint", "Depth Tint")], "parallax").sec("Depth"),
        s("shift", "Shift", -150.0, 150.0, 20.0).step(1.0).curve(1.5).sec("Depth").vis(when("mode", "parallax")),
        s("angle", "Direction", -180.0, 180.0, 0.0).step(1.0).unit("°").sec("Depth").vis(when("mode", "parallax")),
        s("pivot", "Pivot Depth", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Depth").vis(when("mode", "parallax")),
        s("amount", "Amount", 0.0, 100.0, 70.0).step(1.0).unit("%").sec("Depth").vis(when_not("mode", "parallax")),
        s("light", "Light Angle", 0.0, 360.0, 135.0).step(1.0).unit("°").sec("Depth").vis(when("mode", "relief")),
        s("strength", "Relief Depth", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Depth").vis(when("mode", "relief")),
        ParamDef::color("nearColor", "Near Color", hex("#FF6A3D")).sec("Depth").vis(when("mode", "tint")),
        ParamDef::color("farColor", "Far Color", hex("#2A3BFF")).sec("Depth").vis(when("mode", "tint")),
        ParamDef::segmented("tintBlend", "Blend", &[("soft", "Soft"), ("multiply", "Multiply"), ("normal", "Normal")], "soft").sec("Depth").vis(when("mode", "tint")),
    ])
    .looks(vec![
        look("pop", "3D Pop", vals![]),
        look("wiggle", "Wiggle Frame", vals!["shift" => 35.0, "angle" => 0.0, "pivot" => 60.0]),
        look("emboss", "Embossed Relief", vals!["mode" => "relief", "amount" => 80.0, "strength" => 70.0]),
        look("thermal", "Depth Heat", vals!["mode" => "tint", "nearColor" => hex("#FFD23D"), "farColor" => hex("#3A0CA3"), "tintBlend" => "normal", "amount" => 85.0]),
        look("sunset", "Warm Near, Cool Far", vals!["mode" => "tint", "nearColor" => hex("#FFB070"), "farColor" => hex("#5A7BFF"), "amount" => 55.0]),
    ])
}
