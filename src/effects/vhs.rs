use super::*;
use crate::overlay::{OverlayCanvas, Weight};

/// VCR-style on-screen text: chunky, slightly stretched monospace with an outline.
fn osd(p: &Params, ctx: &Ctx, w: usize, h: usize, camcorder: bool) -> OverlayCanvas {
    let mut ov = OverlayCanvas::new(w, h);
    let size = ctx.px(p.num("osdSize", 30.0));
    let margin = w.min(h) as f64 * 0.07;
    let color = p.color_or("osdColor", Rgba::WHITE);
    let outline = if camcorder { Some(Rgba::new(0.05, 0.05, 0.05)) } else { None };
    let mut draw = |ov: &mut OverlayCanvas, s: &str, x: f64, y: f64| {
        ov.text(s, x, y, size, color, Weight::Heavy, true, !camcorder, None, 0.0, None, 1.18, outline, 0.04);
    };
    let top = p.text_or("osdTop", "PLAY ▶");
    if !top.is_empty() {
        draw(&mut ov, &top, margin, margin);
    }
    let bottom = p.text_or("osdBottom", "AM 10:24\nNOV. 24 1997");
    let lines: Vec<&str> = bottom.split('\n').collect();
    let lh = size * 1.2;
    let mut y = h as f64 - margin - lines.len() as f64 * lh;
    for l in &lines {
        if !(!l.is_empty() || lines.len() > 1) {
            continue;
        }
        if camcorder {
            // camcorders print the date bottom-right
            let est = l.chars().count() as f64 * size * 0.62 * 1.18;
            draw(&mut ov, l, w as f64 - margin - est, y);
        } else {
            draw(&mut ov, l, margin, y);
        }
        y += lh;
    }
    ov
}

