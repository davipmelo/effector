use std::collections::HashMap;
use std::sync::Mutex;

use super::*;
use crate::swatches;

pub fn gradient_map() -> Effect {
    Effect::new("gradientmap", "Gradient Map", Category::Adjust, |input, p, ctx| {
        let g = p.gradient("gradient").unwrap_or_else(swatches::thermal_iron);
        let lut: Vec<[f32; 4]> = (0..256)
            .map(|i| {
                let c = g.sample(i as f64 / 255.0);
                [c.r as f32, c.g as f32, c.b as f32, 1.0]
            })
            .collect();
        let lut = ctx.gpu().lut1d(&lut);
        ctx.apply(
            "k_gradmap",
            &[input, &lut],
            &[p.fd("amount", 100.0) / 100.0, p.f("black") / 100.0, p.fd("white", 100.0) / 100.0, p.b("reverse")],
        )
    })
    .symbol("thermometer.medium")
    .summary("Maps brightness to a gradient: duotones, thermal, night vision.")
    .params(vec![
        ParamDef::gradient("gradient", "Gradient", swatches::thermal_iron()).sec("Gradient"),
        s("amount", "Amount", 0.0, 100.0, 100.0).step(1.0).unit("%").sec("Gradient"),
        s("black", "Input Black", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Range"),
        s("white", "Input White", 0.0, 100.0, 100.0).step(1.0).unit("%").sec("Range"),
        ParamDef::toggle("reverse", "Reverse", false).sec("Range"),
    ])
}

/// Cache of 3D LUT textures (parsing/uploading is expensive).
fn lut_texture(lut: &LutData) -> T {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<usize, T>>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = std::sync::Arc::as_ptr(&lut.values) as usize ^ lut.size;
    let mut c = cache.lock().unwrap();
    if let Some(t) = c.get(&key) {
        return t.clone();
    }
    let n = lut.size;
    let mut rgba = vec![1f32; n * n * n * 4];
    for i in 0..n * n * n {
        rgba[i * 4] = lut.values[i * 3];
        rgba[i * 4 + 1] = lut.values[i * 3 + 1];
        rgba[i * 4 + 2] = lut.values[i * 3 + 2];
    }
    let t = crate::gpu::gpu().texture_from_rgba_f32(&rgba, n, n, n);
    if c.len() > 8 {
        c.clear();
    }
    c.insert(key, t.clone());
    t
}

pub fn lut() -> Effect {
    Effect::new("lut", "LUT (.cube)", Category::Adjust, |input, p, ctx| {
        let Some(l) = p.lut("lut").filter(|l| !l.is_empty()) else { return input.clone() };
        let t = lut_texture(&l);
        ctx.apply("k_lut3d", &[input, &t], &[p.fd("amount", 100.0) / 100.0])
    })
    .symbol("cube")
    .summary("Apply a .cube colour LUT. Drop .cube files on the layer or the canvas.")
    .params(vec![ParamDef::lut("lut", "LUT"), s("amount", "Amount", 0.0, 100.0, 100.0).step(1.0).unit("%")])
}

/// Parses an Adobe/Resolve `.cube` 3D LUT (with DOMAIN_MIN/MAX normalisation).
pub fn parse_cube(name: &str, text: &str) -> Option<LutData> {
    let mut size = 0usize;
    let mut vals: Vec<f32> = vec![];
    let (mut dmin, mut dmax) = (vec![0f32; 3], vec![1f32; 3]);
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split([' ', '\t']).filter(|s| !s.is_empty()).collect();
        if line.starts_with("LUT_3D_SIZE") {
            size = parts.last().and_then(|x| x.parse().ok()).unwrap_or(0);
            continue;
        }
        if line.starts_with("DOMAIN_MIN") {
            dmin = parts[1..].iter().filter_map(|x| x.parse().ok()).collect();
            continue;
        }
        if line.starts_with("DOMAIN_MAX") {
            dmax = parts[1..].iter().filter_map(|x| x.parse().ok()).collect();
            continue;
        }
        if line.starts_with("TITLE") || line.starts_with("LUT_1D") || line.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
            continue;
        }
        if parts.len() >= 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>(), parts[2].parse::<f32>()) {
                vals.extend([r, g, b]);
            }
        }
    }
    if size < 2 || vals.len() != size * size * size * 3 {
        return None;
    }
    if dmin.len() == 3 && dmax.len() == 3 && (dmin != [0.0, 0.0, 0.0] || dmax != [1.0, 1.0, 1.0]) {
        for i in (0..vals.len()).step_by(3) {
            for c in 0..3 {
                vals[i + c] = (vals[i + c] - dmin[c]) / (dmax[c] - dmin[c]).max(1e-6);
            }
        }
    }
    Some(LutData { name: name.into(), size, values: std::sync::Arc::new(vals) })
}

