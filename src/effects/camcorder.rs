//! Camcorder: one effect for every home-video look (Hi8 / VHS camcorders, MiniDV, NightShot,
//! CCTV, early digicams, a TV filmed off the screen). Replaces the old VHS and Digicam effects
//! (kept hidden for older projects).
//!
//! Pipeline, in the order a real camera works: lens → sensor (camera resolution) → colour →
//! edge enhancement → chroma → compression → interlace → date/OSD burned in → tape → screen.

use super::*;
use crate::exif::DateTaken;
use crate::overlay::{measure, OverlayCanvas, Weight};

const MONTHS: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];

/// Date stamp lines for a capture date in one of the classic formats.
pub fn date_lines(d: &DateTaken, format: &str, time: bool, time_first: bool) -> Vec<String> {
    let (h12, ampm) = {
        let h = d.hour % 12;
        (if h == 0 { 12 } else { h }, if d.hour < 12 { "AM" } else { "PM" })
    };
    let (date, t) = match format {
        "eu" => (format!("{}. {}. {}", d.day, d.month, d.year), format!("{}:{:02}", d.hour, d.minute)),
        "iso" => (format!("{}.{:02}.{:02}", d.year, d.month, d.day), format!("{:02}:{:02}", d.hour, d.minute)),
        "digicam" => (format!("'{:02} {} {}", d.year % 100, d.month, d.day), format!("{:02}:{:02}", d.hour, d.minute)),
        "cctv" => {
            let s = format!("{:02}-{:02}-{}  {:02}:{:02}:{:02}", d.month, d.day, d.year, d.hour, d.minute, d.second);
            return vec![if time { s } else { format!("{:02}-{:02}-{}", d.month, d.day, d.year) }];
        }
        _ => (
            format!("{} {:2} {}", MONTHS[(d.month as usize).clamp(1, 12) - 1], d.day, d.year),
            format!("{}:{:02}:{:02} {}", h12, d.minute, d.second, ampm),
        ),
    };
    match (time, time_first) {
        (false, _) => vec![date],
        (true, true) => vec![t, date],
        (true, false) => vec![date, t],
    }
}

fn text_style(style: &str) -> (Weight, bool, f64, Option<Rgba>, bool) {
    // (weight, shadow, stretch, outline, led glow)
    match style {
        "vcr" => (Weight::Heavy, false, 1.18, None, false),
        "led" => (Weight::Bold, false, 1.0, None, true),
        "cctv" => (Weight::Semibold, false, 1.0, Some(Rgba::new(0.02, 0.02, 0.02)), false),
        _ => (Weight::Heavy, true, 1.18, Some(Rgba::new(0.05, 0.05, 0.05)), false),
    }
}