pub fn vhs() -> Effect {
    Effect::new("vhs", "VHS", Category::Glitch, |input, p, ctx| {
        let on = |k: &str| if p.bool_or(k, true) { 1.0f32 } else { 0.0 };
        let seed = p.fd("seed", 1.0) * 7.31;
        let unit = ctx.scale as f32;
        let line_h = ctx.px(1.5).round().max(1.0) as f32;
        let osd_on = p.bool_or("osd", true);
        let burn_in = p.choice_or("osdStyle", "vcr") == "camcorder";
        let mut src = input.clone();
        // Camcorder date stamps are recorded on tape: draw them before the degradation.
        if osd_on && burn_in {
            let t = osd(p, ctx, input.width(), input.height(), true).texture();
            src = ctx.apply("k_over_pm", &[&src, &t], &[]);
        }
        let geo = ctx.apply("k_vhs_geo", &[&src], &[
            seed, unit, on("tracking"), p.fd("trackAmt", 5.0), p.fd("bandPos", 82.0) / 100.0, p.fd("bandH", 5.0) / 100.0,
            on("wobble"), p.fd("wobbleAmt", 1.5), p.fd("wobbleFreq", 3.0), on("headSwitch"), p.fd("headH", 2.5) / 100.0, line_h,
        ]);
        let soft_on = p.bool_or("soft", true);
        let color_on = p.bool_or("colorOn", true);
        let u = [
            seed, unit, if soft_on { p.fd("softAmt", 3.0) } else { 0.3 }, if soft_on { p.fd("sharpen", 35.0) / 100.0 } else { 0.0 },
            if soft_on { on("chroma") } else { 0.0 }, p.fd("bleed", 10.0), p.fd("shift", 3.0),
            if soft_on { p.fd("chromaNoise", 25.0) / 100.0 } else { 0.0 },
            if color_on { p.fd("saturation", 85.0) / 100.0 * (1.0 - p.fd("desat", 0.0) / 100.0) } else { 1.0 },
            on("noise"), p.fd("noiseAmt", 40.0) / 100.0, on("dropouts"), p.fd("dropAmt", 30.0) / 100.0,
            on("scanlines"), p.fd("scanAmt", 18.0) / 100.0, ctx.px(p.num("scanSpacing", 3.0)).round().max(2.0) as f32,
            on("vignette"), p.fd("vigAmt", 30.0) / 100.0, if color_on { p.fd("blackLift", 6.0) / 100.0 } else { 0.0 },
            if color_on { p.fd("rolloff", 40.0) / 100.0 } else { 0.0 },
            if color_on { p.fd("warmth", 10.0) / 100.0 } else { 0.0 }, if color_on { p.fd("tint", 8.0) / 100.0 } else { 0.0 },
            on("tracking"), p.fd("bandPos", 82.0) / 100.0, p.fd("bandH", 5.0) / 100.0, p.fd("trackAmt", 5.0),
            on("headSwitch"), p.fd("headH", 2.5) / 100.0, line_h, if color_on { p.fd("contrast", 92.0) / 100.0 } else { 1.0 },
        ];
        let mut out = ctx.apply("k_vhs", &[&geo, input], &u);
        if osd_on && !burn_in {
            let t = osd(p, ctx, input.width(), input.height(), false).texture();
            out = ctx.apply("k_over_pm", &[&out, &t], &[]);
        }
        out
    })
    .symbol("videoprojector")
    .summary("Analog tape: soft luma, bleeding chroma, tracking errors, noise and OSD. Try the Looks.")
    .params(vec![
        ParamDef::section_switch("soft", "Signal", true),
        s("softAmt", "Softness", 0.0, 10.0, 3.0).step(0.1).sec("Signal"),
        s("sharpen", "Edge Enhance", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Signal"),
        ParamDef::toggle("chroma", "Chroma Bleed", true).sec("Signal"),
        s("bleed", "Bleed", 0.0, 40.0, 10.0).step(0.5).curve(1.4).sec("Signal").vis(when_on("chroma")),
        s("shift", "Chroma Delay", -15.0, 15.0, 3.0).step(0.5).sec("Signal").vis(when_on("chroma")),
        s("chromaNoise", "Chroma Noise", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Signal"),
        ParamDef::section_switch("colorOn", "Color", true),
        s("saturation", "Saturation", 0.0, 150.0, 85.0).step(1.0).unit("%").sec("Color"),
        s("contrast", "Contrast", 50.0, 150.0, 92.0).step(1.0).unit("%").sec("Color"),
        s("blackLift", "Black Lift", 0.0, 30.0, 6.0).step(0.5).unit("%").sec("Color"),
        s("rolloff", "Highlight Roll-off", 0.0, 100.0, 40.0).step(1.0).unit("%").sec("Color"),
        s("warmth", "Warmth", -100.0, 100.0, 10.0).step(1.0).sec("Color").track(vec![hex("#3A7BFF"), hex("#9A9A9A"), hex("#FFB020")]),
        s("tint", "Tint", -100.0, 100.0, 8.0).step(1.0).sec("Color").track(vec![hex("#2FBF5A"), hex("#9A9A9A"), hex("#E040C0")]),
        ParamDef::toggle("tracking", "Tracking Errors", true).sec("Tracking"),
        s("trackAmt", "Jitter", 0.0, 30.0, 5.0).step(0.5).curve(1.5).sec("Tracking").vis(when_on("tracking")),
        s("bandPos", "Band Position", 0.0, 100.0, 82.0).step(1.0).unit("%").sec("Tracking").vis(when_on("tracking")),
        s("bandH", "Band Height", 0.0, 30.0, 5.0).step(0.5).unit("%").sec("Tracking").vis(when_on("tracking")),
        ParamDef::toggle("headSwitch", "Head Switching", true).sec("Tracking"),
        s("headH", "Head Switch Height", 0.5, 10.0, 2.5).step(0.1).unit("%").sec("Tracking").vis(when_on("headSwitch")),
        ParamDef::toggle("noise", "Tape Noise", true).sec("Noise"),
        s("noiseAmt", "Amount", 0.0, 100.0, 40.0).step(1.0).unit("%").sec("Noise").vis(when_on("noise")),
        ParamDef::toggle("dropouts", "Dropouts", true).sec("Noise"),
        s("dropAmt", "Dropout Amount", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Noise").vis(when_on("dropouts")),
        ParamDef::section_switch("scanlines", "Scanlines", true),
        s("scanAmt", "Intensity", 0.0, 100.0, 18.0).step(1.0).unit("%").sec("Scanlines"),
        s("scanSpacing", "Spacing", 1.0, 12.0, 3.0).step(0.5).sec("Scanlines"),
        ParamDef::section_switch("wobble", "Wobble", true),
        s("wobbleAmt", "Amount", 0.0, 20.0, 1.5).step(0.1).curve(1.6).sec("Wobble"),
        s("wobbleFreq", "Frequency", 0.5, 20.0, 3.0).step(0.1).sec("Wobble"),
        ParamDef::section_switch("vignette", "Vignette", true),
        s("vigAmt", "Amount", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Vignette"),
        ParamDef::section_switch("osd", "OSD", true),
        ParamDef::choice("osdStyle", "OSD Style", &[("vcr", "VCR (crisp)"), ("camcorder", "Camcorder (burned in)")], "vcr").sec("OSD"),
        ParamDef::text("osdTop", "Top Text", "PLAY ▶").sec("OSD"),
        ParamDef::text("osdBottom", "Bottom Text", "AM 10:24\nNOV. 24 1997").sec("OSD"),
        s("osdSize", "Text Size", 10.0, 80.0, 30.0).step(1.0).sec("OSD"),
        ParamDef::color("osdColor", "Text Color", Rgba::WHITE).sec("OSD"),
        ParamDef::seed("seed").sec("OSD"),
    ])
    .looks(vec![
        look_g("clean", "Clean SP Tape", "VHS", vals!["softAmt" => 2.0, "sharpen" => 25.0, "bleed" => 6.0, "shift" => 2.0, "chromaNoise" => 10.0, "saturation" => 92.0, "blackLift" => 4.0, "tracking" => false, "headSwitch" => true, "headH" => 1.5, "noiseAmt" => 15.0, "dropAmt" => 8.0, "scanAmt" => 12.0, "wobbleAmt" => 0.6, "vigAmt" => 20.0, "osd" => false]),
        look_g("home86", "Home Video '86", "VHS", vals!["softAmt" => 4.0, "sharpen" => 30.0, "bleed" => 16.0, "shift" => 4.0, "chromaNoise" => 35.0, "saturation" => 80.0, "contrast" => 88.0, "blackLift" => 9.0, "rolloff" => 60.0, "warmth" => 30.0, "tint" => 12.0, "trackAmt" => 4.0, "bandH" => 4.0, "noiseAmt" => 45.0, "dropAmt" => 35.0, "wobbleAmt" => 2.5, "vigAmt" => 40.0, "osdTop" => ParamValue::Text("PLAY ▶".into()), "osdBottom" => ParamValue::Text("JUL. 4 1986\n6:42 PM".into())]),
        look_g("music89", "Music Video '89", "VHS", vals!["softAmt" => 2.5, "sharpen" => 55.0, "bleed" => 12.0, "shift" => 5.0, "chromaNoise" => 20.0, "saturation" => 135.0, "contrast" => 110.0, "blackLift" => 3.0, "warmth" => -5.0, "tint" => 30.0, "tracking" => false, "noiseAmt" => 25.0, "dropAmt" => 15.0, "scanAmt" => 22.0, "wobbleAmt" => 1.0, "vigAmt" => 30.0, "osd" => false]),
        look_g("rental94", "Rental Tape '94", "VHS", vals!["softAmt" => 3.5, "sharpen" => 35.0, "bleed" => 12.0, "shift" => 3.0, "chromaNoise" => 30.0, "saturation" => 78.0, "blackLift" => 7.0, "warmth" => 5.0, "tint" => 6.0, "trackAmt" => 9.0, "bandPos" => 86.0, "bandH" => 7.0, "noiseAmt" => 55.0, "dropAmt" => 55.0, "wobbleAmt" => 1.8, "vigAmt" => 30.0, "osdTop" => ParamValue::Text("PLAY ▶".into()), "osdBottom" => ParamValue::Text("SP  0:47:12".into())]),
        look_g("dubbed", "Dubbed 3rd Generation", "VHS", vals!["softAmt" => 6.5, "sharpen" => 60.0, "bleed" => 26.0, "shift" => 7.0, "chromaNoise" => 60.0, "saturation" => 70.0, "contrast" => 85.0, "blackLift" => 12.0, "rolloff" => 70.0, "warmth" => 15.0, "tint" => 25.0, "trackAmt" => 7.0, "noiseAmt" => 65.0, "dropAmt" => 45.0, "wobbleAmt" => 3.0, "vigAmt" => 45.0, "osd" => false]),
        look_g("latenight", "Late Night TV Rip", "VHS", vals!["softAmt" => 3.0, "sharpen" => 40.0, "bleed" => 10.0, "chromaNoise" => 40.0, "saturation" => 88.0, "blackLift" => 5.0, "warmth" => -15.0, "tint" => -18.0, "tracking" => false, "noiseAmt" => 70.0, "dropAmt" => 20.0, "scanAmt" => 35.0, "scanSpacing" => 2.0, "wobbleAmt" => 1.0, "vigAmt" => 35.0, "osd" => true, "osdTop" => ParamValue::Text("CH 03".into()), "osdBottom" => ParamValue::Text("".into())]),
        look_g("eaten", "Eaten Tape", "VHS", vals!["softAmt" => 5.0, "sharpen" => 50.0, "bleed" => 22.0, "shift" => 9.0, "chromaNoise" => 70.0, "saturation" => 75.0, "blackLift" => 10.0, "trackAmt" => 22.0, "bandPos" => 55.0, "bandH" => 16.0, "headH" => 6.0, "noiseAmt" => 90.0, "dropAmt" => 100.0, "wobbleAmt" => 7.0, "wobbleFreq" => 6.0, "vigAmt" => 40.0, "osd" => false]),
        look_g("camcorder98", "Camcorder '98", "Camcorder", vals!["softAmt" => 2.0, "sharpen" => 75.0, "bleed" => 9.0, "shift" => 2.0, "chromaNoise" => 20.0, "saturation" => 95.0, "contrast" => 98.0, "blackLift" => 5.0, "warmth" => -8.0, "tint" => 4.0, "tracking" => false, "headH" => 1.8, "noiseAmt" => 30.0, "dropAmt" => 10.0, "scanAmt" => 10.0, "wobbleAmt" => 0.8, "vigAmt" => 25.0, "osdStyle" => "camcorder", "osdTop" => ParamValue::Text("● REC".into()), "osdBottom" => ParamValue::Text("AUG 14 1998\n10:24:07 PM".into()), "osdSize" => 26.0]),
        look_g("hi8", "Hi8 Vacation", "Camcorder", vals!["softAmt" => 1.6, "sharpen" => 60.0, "bleed" => 7.0, "chromaNoise" => 15.0, "saturation" => 110.0, "contrast" => 104.0, "blackLift" => 3.0, "warmth" => 25.0, "tint" => 0.0, "tracking" => false, "headSwitch" => false, "noiseAmt" => 22.0, "dropAmt" => 5.0, "scanAmt" => 8.0, "wobbleAmt" => 0.5, "vigAmt" => 20.0, "osdStyle" => "camcorder", "osdTop" => ParamValue::Text("".into()), "osdBottom" => ParamValue::Text("12. 7. 1996".into())]),
        look_g("minidv", "MiniDV 2003", "Digital Video", vals!["softAmt" => 0.8, "sharpen" => 70.0, "chroma" => true, "bleed" => 3.0, "shift" => 1.0, "chromaNoise" => 8.0, "saturation" => 100.0, "contrast" => 104.0, "blackLift" => 2.0, "rolloff" => 15.0, "warmth" => -20.0, "tint" => -5.0, "tracking" => false, "headSwitch" => false, "noise" => true, "noiseAmt" => 12.0, "dropouts" => false, "scanAmt" => 14.0, "scanSpacing" => 2.0, "wobble" => false, "vigAmt" => 10.0, "osdStyle" => "camcorder", "osdTop" => ParamValue::Text("SP  ▶".into()), "osdBottom" => ParamValue::Text("2003.07.12  16:20".into())]),
        look_g("cctv", "CCTV Recorder", "Digital Video", vals!["softAmt" => 3.0, "sharpen" => 45.0, "chroma" => false, "chromaNoise" => 0.0, "saturation" => 0.0, "contrast" => 112.0, "blackLift" => 6.0, "warmth" => 0.0, "tint" => 0.0, "trackAmt" => 3.0, "bandH" => 3.0, "noiseAmt" => 55.0, "dropAmt" => 15.0, "scanAmt" => 25.0, "scanSpacing" => 2.0, "wobbleAmt" => 0.4, "vigAmt" => 45.0, "osdTop" => ParamValue::Text("CAM 04".into()), "osdBottom" => ParamValue::Text("11-24-1999  03:14:07".into())]),
    ])
}
