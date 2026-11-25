use super::*;

/// Shared "where" controls for blurs: everywhere, only the edges or only the centre.
pub fn area_params(section: &str) -> Vec<ParamDef> {
    vec![
        ParamDef::segmented("area", "Where", &[("all", "Everywhere"), ("edges", "Edges"), ("center", "Center")], "all").sec(section),
        ParamDef::segmented("shape", "Shape", &[("round", "Round"), ("frame", "Frame")], "round").sec(section).vis(when_not("area", "all")),
        s("protect", "Center Size", 0.0, 90.0, 35.0).step(1.0).unit("%").sec(section).vis(when_not("area", "all")),
        s("softness", "Transition", 2.0, 100.0, 45.0).step(1.0).unit("%").sec(section).vis(when_not("area", "all")),
        s("cx", "Center X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec(section).vis(when_not("area", "all")),
        s("cy", "Center Y", 0.0, 100.0, 50.0).step(1.0).unit("%").sec(section).vis(when_not("area", "all")),
    ]
}

pub fn area_mix(input: &T, blurred: &T, p: &Params, ctx: &mut Ctx, amount: f32) -> T {
    let area = match p.choice_or("area", "all").as_str() { "edges" => 1.0, "center" => 2.0, _ => 0.0 };
    if area == 0.0 && amount >= 0.999 {
        return blurred.clone();
    }
    ctx.apply(
        "k_maskmix",
        &[input, blurred],
        &[
            area,
            p.fd("protect", 35.0) / 100.0,
            p.fd("softness", 45.0) / 100.0,
            if p.choice_or("shape", "round") == "frame" { 1.0 } else { 0.0 },
            p.fd("cx", 50.0) / 100.0,
            p.fd("cy", 50.0) / 100.0,
            amount,
        ],
    )
}

pub fn blur() -> Effect {
    let mut params = vec![
        ParamDef::segmented("type", "Type", &[("gaussian", "Gaussian"), ("surface", "Surface (Smart)")], "gaussian").sec("Blur"),
        s("radius", "Radius", 0.0, 200.0, 8.0).step(0.5).curve(2.2).sec("Blur"),
        s("threshold", "Edge Threshold", 1.0, 100.0, 20.0).step(1.0).unit("%").sec("Blur").vis(when("type", "surface")),
        s("amount", "Amount", 0.0, 100.0, 100.0).step(1.0).unit("%").sec("Blur"),
    ];
    params.extend(area_params("Area"));
    Effect::new("blur", "Blur", Category::Blur, |input, p, ctx| {
        let r = ctx.px(p.num("radius", 8.0));
        let blurred = if p.choice_or("type", "gaussian") == "surface" {
            ctx.apply("k_surface", &[input], &[r.max(1.0) as f32, p.fd("threshold", 20.0) / 100.0])
        } else {
            ctx.blur(input, r / 2.0)
        };
        area_mix(input, &blurred, p, ctx, p.fd("amount", 100.0) / 100.0)
    })
    .symbol("aqi.medium")
    .summary("Gaussian or edge-preserving (surface) blur, everywhere, only on the edges or only in the centre.")
    .params(params)
    .looks(vec![
        look("soft", "Soft Focus", vals!["radius" => 6.0, "amount" => 50.0]),
        look("edges", "Blurry Edges", vals!["radius" => 20.0, "area" => "edges", "protect" => 30.0]),
        look("skin", "Smooth Skin", vals!["type" => "surface", "radius" => 10.0, "threshold" => 12.0]),
        look("dream", "Dreamy Center", vals!["radius" => 14.0, "area" => "center", "amount" => 60.0]),
    ])
}

pub fn radial() -> Effect {
    Effect::new("radialblur", "Radial Blur", Category::Blur, |input, p, ctx| {
        let mode = match p.choice_or("mode", "zoom").as_str() { "spin" => 1.0, "both" => 2.0, _ => 0.0 };
        let base = vec![
            mode,
            p.fd("amount", 40.0) / 100.0,
            p.fd("cx", 50.0) / 100.0,
            p.fd("cy", 50.0) / 100.0,
            p.fd("protect", 15.0) / 100.0,
            p.fd("softness", 60.0) / 100.0,
            if p.choice_or("shape", "round") == "frame" { 1.0 } else { 0.0 },
        ];
        let mut u1 = base.clone();
        u1.push(1.0);
        let mut out = ctx.apply("k_radialblur", &[input], &u1);
        if p.choice_or("quality", "high") == "high" {
            // second, finer pass fills the gaps between samples
            let mut u2 = base;
            u2.push(0.12);
            out = ctx.apply("k_radialblur", &[&out], &u2);
        }
        out
    })
    .symbol("smallcircle.filled.circle")
    .summary("Zoom or spin blur around a point, with a sharp protected centre — like Resolve's radial blur.")
    .params(vec![
        ParamDef::segmented("mode", "Mode", &[("zoom", "Zoom"), ("spin", "Spin"), ("both", "Both")], "zoom").sec("Blur"),
        s("amount", "Amount", 0.0, 200.0, 40.0).step(1.0).unit("%").curve(1.3).sec("Blur"),
        s("cx", "Center X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Blur"),
        s("cy", "Center Y", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Blur"),
        ParamDef::segmented("shape", "Shape", &[("round", "Round"), ("frame", "Frame")], "round").sec("Sharp Center"),
        s("protect", "Center Size", 0.0, 90.0, 15.0).step(1.0).unit("%").sec("Sharp Center"),
        s("softness", "Transition", 2.0, 100.0, 60.0).step(1.0).unit("%").sec("Sharp Center"),
        ParamDef::segmented("quality", "Quality", &[("fast", "Fast"), ("high", "High")], "high").sec("Sharp Center"),
    ])
    .looks(vec![
        look("zoom", "Zoom Burst", vals!["mode" => "zoom", "amount" => 60.0]),
        look("spin", "Spin", vals!["mode" => "spin", "amount" => 50.0]),
        look("vortex", "Vortex", vals!["mode" => "both", "amount" => 80.0, "protect" => 10.0]),
        look("subtle", "Subtle Focus Pull", vals!["mode" => "zoom", "amount" => 20.0, "protect" => 35.0]),
    ])
}

pub fn motion() -> Effect {
    let mut params = vec![
        s("angle", "Angle", -180.0, 180.0, 0.0).step(1.0).unit("°").sec("Blur"),
        s("distance", "Distance", 0.0, 600.0, 60.0).step(1.0).curve(1.8).sec("Blur"),
        ParamDef::toggle("centered", "Centered", true).sec("Blur"),
        ParamDef::segmented("shutter", "Shutter", &[("box", "Even"), ("fade", "Fade")], "fade").sec("Blur"),
        s("amount", "Amount", 0.0, 100.0, 100.0).step(1.0).unit("%").sec("Blur"),
    ];
    params.extend(area_params("Area"));
    Effect::new("motionblur", "Motion Blur", Category::Blur, |input, p, ctx| {
        let a = p.n("angle") * std::f64::consts::PI / 180.0;
        let d = ctx.px(p.num("distance", 60.0));
        let (dx, dy) = ((a.cos() * d) as f32, (-a.sin() * d) as f32);
        let shutter = if p.choice_or("shutter", "fade") == "fade" { 1.0 } else { 0.0 };
        let centered = if p.bool_or("centered", true) { 1.0 } else { 0.0 };
        let mut b = ctx.apply("k_dirblur", &[input], &[dx, dy, centered, shutter]);
        if d > 30.0 {
            b = ctx.apply("k_dirblur", &[&b], &[dx / 12.0, dy / 12.0, 1.0, 0.0]);
        }
        area_mix(input, &b, p, ctx, p.fd("amount", 100.0) / 100.0)
    })
    .symbol("arrow.right.and.line.vertical.and.arrow.left")
    .summary("Straight-line motion blur at any angle, centred or trailing, optionally only on the edges.")
    .params(params)
}
