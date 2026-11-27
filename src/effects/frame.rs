use super::*;
use crate::overlay::{OverlayCanvas, Weight};

pub const FRAME_RATIOS: [(&str, &str, f64); 9] = [
    ("1.33", "1.33:1 (Academy 4:3)", 4.0 / 3.0),
    ("1.66", "1.66:1 (European)", 1.66),
    ("1.78", "1.78:1 (16:9)", 16.0 / 9.0),
    ("1.85", "1.85:1 (Flat)", 1.85),
    ("2.00", "2.00:1 (Univisium)", 2.0),
    ("2.20", "2.20:1 (70 mm)", 2.2),
    ("2.39", "2.39:1 (Scope)", 2.39),
    ("2.76", "2.76:1 (Ultra Panavision)", 2.76),
    ("3.00", "3.00:1 (Ultra-wide)", 3.0),
];

/// CGRect.integral: smallest integer rect containing (x, y, w, h).
fn integral(x: f64, y: f64, w: f64, h: f64) -> (f64, f64, f64, f64) {
    let (x0, y0, x1, y1) = (x.floor(), y.floor(), (x + w).ceil(), (y + h).ceil());
    (x0, y0, x1 - x0, y1 - y0)
}

pub fn frame() -> Effect {
    // produces alpha: Torn Paper can leave the outside transparent
    let mut e = Effect::new("frame", "Frames & Borders", Category::Overlay, |input, p, ctx| {
        let (w, h) = (input.width() as f64, input.height() as f64);
        let m = w.min(h);
        let k = p.num("size", 100.0) / 100.0;
        let style = p.choice_or("style", "letterbox");
        let mut frame_color = p.color_or("color", hex("#F5F4F0"));
        let rect: (f64, f64, f64, f64);
        let mut radius = ctx.px(p.num("radius", 0.0));
        let mut texture = p.fd("texture", 50.0) / 100.0;
        let mut shadow = p.fd("shadow", 35.0) / 100.0;
        let mut src = (0.0, 0.0, 1.0, 1.0);
        if style == "torn" {
            return torn(input, p, ctx);
        }
        match style.as_str() {
            "letterbox" => {
                frame_color = p.color_or("barColor", Rgba::BLACK);
                texture = 0.0;
                shadow = 0.0;
                radius = 0.0;
                let rid = p.choice_or("ratio", "2.39");
                let a = FRAME_RATIOS.iter().find(|r| r.0 == rid).map(|r| r.2).unwrap_or(2.39);
                let canvas = w / h;
                if a >= canvas {
                    let hh = (w / a).round();
                    rect = (0.0, ((h - hh) / 2.0).round(), w, hh);
                } else {
                    let ww = (h * a).round();
                    rect = (((w - ww) / 2.0).round(), 0.0, ww, h);
                }
                // the bars cover the image (same scale); Reframe slides the visible part
                let (slack_y, slack_x) = (1.0 - rect.3 / h, 1.0 - rect.2 / w);
                let o = p.n("offset") / 100.0;
                src = (slack_x / 2.0 * (1.0 + o), slack_y / 2.0 * (1.0 + o), rect.2 / w, rect.3 / h);
            }
            "border" | "rounded" => {
                let s = m * 0.05 * k;
                rect = integral(s, s, w - 2.0 * s, h - 2.0 * s);
                if style == "rounded" && radius == 0.0 {
                    radius = m * 0.02;
                }
            }
            _ => {
                // polaroid
                let (s, b) = (m * 0.055 * k, m * 0.22 * k);
                rect = integral(s, s, w - 2.0 * s, h - s - b);
                radius = m * 0.002;
            }
        }
        if style != "letterbox" {
            // cover-fit the image into the window
            let (ra, ia) = (rect.2 / rect.3.max(1.0), w / h);
            if ia > ra {
                let ww = ra / ia;
                src = ((1.0 - ww) / 2.0, 0.0, ww, 1.0);
            } else {
                let hh = ia / ra;
                src = (0.0, (1.0 - hh) / 2.0, 1.0, hh);
            }
        }
        let mut u = vec![rect.0 as f32, rect.1 as f32, rect.2 as f32, rect.3 as f32, radius as f32];
        u.extend(frame_color.floats());
        u.extend([texture, shadow, p.fd("seed", 1.0) * 3.3, ctx.scale as f32, src.0 as f32, src.1 as f32, src.2 as f32, src.3 as f32]);
        let mut out = ctx.apply("k_frame", &[input], &u);
        let caption = p.text("caption");
        if p.bool_or("textOn", false) && !caption.is_empty() {
            let mut ov = OverlayCanvas::new(input.width(), input.height());
            let fs = ctx.px(p.num("textSize", 26.0));
            let color = p.color_or("textColor", hex("#F2E36B"));
            let custom = crate::overlay::font_param(p);
            let max_y = rect.1 + rect.3;
            match style.as_str() {
                "letterbox" => {
                    let in_bar = p.choice_or("textPlace", "bar") == "bar" && rect.1 > fs * 1.6;
                    let y = if in_bar { max_y + (h - max_y - fs) / 2.0 } else { max_y - fs * 2.2 };
                    let est = crate::overlay::measure(&caption, fs as f32, false, Weight::Medium, custom.as_deref(), 1.0, 0.0).0 as f64;
                    ov.text(&caption, (w - est) / 2.0, y, fs, color, Weight::Medium, false, !in_bar, None, 0.0, custom.as_deref(), 1.0, None, 0.0);
                }
                "polaroid" => {
                    let bottom = h - max_y;
                    ov.text(&caption, rect.0 + rect.2 * 0.06, max_y + (bottom - fs) * 0.45, fs, color, Weight::Regular, false, false, None,
                        0.0, custom.as_deref().or(Some("Noteworthy-Bold")), 1.0, None, 0.0);
                }
                _ => {
                    ov.text(&caption, rect.0, max_y + ((h - max_y - fs) / 2.0).max(2.0), fs, color, Weight::Medium, false, false, None, 0.0,
                        custom.as_deref(), 1.0, None, 0.0);
                }
            }
            let t = ov.texture();
            out = ctx.apply("k_over_pm", &[&out, &t], &[]);
        }
        out
    })
    .symbol("photo.artframe")
    .summary("Cinema black bars (letterbox) in classic aspect ratios, Polaroid, a simple border, a rounded print or torn paper edges.")
    .params(vec![
        ParamDef::choice("style", "Style", &[("letterbox", "Black Bars (Letterbox)"), ("polaroid", "Polaroid"), ("border", "Simple Border"), ("rounded", "Rounded Print"), ("torn", "Torn Paper")], "letterbox").sec("Frame"),
        ParamDef::choice_opts("ratio", "Aspect Ratio", FRAME_RATIOS.iter().map(|r| opt(r.0, r.1)).collect(), "2.39").sec("Frame").vis(when("style", "letterbox")),
        s("offset", "Reframe", -100.0, 100.0, 0.0).step(1.0).unit("%").sec("Frame").vis(when("style", "letterbox")),
        s("size", "Border Size", 30.0, 250.0, 100.0).step(1.0).unit("%").sec("Frame").vis(when_any("style", &["polaroid", "border", "rounded"])),
        s("radius", "Corner Radius", 0.0, 60.0, 0.0).step(0.5).sec("Frame").vis(when_any("style", &["border", "rounded"])),
        ParamDef::color("color", "Frame Color", hex("#F5F4F0")).sec("Frame").vis(when_not("style", "letterbox")),
        ParamDef::color("barColor", "Bar Color", Rgba::BLACK).sec("Frame").vis(when("style", "letterbox")),
        s("texture", "Texture", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Frame").vis(when_not("style", "letterbox")),
        s("shadow", "Inner Shadow", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Frame").vis(when_any("style", &["polaroid", "border", "rounded"])),
        // Torn Paper (0.9.1)
        ParamDef::segmented("tornMode", "Tear Mode", &[("photo", "Torn Photo"), ("frame", "Paper Frame")], "photo").sec("Torn Edges").vis(when("style", "torn")),
        ParamDef::segmented("tornFill", "Outside the Tear", &[("color", "Color"), ("paper", "Paper"), ("transparent", "Transparent")], "color").sec("Torn Edges").vis(when("style", "torn")),
        ParamDef::image("tornPaper", "Paper", ImageRef::builtin("paper/paper-1", "Paper 1"))
            .sec("Torn Edges")
            .vis(Arc::new(|p: &Params| p.is("style", "torn") && p.is("tornFill", "paper"))),
        s("tornAmount", "Tear Width", 0.0, 100.0, 45.0).step(1.0).unit("%").sec("Torn Edges").vis(when("style", "torn")),
        s("tornDetail", "Raggedness", 0.0, 100.0, 55.0).step(1.0).unit("%").sec("Torn Edges").vis(when("style", "torn")),
        s("tornScale", "Tear Scale", 20.0, 400.0, 100.0).step(1.0).unit("%").curve(1.5).sec("Torn Edges").vis(when("style", "torn")),
        s("tornFiber", "White Fibers", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Torn Edges").vis(when("style", "torn")),
        s("tornShadow", "Shadow", 0.0, 100.0, 40.0).step(1.0).unit("%").sec("Torn Edges").vis(when("style", "torn")),
        ParamDef::seed("tornSeed").sec("Torn Edges").vis(when("style", "torn")),
        ParamDef::toggle("tornTop", "Top", true).sec("Sides").vis(when("style", "torn")),
        s("insetTop", "Top Inset", 0.0, 40.0, 3.0).step(0.5).unit("%").sec("Sides").vis(side_on("tornTop")),
        ParamDef::toggle("tornBottom", "Bottom", true).sec("Sides").vis(when("style", "torn")),
        s("insetBottom", "Bottom Inset", 0.0, 40.0, 3.0).step(0.5).unit("%").sec("Sides").vis(side_on("tornBottom")),
        ParamDef::toggle("tornLeft", "Left", true).sec("Sides").vis(when("style", "torn")),
        s("insetLeft", "Left Inset", 0.0, 40.0, 3.0).step(0.5).unit("%").sec("Sides").vis(side_on("tornLeft")),
        ParamDef::toggle("tornRight", "Right", true).sec("Sides").vis(when("style", "torn")),
        s("insetRight", "Right Inset", 0.0, 40.0, 3.0).step(0.5).unit("%").sec("Sides").vis(side_on("tornRight")),
        ParamDef::section_switch("textOn", "Text", false),
        ParamDef::text("caption", "Caption / Subtitle", "I've seen things you people wouldn't believe.").sec("Text"),
        ParamDef::color("textColor", "Text Color", hex("#F2E36B")).sec("Text"),
        s("textSize", "Text Size", 8.0, 80.0, 26.0).step(1.0).sec("Text"),
        crate::overlay::font_def("Text"),
        ParamDef::segmented("textPlace", "Placement", &[("bar", "In the Bar"), ("image", "On the Image")], "bar").sec("Text").vis(when("style", "letterbox")),
        ParamDef::seed("seed").sec("Text"),
    ])
    .looks(vec![
        look_g("scope", "Scope 2.39:1", "Letterbox", vals!["style" => "letterbox", "ratio" => "2.39"]),
        look_g("flat", "Flat 1.85:1", "Letterbox", vals!["style" => "letterbox", "ratio" => "1.85"]),
        look_g("ultra", "Ultra Panavision 2.76:1", "Letterbox", vals!["style" => "letterbox", "ratio" => "2.76"]),
        look_g("subtitle", "Movie Still with Subtitle", "Letterbox", vals!["style" => "letterbox", "ratio" => "2.39", "textOn" => true]),
        look_g("academy", "Academy 4:3", "Letterbox", vals!["style" => "letterbox", "ratio" => "1.33"]),
        look_g("polaroid", "Polaroid", "Print", vals!["style" => "polaroid"]),
        look_g("white", "White Border", "Print", vals!["style" => "border", "color" => hex("#F7F7F5")]),
        look_g("black", "Black Border", "Print", vals!["style" => "border", "color" => hex("#0B0B0B"), "texture" => 0.0]),
        look_g("rounded", "Rounded Print", "Print", vals!["style" => "rounded"]),
        look_g("torn", "Torn Photo", "Torn Paper", vals!["style" => "torn", "tornMode" => "photo", "color" => hex("#F3F0EA")]),
        look_g("tornDark", "Torn Photo on Black", "Torn Paper", vals!["style" => "torn", "tornMode" => "photo", "color" => hex("#111111"), "tornShadow" => 20.0]),
        look_g("tornFrame", "Torn Paper Frame", "Torn Paper", vals!["style" => "torn", "tornMode" => "frame", "tornFill" => "paper", "color" => hex("#F1ECE2"), "insetTop" => 7.0, "insetBottom" => 7.0, "insetLeft" => 7.0, "insetRight" => 7.0]),
        look_g("tornStrip", "Torn Strip (Top & Bottom)", "Torn Paper", vals!["style" => "torn", "tornLeft" => false, "tornRight" => false, "insetTop" => 12.0, "insetBottom" => 12.0, "tornAmount" => 60.0]),
        look_g("tornOne", "Torn Bottom Edge", "Torn Paper", vals!["style" => "torn", "tornTop" => false, "tornLeft" => false, "tornRight" => false, "insetBottom" => 6.0]),
        look_g("tornCutout", "Cut-Out (Transparent)", "Torn Paper", vals!["style" => "torn", "tornFill" => "transparent", "tornShadow" => 0.0]),
    ]);
    e.produces_alpha = true;
    e
}

fn side_on(key: &'static str) -> Visible { Arc::new(move |p: &Params| p.is("style", "torn") && p.bool_or(key, true)) }

/// Torn Paper style: the photo as a torn piece on a background, or a paper frame torn open over it.
fn torn(input: &T, p: &Params, ctx: &mut Ctx) -> T {
    let (w, h) = (input.width() as f64, input.height() as f64);
    let m = w.min(h);
    let pct = |k: &str| m * p.num(k, 3.0) / 100.0;
    let scale = p.num("tornScale", 100.0) / 100.0;
    let fill = match p.choice_or("tornFill", "color").as_str() {
        "transparent" => 1.0,
        "paper" => 2.0,
        _ => 0.0,
    };
    let paper = if fill == 2.0 { p.image("tornPaper").and_then(|r| crate::textures::texture(&r)) } else { None };
    let mean = paper.as_ref().map(|t| ctx.resize(t, 1, 1, false));
    let color = p.color_or("color", hex("#F5F4F0"));
    let mut u = vec![
        if p.is("tornMode", "frame") { 1.0 } else { 0.0 },
        if fill == 2.0 && paper.is_none() { 0.0 } else { fill },
        color.r as f32,
        color.g as f32,
        color.b as f32,
    ];
    for k in ["tornTop", "tornRight", "tornBottom", "tornLeft"] {
        u.push(if p.bool_or(k, true) { 1.0 } else { 0.0 });
    }
    for k in ["insetTop", "insetRight", "insetBottom", "insetLeft"] {
        u.push(pct(k) as f32);
    }
    u.extend([
        (m * 0.034 * p.num("tornAmount", 45.0) / 100.0 * scale) as f32,
        (m * 0.007 * p.num("tornDetail", 55.0) / 100.0 * scale.sqrt()) as f32,
        (m * 0.22 * scale) as f32,
        (ctx.px(8.0) * p.num("tornFiber", 50.0) / 100.0 * scale.sqrt()) as f32,
        (p.num("tornShadow", 40.0) / 100.0) as f32,
        ctx.px(9.0) as f32,
        p.fd("tornSeed", 1.0) * 3.3,
        ctx.scale as f32,
        paper.is_some() as i32 as f32,
        paper.as_ref().map(|t| t.width() as f32 / t.height().max(1) as f32).unwrap_or(1.0),
        p.fd("texture", 50.0) / 100.0,
        1.4,
    ]);
    match (&paper, &mean) {
        (Some(pt), Some(mt)) => ctx.apply("k_torn", &[input, pt, mt], &u),
        _ => ctx.apply("k_torn", &[input], &u),
    }
}

// SWIFT extra members of FrameEffect:
// static let ratios: [(String, String, Double)] = [
