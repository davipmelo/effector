use super::*;

pub fn chromatic() -> Effect {
    Effect::new("chromab", "Chromatic Aberration", Category::Lens, |input, p, ctx| {
        ctx.apply("k_chromab", &[input], &[if p.choice_or("mode", "radial") == "linear" { 1.0 } else { 0.0 }, ctx.px(p.num("amount", 6.0)) as f32,
            p.f("angle"), p.fd("falloff", 1.6), p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0])
    })
    .symbol("camera.aperture")
    .summary("Red/blue colour fringing from a cheap lens, radial or linear.")
    .params(vec![
        ParamDef::segmented("mode", "Mode", &[("radial", "Radial"), ("linear", "Linear")], "radial"),
        s("amount", "Amount", 0.0, 60.0, 6.0).step(0.1).curve(1.8),
        s("angle", "Angle", -180.0, 180.0, 0.0).step(1.0).unit("°").vis(when("mode", "linear")),
        s("falloff", "Falloff", 0.2, 4.0, 1.6).step(0.05).vis(when_not("mode", "linear")),
        s("cx", "Center X", 0.0, 100.0, 50.0).step(1.0).unit("%").vis(when_not("mode", "linear")),
        s("cy", "Center Y", 0.0, 100.0, 50.0).step(1.0).unit("%").vis(when_not("mode", "linear")),
    ])
}

pub fn vignette() -> Effect {
    Effect::new("vignette", "Vignette", Category::Lens, |input, p, ctx| {
        let mut img = input.clone();
        if p.bool_or("blurOn", false) {
            let b = ctx.blur(input, ctx.px(p.num("blur", 12.0)));
            img = ctx.apply("k_maskmix", &[input, &b], &[1.0, p.fd("midpoint", 50.0) / 100.0 * 0.8, p.fd("feather", 55.0) / 100.0,
                0.0, 0.5, 0.5, p.fd("blurAmount", 80.0) / 100.0]);
        }
        let mut u = vec![p.fd("amount", 45.0) / 100.0, p.fd("midpoint", 50.0) / 100.0, p.fd("roundness", 50.0) / 100.0, p.fd("feather", 55.0) / 100.0];
        u.extend(p.color_or("color", Rgba::BLACK).floats());
        u.push(p.f("protect") / 100.0);
        ctx.apply("k_vignette", &[&img], &u)
    })
    .symbol("circle.dashed")
    .summary("Darken (or lighten) the edges of the frame.")
    .params(vec![
        s("amount", "Amount", -100.0, 100.0, 45.0).step(1.0).unit("%"),
        s("midpoint", "Midpoint", 0.0, 100.0, 50.0).step(1.0).unit("%"),
        s("roundness", "Roundness", 0.0, 100.0, 50.0).step(1.0).unit("%"),
        s("feather", "Feather", 5.0, 100.0, 55.0).step(1.0).unit("%"),
        s("protect", "Protect Highlights", 0.0, 100.0, 0.0).step(1.0).unit("%"),
        ParamDef::color("color", "Color", hex("#000000")),
        ParamDef::section_switch("blurOn", "Edge Blur", false),
        s("blur", "Radius", 1.0, 80.0, 12.0).step(0.5).curve(1.6).sec("Edge Blur"),
        s("blurAmount", "Amount", 0.0, 100.0, 80.0).step(1.0).unit("%").sec("Edge Blur"),
    ])
}

pub fn sharpen() -> Effect {
    Effect::new("sharpen", "Sharpen", Category::Adjust, |input, p, ctx| {
        let b = ctx.blur(input, ctx.px(p.num("radius", 1.5)).max(0.35));
        ctx.apply("k_unsharp", &[input, &b], &[p.fd("amount", 80.0) / 100.0, p.fd("threshold", 2.0) / 100.0, if p.bool_or("luma", true) { 1.0 } else { 0.0 }])
    })
    .symbol("triangle")
    .summary("Unsharp mask. Push it for the crunchy over-sharpened digicam look.")
    .params(vec![
        s("amount", "Amount", 0.0, 500.0, 80.0).step(1.0).unit("%").curve(1.5),
        s("radius", "Radius", 0.3, 20.0, 1.5).step(0.05).curve(2.0),
        s("threshold", "Threshold", 0.0, 50.0, 2.0).step(0.5).unit("%"),
        ParamDef::toggle("luma", "Luminance Only", true),
    ])
}