/// Date stamp + camera OSD, drawn like the camera burns it into the recording.
fn draw_osd(p: &Params, ctx: &Ctx, w: usize, h: usize) -> Option<(OverlayCanvas, bool)> {
    let date_on = p.bool_or("dateOn", true);
    let osd_on = p.bool("osdOn");
    if !date_on && !osd_on {
        return None;
    }
    let mut ov = OverlayCanvas::new(w, h);
    let (wf, hf) = (w as f64, h as f64);
    let margin = wf.min(hf) * 0.065;
    let style = p.choice_or("dateStyle", "camcorder");
    let (weight, shadow, stretch, outline, glow) = text_style(&style);
    let color = p.color_or("dateColor", Rgba::WHITE);
    let size = ctx.px(p.num("dateSize", 30.0));
    let custom = crate::overlay::font_param(p);
    let font = custom.as_deref().or(if style == "led" { Some("Menlo-Bold") } else { None });
    let mut put = |ov: &mut OverlayCanvas, s: &str, x: f64, y: f64, size: f64, c: Rgba| {
        ov.text(s, x, y, size, c, weight, true, shadow, None, 0.0, font, stretch, outline, 0.04);
    };
    if date_on {
        let lines: Vec<String> = if p.choice_or("dateSource", "photo") == "custom" {
            p.text_or("dateText", "AUG 14 1998\n10:24:07 PM").split('\n').map(|s| s.to_string()).collect()
        } else {
            let d = ctx.taken.unwrap_or_else(DateTaken::now);
            date_lines(&d, &p.choice_or("dateFormat", "us"), p.bool_or("showTime", true), p.bool("timeFirst"))
        };
        let lh = size * 1.18;
        let pos = p.choice_or("datePos", "br");
        let block_h = lines.len() as f64 * lh;
        let mut y = if pos.starts_with('t') { margin } else { hf - margin - block_h };
        for l in &lines {
            let (tw, _, _) = measure(l, size as f32, true, weight, font, stretch as f32, 0.04);
            let x = if pos.ends_with('r') { wf - margin - tw as f64 } else { margin };
            put(&mut ov, l, x, y, size, color);
            y += lh;
        }
    }
    if osd_on {
        let s2 = size * 0.85;
        if p.bool_or("rec", true) {
            let r = s2 * 0.32;
            ov.set_fill(hex("#FF2A2A"), 1.0);
            ov.fill_ellipse(margin, margin + s2 * 0.25, r * 2.0, r * 2.0);
            put(&mut ov, "REC", margin + r * 2.0 + s2 * 0.35, margin, s2, color);
        }
        let mut y = margin;
        if p.bool_or("battery", true) {
            let (bw, bh) = (s2 * 1.5, s2 * 0.7);
            let x = wf - margin - bw;
            ov.set_stroke(color, 1.0);
            ov.set_line_width((s2 * 0.08).max(1.0));
            ov.stroke_rect(x, y + s2 * 0.15, bw, bh);
            ov.set_fill(color, 1.0);
            ov.fill_rect(x - s2 * 0.12, y + s2 * 0.15 + bh * 0.3, s2 * 0.12, bh * 0.4);
            for i in 0..3 {
                ov.fill_rect(x + bw - (i as f64 + 1.0) * bw * 0.3 + bw * 0.02, y + s2 * 0.15 + bh * 0.18, bw * 0.24, bh * 0.64);
            }
            y += s2 * 1.25;
        }
        if p.bool_or("counter", true) {
            let t = p.text_or("counterText", "SP  0:12:34");
            let (tw, _, _) = measure(&t, s2 as f32, true, weight, font, stretch as f32, 0.04);
            put(&mut ov, &t, wf - margin - tw as f64, y, s2, color);
        }
    }
    Some((ov, glow))
}

fn sub_effect(id: &str, input: &T, ctx: &mut Ctx, set: &[(&str, ParamValue)]) -> T {
    let Some(e) = crate::effects::effect(id) else { return input.clone() };
    let mut n = LayerNode::effect(id, None, None);
    for (k, v) in set {
        n.params.set(k, v.clone());
    }
    (e.apply)(input, &n.params, ctx)
}