/// Writes a LUT back as `.cube` text.
pub fn cube_text(l: &LutData) -> String {
    let mut t = format!("TITLE \"{}\"\nLUT_3D_SIZE {}\n", l.name, l.size);
    for i in (0..l.values.len()).step_by(3) {
        t += &format!("{:.6} {:.6} {:.6}\n", l.values[i], l.values[i + 1], l.values[i + 2]);
    }
    t
}

/// Lightroom-style colour grading: hue/saturation wheels and luminance for shadows, midtones,
/// highlights and the whole image, with Blending and Balance. The "Split Tone" mode (the old
/// Split Tone effect, merged in 0.8.1) tints shadows and highlights with two colours.
pub fn color_grade() -> Effect {
    let zones: [(&'static str, &'static str); 4] = [("sh", "Shadows"), ("mid", "Midtones"), ("hi", "Highlights"), ("gl", "Global")];
    // older projects have no "mode": they are colour wheels
    let wheels = || when_not("mode", "split");
    let split = || when("mode", "split");
    let mut params = vec![
        ParamDef::segmented("mode", "Mode", &[("wheels", "Color Wheels"), ("split", "Split Tone")], "wheels").sec("Color Grading"),
        ParamDef::segmented("view", "View", &[("all", "3-Way"), ("sh", "Shad."), ("mid", "Mids"), ("hi", "Highs"), ("gl", "Global")], "all").sec("Color Grading").vis(wheels()),
        ParamDef::wheel_row("wheels", &["sh", "mid", "hi"]).sec("Color Grading").vis(when_all(vec![wheels(), when("view", "all")])),
        ParamDef::wheel_row("glWheels", &["gl"]).sec("Color Grading").vis(when_all(vec![wheels(), when("view", "all")])),
        ParamDef::color("shadow", "Shadow Color", hex("#1F4E9E")).sec("Color Grading").vis(split()),
        s("shadowAmt", "Shadow Amount", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Color Grading").vis(split()),
        ParamDef::color("highlight", "Highlight Color", hex("#FFB46E")).sec("Color Grading").vis(split()),
        s("highlightAmt", "Highlight Amount", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Color Grading").vis(split()),
        ParamDef::toggle("preserve", "Preserve Brightness", true).sec("Color Grading").vis(split()),
    ];
    let hue_track: Vec<Rgba> = (0..13).map(|i| oklch_rgb(0.72, 0.13, i as f64 * 30.0)).collect();
    for (z, label) in zones {
        let v = when_all(vec![wheels(), when("view", z)]);
        params.push(ParamDef::wheel(&format!("{z}Wheel"), label, z).sec("Color Grading").vis(v.clone()));
        params.push(s(&format!("{z}Hue"), "Hue", 0.0, 360.0, 0.0).step(1.0).unit("°").sec("Color Grading").track(hue_track.clone()).vis(v.clone()));
        params.push(s(&format!("{z}Sat"), "Saturation", 0.0, 100.0, 0.0).step(1.0).sec("Color Grading").vis(v.clone()));
        params.push(s(&format!("{z}Lum"), "Luminance", -100.0, 100.0, 0.0).step(1.0).sec("Color Grading").vis(v));
    }
    params.push(s("blending", "Blending", 0.0, 100.0, 50.0).step(1.0).sec("Color Grading").vis(wheels()));
    params.push(s("balance", "Balance", -100.0, 100.0, 0.0).step(1.0).sec("Color Grading").track(vec![hex("#3A3A3A"), hex("#9A9A9A"), hex("#E8E8E8")]));
    Effect::new("colorgrade", "Color Grading", Category::Adjust, |input, p, ctx| {
        if p.choice("mode") == "split" {
            let mut u = vec![];
            u.extend(p.color_or("shadow", hex("#1F4E9E")).floats());
            u.extend(p.color_or("highlight", hex("#FFB46E")).floats());
            u.extend([p.fd("shadowAmt", 50.0) / 100.0, p.fd("highlightAmt", 30.0) / 100.0, p.f("balance") / 100.0, p.b("preserve")]);
            return ctx.apply("k_splittone", &[input], &u);
        }
        let mut u = vec![];
        for z in ["sh", "mid", "hi", "gl"] {
            u.push(p.f(&format!("{z}Hue")).to_radians());
            u.push(p.f(&format!("{z}Sat")) / 100.0);
            u.push(p.f(&format!("{z}Lum")) / 100.0);
        }
        u.push(p.fd("blending", 50.0) / 100.0);
        u.push(p.f("balance") / 100.0);
        ctx.apply("k_colorgrade", &[input], &u)
    })
    .symbol("circle.lefthalf.filled")
    .summary("Color wheels for shadows, midtones, highlights and global, like Lightroom's Color Grading — or a two-colour Split Tone.")
    .params(params)
    .depth_bias()
    .looks({
        let g = |pairs: &[(&str, f64)]| -> Vec<(String, ParamValue)> { pairs.iter().map(|(k, v)| (k.to_string(), ParamValue::Number(*v))).collect() };
        vec![
            look_g("tealorange", "Teal & Orange", "Cinema", g(&[("shHue", 220.0), ("shSat", 45.0), ("hiHue", 65.0), ("hiSat", 35.0), ("midHue", 60.0), ("midSat", 8.0)])),
            look_g("bleach", "Cold Thriller", "Cinema", g(&[("shHue", 200.0), ("shSat", 35.0), ("midHue", 180.0), ("midSat", 15.0), ("hiHue", 150.0), ("hiSat", 10.0), ("shLum", -10.0)])),
            look_g("golden", "Golden Hour", "Warm", g(&[("hiHue", 75.0), ("hiSat", 45.0), ("midHue", 55.0), ("midSat", 20.0), ("shHue", 30.0), ("shSat", 15.0)])),
            look_g("kodak", "Warm Print", "Warm", g(&[("shHue", 300.0), ("shSat", 12.0), ("midHue", 70.0), ("midSat", 15.0), ("hiHue", 90.0), ("hiSat", 25.0), ("shLum", 8.0)])),
            look_g("matrix", "Green Code", "Stylized", g(&[("shHue", 150.0), ("shSat", 40.0), ("midHue", 135.0), ("midSat", 25.0), ("hiHue", 110.0), ("hiSat", 15.0)])),
            look_g("vapor", "Vaporwave", "Stylized", g(&[("shHue", 285.0), ("shSat", 55.0), ("hiHue", 200.0), ("hiSat", 40.0), ("midHue", 330.0), ("midSat", 20.0)])),
            look_g("nightblue", "Blue Night", "Cool", g(&[("shHue", 260.0), ("shSat", 50.0), ("midHue", 250.0), ("midSat", 25.0), ("hiHue", 220.0), ("hiSat", 10.0), ("midLum", -8.0)])),
            look_g("faded", "Faded Pastel", "Cool", g(&[("shHue", 230.0), ("shSat", 20.0), ("shLum", 18.0), ("hiHue", 20.0), ("hiSat", 15.0), ("hiLum", -6.0), ("blending", 80.0)])),
            look_g("split", "Classic Split Tone", "Classic", g(&[("shHue", 262.0), ("shSat", 40.0), ("hiHue", 65.0), ("hiSat", 30.0)])),
            look_g("sepia", "Sepia Wash", "Classic", g(&[("glHue", 65.0), ("glSat", 30.0), ("shHue", 40.0), ("shSat", 15.0)])),
            look_g("stcool", "Cool Shadows, Warm Highlights", "Split Tone", vals!["mode" => "split"]),
            look_g("stteal", "Teal & Cream", "Split Tone", vals!["mode" => "split", "shadow" => hex("#0F3B36"), "shadowAmt" => 55.0, "highlight" => hex("#F6E7CF"), "highlightAmt" => 18.0]),
            look_g("stnavy", "Navy & Peach", "Split Tone", vals!["mode" => "split", "shadow" => hex("#0E3A66"), "shadowAmt" => 50.0, "highlight" => hex("#FFC9A0"), "highlightAmt" => 15.0]),
            look_g("stforest", "Forest & Sage", "Split Tone", vals!["mode" => "split", "shadow" => hex("#12281A"), "shadowAmt" => 60.0, "highlight" => hex("#D9E9C2"), "highlightAmt" => 25.0]),
            look_g("stember", "Ember", "Split Tone", vals!["mode" => "split", "shadow" => hex("#0B2A33"), "shadowAmt" => 45.0, "highlight" => hex("#FF7A2A"), "highlightAmt" => 50.0]),
        ]
    })
}

/// sRGB colour of an Oklch value (for UI swatches; hue in degrees).
pub fn oklch_rgb(l: f64, c: f64, h_deg: f64) -> Rgba {
    let h = h_deg.to_radians();
    let (a, b) = (c * h.cos(), c * h.sin());
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
    let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    let r = 4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3;
    let g = -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3;
    let bl = -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3;
    let enc = |x: f64| {
        let x = x.clamp(0.0, 1.0);
        if x <= 0.0031308 { x * 12.92 } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 }
    };
    Rgba::new(enc(r), enc(g), enc(bl))
}