pub fn crt() -> Effect {
    Effect::new("crt", "CRT Screen", Category::Glitch, |input, p, ctx| {
        let on = |k: &str| if p.bool_or(k, true) { 1.0f32 } else { 0.0 };
        let mask_on = p.bool_or("maskOn", true) && p.choice_or("mask", "aperture") != "none";
        let mask: f32 = if mask_on { match p.choice_or("mask", "aperture").as_str() { "shadow" => 2.0, "slot" => 3.0, _ => 1.0 } } else { 0.0 };
        let glow_on = p.bool_or("glowOn", true);
        let glow = if glow_on { ctx.blur(input, ctx.px(p.num("glowRadius", 8.0))) } else { input.clone() };
        let u = [
            p.fd("curvature", 30.0) / 100.0, mask, ctx.px(p.num("maskSize", 1.5)).round().max(1.0) as f32, p.fd("maskAmt", 55.0) / 100.0,
            p.fd("scan", 55.0) / 100.0 * on("scanOn"), ctx.px(p.num("scanSize", 4.0)).round().max(2.0) as f32,
            if glow_on { p.fd("glow", 35.0) / 100.0 * 0.8 } else { 0.0 },
            p.fd("brightness", 125.0) / 100.0, p.fd("vignette", 35.0) / 100.0 * on("vigOn"), p.fd("corner", 8.0) / 100.0,
            p.fd("noise", 10.0) / 100.0 * on("noiseOn"), on("bezel"),
        ];
        ctx.apply("k_crt", &[input, &glow], &u)
    })
    .symbol("tv")
    .summary("Old tube monitor: phosphor mask, scanlines, glow and curved glass.")
    .params(vec![
        s("curvature", "Curvature", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Screen"),
        ParamDef::toggle("bezel", "Black Border", true).sec("Screen"),
        s("corner", "Corner Radius", 0.0, 30.0, 8.0).step(0.5).unit("%").sec("Screen").vis(when_on("bezel")),
        s("brightness", "Brightness", 50.0, 200.0, 125.0).step(1.0).unit("%").sec("Screen"),
        ParamDef::section_switch("maskOn", "Phosphors", true),
        ParamDef::choice("mask", "Phosphor Mask", &[("aperture", "Aperture Grille"), ("shadow", "Shadow Mask"), ("slot", "Slot Mask")], "aperture").sec("Phosphors"),
        s("maskSize", "Mask Size", 1.0, 8.0, 1.5).step(0.25).sec("Phosphors"),
        s("maskAmt", "Mask Strength", 0.0, 100.0, 55.0).step(1.0).unit("%").sec("Phosphors"),
        ParamDef::section_switch("scanOn", "Scanlines", true),
        s("scan", "Intensity", 0.0, 100.0, 55.0).step(1.0).unit("%").sec("Scanlines"),
        s("scanSize", "Line Spacing", 2.0, 16.0, 4.0).step(0.5).sec("Scanlines"),
        ParamDef::section_switch("glowOn", "Glow", true),
        s("glow", "Glow", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Glow"),
        s("glowRadius", "Glow Radius", 1.0, 40.0, 8.0).step(0.5).curve(1.6).sec("Glow"),
        ParamDef::section_switch("vigOn", "Vignette", true),
        s("vignette", "Amount", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Vignette"),
        ParamDef::section_switch("noiseOn", "Noise", true),
        s("noise", "Amount", 0.0, 100.0, 10.0).step(1.0).unit("%").sec("Noise"),
    ])
    .looks(vec![
        look("pc", "PC Monitor '99", vals!["mask" => "shadow", "maskSize" => 1.25, "maskAmt" => 45.0, "scan" => 25.0, "curvature" => 15.0]),
        look("tv", "Living Room TV", vals!["mask" => "slot", "maskSize" => 2.0, "scan" => 50.0, "curvature" => 45.0, "glow" => 50.0]),
        look("arcade", "Arcade Cabinet", vals!["mask" => "aperture", "maskSize" => 2.0, "maskAmt" => 70.0, "scan" => 75.0, "scanSize" => 5.0, "curvature" => 25.0, "glow" => 60.0, "brightness" => 140.0]),
        look("pvm", "Studio PVM", vals!["mask" => "aperture", "maskSize" => 1.25, "maskAmt" => 40.0, "scan" => 80.0, "scanSize" => 3.0, "curvature" => 5.0, "corner" => 2.0, "vignette" => 15.0]),
    ])
}