pub fn camcorder() -> Effect {
    Effect::new("camcorder", "Camcorder", Category::Glitch, |input, p, ctx| {
        let (w, h) = (input.width(), input.height());
        let seed = p.fd("seed", 1.0) * 5.17;
        let mut img = input.clone();
        // 1. lens
        if p.bool_or("lensOn", true) {
            let soft = ctx.blur(input, ctx.px(3.0));
            img = ctx.apply("k_camlens", &[&img, &soft], &[
                p.fd("barrel", 15.0) / 100.0, p.fd("ca", 20.0) / 100.0, p.fd("vignette", 30.0) / 100.0,
                p.fd("cornerSoft", 20.0) / 100.0, p.fd("vigRound", 70.0) / 100.0,
            ]);
        }
        // 2. sensor at camera resolution
        let lines = p.num("resolution", 400.0).min(h as f64).max(60.0);
        let cam_h = lines.round() as usize;
        let cam_w = ((cam_h as f64 * w as f64 / h as f64 * p.num("hDetail", 80.0) / 100.0).round() as usize).max(16);
        let mut cam = ctx.resize(&img, cam_w, cam_h, false);
        let unit = (cam_h as f64 / 480.0) as f32;
        let bm = ctx.apply("k_brightmask", &[&cam], &[]);
        let bright = ctx.blur(&bm, (unit as f64 * 4.0).max(1.0));
        let noise = if p.bool_or("noiseOn", true) { p.fd("noise", 25.0) / 100.0 } else { 0.0 };
        cam = ctx.apply("k_digicam", &[&cam, &bright], &[
            seed, unit, p.fd("wb", -10.0) / 100.0, 1.0, p.fd("contrast", 110.0) / 100.0, p.fd("clip", 55.0) / 100.0,
            p.fd("fringe", 20.0) / 100.0, noise, noise * 0.6, p.fd("exposure", 0.15),
        ]);
        // 3. colour science
        let night = if p.bool("nightOn") { p.fd("nightAmt", 100.0) / 100.0 } else { 0.0 };
        let nt = p.color_or("nightTint", hex("#8CFFD2"));
        let tint = p.color_or("cast", Rgba::WHITE);
        cam = ctx.apply("k_camcolor", &[&cam], &[
            night, nt.r as f32, nt.g as f32, nt.b as f32, p.fd("blackLift", 3.0) / 100.0, p.fd("yellowPush", 40.0) / 100.0,
            p.fd("coolShadows", 30.0) / 100.0, p.fd("saturation", 110.0) / 100.0, p.fd("gamma", 100.0) / 100.0,
            tint.r as f32 * 0.4 + 0.6, tint.g as f32 * 0.4 + 0.6, tint.b as f32 * 0.4 + 0.6,
        ]);
        // 4. edge enhancement (the white outlines of home video)
        if p.bool_or("sharpOn", true) {
            let b1 = ctx.blur(&cam, 0.9);
            let b2 = ctx.blur(&cam, 2.6);
            cam = ctx.apply("k_camedge", &[&cam, &b1, &b2], &[
                p.fd("sharpen", 120.0) / 100.0, p.fd("ringing", 80.0) / 100.0, p.fd("edgeFringe", 35.0) / 100.0, 1.2,
                p.fd("halo", 40.0) / 100.0,
            ]);
        }
        // 5. chroma at low resolution, a little late
        if p.bool_or("chromaOn", true) {
            cam = ctx.apply("k_camchroma", &[&cam], &[
                p.fd("bleed", 8.0) * unit.max(0.3), p.fd("delay", 1.5) * unit.max(0.3), p.fd("chromaNoise", 20.0) / 100.0, seed,
                p.fd("lumaSoft", 0.8),
            ]);
        }
        // 6. digital compression (DV / DVR / phone)
        if p.bool("dctOn") {
            cam = super::digital::jpeg_roundtrip(ctx, &cam, p.num("quality", 35.0) / 100.0, 1);
        }
        // 7. interlace combing
        if p.bool("interlaceOn") {
            cam = ctx.apply("k_interlace", &[&cam], &[p.fd("fieldShift", 2.0), p.fd("fieldDark", 6.0) / 100.0, p.fd("fieldJitter", 1.0), seed, 1.0]);
        }
        img = ctx.resize(&cam, w, h, true);
        // 8. date stamp / OSD, burned in (slightly soft, like everything else on tape)
        if let Some((ov, glow)) = draw_osd(p, ctx, w, h) {
            let t = ov.texture();
            let mut t = ctx.apply("k_copy", &[&t], &[]);
            let soft = h as f64 / cam_h as f64 * 0.35;
            if soft > 0.3 {
                t = ctx.blur(&t, soft);
            }
            if glow {
                let g = ctx.blur(&t, ctx.px(p.num("dateSize", 30.0)) * 0.12);
                img = ctx.apply("k_over_pm_op", &[&img, &g], &[0.6]);
            }
            img = ctx.apply("k_over_pm_op", &[&img, &t], &[0.95]);
        }
        // 9. tape transport
        if p.bool("tapeOn") {
            let n = |v: f64| ParamValue::Number(v);
            let b = ParamValue::Bool;
            img = sub_effect("vhs", &img, ctx, &[
                ("soft", b(false)), ("colorOn", b(false)), ("scanlines", b(false)), ("vignette", b(false)), ("osd", b(false)),
                ("tracking", b(p.bool("tracking"))), ("trackAmt", n(p.num("trackAmt", 5.0))), ("bandPos", n(p.num("bandPos", 82.0))),
                ("bandH", n(p.num("bandH", 5.0))), ("headSwitch", b(p.bool_or("headSwitch", true))), ("headH", n(p.num("headH", 2.0))),
                ("wobble", b(p.num("wobbleAmt", 0.8) > 0.01)), ("wobbleAmt", n(p.num("wobbleAmt", 0.8))), ("noise", b(true)),
                ("noiseAmt", n(p.num("tapeNoise", 25.0))), ("dropouts", b(p.num("dropouts", 10.0) > 0.5)), ("dropAmt", n(p.num("dropouts", 10.0))),
                ("seed", n(p.num("seed", 1.0))),
            ]);
        }
        // 10. watched on a CRT and photographed
        if p.bool("screenOn") {
            let n = |v: f64| ParamValue::Number(v);
            img = sub_effect("crt", &img, ctx, &[
                ("curvature", n(p.num("crtCurve", 35.0))), ("scan", n(p.num("crtScan", 40.0))), ("glow", n(p.num("crtGlow", 45.0))),
                ("bezel", ParamValue::Bool(p.bool_or("crtBezel", true))),
            ]);
        }
        img
    })
    .symbol("video")
    .summary("Home video from the 90s and 2000s: camcorder lens, CCD colour, white edge outlines, chroma bleed, NightShot, tape and date stamp. Start from a Look.")
    .params({
        let mut v = vec![
            // camera
            s("resolution", "Resolution (lines)", 60.0, 1080.0, 400.0).step(1.0).curve(1.4).sec("Camera"),
            s("hDetail", "Horizontal Detail", 30.0, 100.0, 80.0).step(1.0).unit("%").sec("Camera"),
            s("exposure", "Exposure", -2.0, 2.0, 0.15).step(0.05).unit(" EV").sec("Camera"),
            s("wb", "White Balance", -100.0, 100.0, -10.0).step(1.0).sec("Camera").track(vec![hex("#3A7BFF"), hex("#9A9A9A"), hex("#FFB020")]),
            s("contrast", "Contrast", 50.0, 200.0, 110.0).step(1.0).unit("%").sec("Camera"),
            s("clip", "Blown Highlights", 0.0, 100.0, 55.0).step(1.0).unit("%").sec("Camera"),
            s("fringe", "Purple Fringing", 0.0, 100.0, 20.0).step(1.0).unit("%").sec("Camera"),
            // colour
            s("saturation", "Saturation", 0.0, 200.0, 110.0).step(1.0).unit("%").sec("Color"),
            s("yellowPush", "Warm Skin (CCD)", 0.0, 100.0, 40.0).step(1.0).unit("%").sec("Color"),
            s("coolShadows", "Cool Shadows", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Color"),
            s("blackLift", "Black Lift", 0.0, 30.0, 3.0).step(0.5).unit("%").sec("Color"),
            s("gamma", "Gamma", 50.0, 200.0, 100.0).step(1.0).unit("%").sec("Color"),
            ParamDef::color("cast", "Color Cast", Rgba::WHITE).sec("Color"),
            ParamDef::section_switch("nightOn", "NightShot", false),
            s("nightAmt", "Amount", 0.0, 100.0, 100.0).step(1.0).unit("%").sec("NightShot"),
            ParamDef::color("nightTint", "Tint", hex("#8CFFD2")).sec("NightShot"),
            // lens
            ParamDef::section_switch("lensOn", "Lens", true),
            s("barrel", "Wide Converter", -60.0, 100.0, 15.0).step(1.0).unit("%").sec("Lens"),
            s("ca", "Color Fringes", 0.0, 100.0, 20.0).step(1.0).unit("%").sec("Lens"),
            s("cornerSoft", "Soft Corners", 0.0, 100.0, 20.0).step(1.0).unit("%").sec("Lens"),
            s("vignette", "Vignette", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Lens"),
            s("vigRound", "Vignette Roundness", 0.0, 100.0, 70.0).step(1.0).unit("%").sec("Lens"),
            // sharpening
            ParamDef::section_switch("sharpOn", "Edge Enhance", true),
            s("sharpen", "Sharpening", 0.0, 300.0, 120.0).step(1.0).unit("%").curve(1.4).sec("Edge Enhance"),
            s("ringing", "Halo Outlines", 0.0, 300.0, 80.0).step(1.0).unit("%").curve(1.4).sec("Edge Enhance"),
            s("halo", "White Halos", 0.0, 100.0, 40.0).step(1.0).unit("%").sec("Edge Enhance"),
            s("edgeFringe", "Edge Color Fringe", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Edge Enhance"),
            // chroma
            ParamDef::section_switch("chromaOn", "Chroma", true),
            s("bleed", "Color Bleed", 0.0, 40.0, 8.0).step(0.5).curve(1.4).sec("Chroma"),
            s("delay", "Color Delay", -10.0, 10.0, 1.5).step(0.25).sec("Chroma"),
            s("chromaNoise", "Chroma Noise", 0.0, 100.0, 20.0).step(1.0).unit("%").sec("Chroma"),
            s("lumaSoft", "Luma Softness", 0.0, 4.0, 0.8).step(0.1).sec("Chroma"),
            // noise
            ParamDef::section_switch("noiseOn", "Sensor Noise", true),
            s("noise", "Amount", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Sensor Noise"),
            // digital
            ParamDef::section_switch("dctOn", "Compression", false),
            s("quality", "Quality", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Compression"),
            ParamDef::section_switch("interlaceOn", "Interlace", false),
            s("fieldShift", "Field Shift", 0.0, 12.0, 2.0).step(0.25).sec("Interlace"),
            s("fieldDark", "Field Darkening", 0.0, 50.0, 6.0).step(1.0).unit("%").sec("Interlace"),
            s("fieldJitter", "Jitter", 0.0, 10.0, 1.0).step(0.25).sec("Interlace"),
            // tape
            ParamDef::section_switch("tapeOn", "Tape", false),
            s("tapeNoise", "Tape Noise", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Tape"),
            s("dropouts", "Dropouts", 0.0, 100.0, 10.0).step(1.0).unit("%").sec("Tape"),
            ParamDef::toggle("headSwitch", "Head Switching", true).sec("Tape"),
            s("headH", "Head Switch Height", 0.5, 10.0, 2.0).step(0.1).unit("%").sec("Tape").vis(when_on("headSwitch")),
            s("wobbleAmt", "Wobble", 0.0, 10.0, 0.8).step(0.1).curve(1.6).sec("Tape"),
            ParamDef::toggle("tracking", "Tracking Errors", false).sec("Tape"),
            s("trackAmt", "Tracking Jitter", 0.0, 30.0, 5.0).step(0.5).curve(1.5).sec("Tape").vis(when_on("tracking")),
            s("bandPos", "Band Position", 0.0, 100.0, 82.0).step(1.0).unit("%").sec("Tape").vis(when_on("tracking")),
            s("bandH", "Band Height", 0.0, 30.0, 5.0).step(0.5).unit("%").sec("Tape").vis(when_on("tracking")),
            // screen
            ParamDef::section_switch("screenOn", "Filmed off a TV", false),
            s("crtCurve", "Curvature", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Filmed off a TV"),
            s("crtScan", "Scanlines", 0.0, 100.0, 40.0).step(1.0).unit("%").sec("Filmed off a TV"),
            s("crtGlow", "Glow", 0.0, 100.0, 45.0).step(1.0).unit("%").sec("Filmed off a TV"),
            ParamDef::toggle("crtBezel", "Black Border", true).sec("Filmed off a TV"),
            // date
            ParamDef::section_switch("dateOn", "Date Stamp", true),
            ParamDef::segmented("dateSource", "Date", &[("photo", "Photo's Date"), ("custom", "Custom Text")], "photo").sec("Date Stamp"),
            ParamDef::text("dateText", "Text", "AUG 14 1998\n10:24:07 PM").sec("Date Stamp").vis(when("dateSource", "custom")),
            ParamDef::choice("dateFormat", "Format", &[
                ("us", "AUG 14 1998 · 10:24 PM"), ("eu", "14. 8. 1998 · 13:59"), ("iso", "1998.08.14 · 16:20"), ("digicam", "'98 8 14"), ("cctv", "08-14-1998 22:24:07"),
            ], "us").sec("Date Stamp").vis(when("dateSource", "photo")),
            ParamDef::toggle("showTime", "Show Time", true).sec("Date Stamp").vis(when("dateSource", "photo")),
            ParamDef::toggle("timeFirst", "Time Above Date", false).sec("Date Stamp").vis(when("dateSource", "photo")),
            ParamDef::choice("datePos", "Position", &[("br", "Bottom Right"), ("bl", "Bottom Left"), ("tr", "Top Right"), ("tl", "Top Left")], "br").sec("Date Stamp"),
            ParamDef::choice("dateStyle", "Style", &[("camcorder", "Camcorder (outlined)"), ("vcr", "VCR (flat)"), ("led", "Digicam LED"), ("cctv", "CCTV")], "camcorder").sec("Date Stamp"),
            s("dateSize", "Size", 8.0, 80.0, 30.0).step(1.0).sec("Date Stamp"),
            crate::overlay::font_def("Date Stamp"),
            ParamDef::color("dateColor", "Color", Rgba::WHITE).sec("Date Stamp"),
            // osd
            ParamDef::section_switch("osdOn", "Camera Display", false),
            ParamDef::toggle("rec", "● REC", true).sec("Camera Display"),
            ParamDef::toggle("battery", "Battery", true).sec("Camera Display"),
            ParamDef::toggle("counter", "Tape Counter", true).sec("Camera Display"),
            ParamDef::text("counterText", "Counter Text", "SP  0:12:34").sec("Camera Display").vis(when_on("counter")),
            ParamDef::seed("seed").sec("Camera"),
        ];
        v.extend(depth_bias_params());
        v
    })
    .looks(camcorder_looks())
}

fn camcorder_looks() -> Vec<Look> {
    let t = |s: &str| ParamValue::Text(s.into());
    vec![
        look_g("handycam", "Handycam 2003", "Camcorder", vals![
            "resolution" => 480.0, "barrel" => 55.0, "ca" => 35.0, "vignette" => 45.0, "cornerSoft" => 35.0, "sharpen" => 160.0, "ringing" => 120.0,
            "halo" => 55.0, "edgeFringe" => 55.0, "yellowPush" => 65.0, "saturation" => 118.0, "clip" => 70.0, "exposure" => 0.3, "dateOn" => false
        ]),
        look_g("home96", "Home Video '96", "Camcorder", vals![
            "resolution" => 300.0, "hDetail" => 65.0, "barrel" => 10.0, "sharpen" => 90.0, "ringing" => 60.0, "bleed" => 16.0, "delay" => 3.0,
            "chromaNoise" => 35.0, "saturation" => 120.0, "yellowPush" => 50.0, "blackLift" => 6.0, "tapeOn" => true, "tapeNoise" => 35.0,
            "dropouts" => 15.0, "dateFormat" => "eu", "timeFirst" => true, "datePos" => "bl", "dateSize" => 38.0
        ]),
        look_g("hi8", "Hi8 Vacation", "Camcorder", vals![
            "resolution" => 400.0, "wb" => 15.0, "saturation" => 125.0, "yellowPush" => 55.0, "sharpen" => 110.0, "ringing" => 70.0, "tapeOn" => true,
            "tapeNoise" => 18.0, "dropouts" => 5.0, "headSwitch" => false, "dateFormat" => "eu", "showTime" => false
        ]),
        look_g("nightshot", "NightShot", "Camcorder", vals![
            "nightOn" => true, "resolution" => 360.0, "exposure" => 0.6, "clip" => 80.0, "noise" => 70.0, "chromaNoise" => 10.0, "sharpen" => 70.0,
            "ringing" => 50.0, "barrel" => 25.0, "vignette" => 55.0, "dateOn" => false, "osdOn" => true, "battery" => false, "counter" => false
        ]),
        look_g("minidv", "MiniDV 2004", "Digital Video", vals![
            "resolution" => 576.0, "hDetail" => 100.0, "wb" => -30.0, "coolShadows" => 55.0, "yellowPush" => 25.0, "sharpen" => 140.0, "ringing" => 90.0,
            "halo" => 45.0, "bleed" => 4.0, "delay" => 0.5, "dctOn" => true, "quality" => 70.0, "interlaceOn" => true, "fieldShift" => 1.5,
            "dateFormat" => "iso", "dateStyle" => "vcr", "dateSize" => 26.0
        ]),
        look_g("cctv", "CCTV DVR", "Digital Video", vals![
            "resolution" => 240.0, "hDetail" => 70.0, "saturation" => 55.0, "wb" => 20.0, "cast" => hex("#C8B48C"), "contrast" => 120.0,
            "clip" => 75.0, "sharpen" => 90.0, "ringing" => 110.0, "halo" => 60.0, "edgeFringe" => 60.0, "noise" => 45.0, "dctOn" => true,
            "quality" => 18.0, "interlaceOn" => true, "fieldShift" => 3.0, "fieldJitter" => 2.0, "barrel" => 35.0, "dateFormat" => "cctv",
            "dateStyle" => "cctv", "datePos" => "tl", "dateSize" => 22.0
        ]),
        look_g("tvphoto", "TV Off the Screen", "Screens", vals![
            "resolution" => 260.0, "sharpen" => 240.0, "ringing" => 260.0, "halo" => 90.0, "edgeFringe" => 90.0, "saturation" => 140.0, "contrast" => 130.0,
            "clip" => 85.0, "bleed" => 20.0, "screenOn" => true, "crtCurve" => 55.0, "crtScan" => 30.0, "crtGlow" => 70.0, "dateOn" => false,
            "lensOn" => false
        ]),
        look_g("vhsrental", "Rental VHS", "Tape", vals![
            "resolution" => 240.0, "hDetail" => 55.0, "sharpen" => 70.0, "ringing" => 50.0, "bleed" => 22.0, "delay" => 4.0, "chromaNoise" => 45.0,
            "saturation" => 90.0, "blackLift" => 8.0, "tapeOn" => true, "tapeNoise" => 55.0, "dropouts" => 35.0, "tracking" => true, "lensOn" => false,
            "dateOn" => false, "osdOn" => true, "rec" => false, "battery" => false, "counterText" => t("PLAY ▶")
        ]),
        look_g("dubbed", "Dubbed 3rd Generation", "Tape", vals![
            "resolution" => 200.0, "hDetail" => 50.0, "sharpen" => 150.0, "ringing" => 140.0, "halo" => 60.0, "bleed" => 30.0, "delay" => 6.0,
            "chromaNoise" => 60.0, "saturation" => 80.0, "blackLift" => 12.0, "tapeOn" => true, "tapeNoise" => 65.0, "dropouts" => 45.0,
            "tracking" => true, "trackAmt" => 7.0, "wobbleAmt" => 2.5, "lensOn" => false, "dateOn" => false
        ]),
        look_g("digicam", "Digicam 2003", "Stills", vals![
            "resolution" => 600.0, "hDetail" => 100.0, "wb" => -25.0, "saturation" => 120.0, "contrast" => 118.0, "clip" => 50.0, "fringe" => 45.0,
            "sharpen" => 120.0, "ringing" => 30.0, "edgeFringe" => 10.0, "chromaOn" => false, "dctOn" => true, "quality" => 55.0, "barrel" => 5.0,
            "dateFormat" => "digicam", "dateStyle" => "led", "dateColor" => hex("#FF8A1E"), "dateSize" => 28.0, "showTime" => false
        ]),
        look_g("flipphone", "Flip Phone VGA", "Stills", vals![
            "resolution" => 240.0, "hDetail" => 100.0, "saturation" => 90.0, "noise" => 60.0, "sharpen" => 60.0, "ringing" => 20.0, "chromaOn" => false,
            "dctOn" => true, "quality" => 22.0, "barrel" => 0.0, "vignette" => 25.0, "dateOn" => false
        ]),
        look_g("webcam", "Webcam 2002", "Stills", vals![
            "resolution" => 180.0, "hDetail" => 100.0, "wb" => 25.0, "saturation" => 80.0, "noise" => 70.0, "chromaNoise" => 50.0, "clip" => 65.0,
            "dctOn" => true, "quality" => 15.0, "sharpen" => 40.0, "dateOn" => false
        ]),
    ]
}
