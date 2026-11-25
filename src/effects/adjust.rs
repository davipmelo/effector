use super::*;

// MARK: - Curve math (monotone cubic / Fritsch–Carlson)

pub fn curve_eval(pts: &[CurvePoint], x: f64) -> f64 {
    let mut p = pts.to_vec();
    p.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    if p.len() < 2 { return x; }
    if x <= p[0].x { return p[0].y; }
    if x >= p[p.len() - 1].x { return p[p.len() - 1].y; }
    let n = p.len();
    let mut d = vec![0.0; n - 1];
    for i in 0..n - 1 {
        let dx = (p[i + 1].x - p[i].x).max(1e-6);
        d[i] = (p[i + 1].y - p[i].y) / dx;
    }
    let mut m = vec![0.0; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    if n > 2 {
        for i in 1..n - 1 {
            m[i] = if d[i - 1] * d[i] <= 0.0 { 0.0 } else { (d[i - 1] + d[i]) / 2.0 };
        }
    }
    for i in 0..n - 1 {
        if d[i].abs() < 1e-9 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
        }
    }
    for i in 0..n - 1 {
        if d[i].abs() >= 1e-9 {
            let (a, b) = (m[i] / d[i], m[i + 1] / d[i]);
            let s = a * a + b * b;
            if s > 9.0 {
                let t = 3.0 / s.sqrt();
                m[i] = t * a * d[i];
                m[i + 1] = t * b * d[i];
            }
        }
    }
    let mut k = 0;
    while k < n - 2 && x > p[k + 1].x { k += 1; }
    let h = p[k + 1].x - p[k].x;
    let t = (x - p[k].x) / h;
    let (t2, t3) = (t * t, t * t * t);
    let y = (2.0 * t3 - 3.0 * t2 + 1.0) * p[k].y + (t3 - 2.0 * t2 + t) * h * m[k] + (-2.0 * t3 + 3.0 * t2) * p[k + 1].y
        + (t3 - t2) * h * m[k + 1];
    y.clamp(0.0, 1.0)
}

pub fn curve_lut(c: &CurveSet, count: usize) -> Vec<[f32; 4]> {
    (0..count)
        .map(|i| {
            let x = i as f64 / (count - 1) as f64;
            let r = curve_eval(&c.master, curve_eval(&c.red, x));
            let g = curve_eval(&c.master, curve_eval(&c.green, x));
            let b = curve_eval(&c.master, curve_eval(&c.blue, x));
            [r as f32, g as f32, b as f32, 1.0]
        })
        .collect()
}

// MARK: - Effects

pub fn basic() -> Effect {
    Effect::new("basic", "Basic Adjustments", Category::Adjust, |input, p, ctx| {
        let keys = ["exposure", "contrast", "highlights", "shadows", "whites", "blacks", "temperature", "tint", "vibrance", "saturation"];
        let u: Vec<f32> = keys.iter().map(|k| p.f(k)).collect();
        ctx.apply("k_basic", &[input], &u)
    })
    .symbol("slider.horizontal.3")
    .summary("Exposure, contrast, highlights, shadows, white balance and saturation.")
    .params(vec![
        s("exposure", "Exposure", -4.0, 4.0, 0.0).step(0.05).unit(" EV").sec("Tone"),
        s("contrast", "Contrast", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
        s("highlights", "Highlights", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
        s("shadows", "Shadows", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
        s("whites", "Whites", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
        s("blacks", "Blacks", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
        s("temperature", "Temperature", -100.0, 100.0, 0.0).step(1.0).sec("Color").track(vec![hex("#3A7BFF"), hex("#9A9A9A"), hex("#FFB020")]),
        s("tint", "Tint", -100.0, 100.0, 0.0).step(1.0).sec("Color").track(vec![hex("#2FBF5A"), hex("#9A9A9A"), hex("#E040C0")]),
        s("vibrance", "Vibrance", -100.0, 100.0, 0.0).step(1.0).sec("Color"),
        s("saturation", "Saturation", -100.0, 100.0, 0.0).step(1.0).sec("Color"),
    ])
}

pub fn curves() -> Effect {
    Effect::new("curves", "Curves", Category::Adjust, |input, p, ctx| {
        let lut = ctx.gpu().lut1d(&curve_lut(&p.curves("curves"), 256));
        ctx.apply("k_curves", &[input, &lut], &[])
    })
    .symbol("point.topleft.down.to.point.bottomright.curvepath")
    .summary("Tone curves for RGB and each channel.")
    .params(vec![ParamDef::curves("curves", "Curves")])
}

pub const HSL_BANDS: [&str; 8] = ["Red", "Orange", "Yellow", "Green", "Aqua", "Blue", "Purple", "Magenta"];
pub const HSL_BAND_COLORS: [&str; 8] = ["#FF3B30", "#FF9500", "#FFD60A", "#34C759", "#32D3E0", "#1E6BFF", "#8E44EF", "#FF2D95"];

pub fn hsl() -> Effect {
    let mut params = vec![];
    for (i, b) in HSL_BANDS.iter().enumerate() {
        let k = b.to_lowercase();
        let c = hex(HSL_BAND_COLORS[i]);
        let prev = hex(HSL_BAND_COLORS[(i + 7) % 8]);
        let next = hex(HSL_BAND_COLORS[(i + 1) % 8]);
        let gray = Rgba::new(c.luma(), c.luma(), c.luma());
        let dark = Rgba::new(c.r * 0.25, c.g * 0.25, c.b * 0.25);
        let light = Rgba::new(0.5 + c.r * 0.5, 0.5 + c.g * 0.5, 0.5 + c.b * 0.5);
        params.push(s(&format!("{k}_h"), "Hue", -100.0, 100.0, 0.0).step(1.0).sec(b).track(vec![prev, c, next]));
        params.push(s(&format!("{k}_s"), "Saturation", -100.0, 100.0, 0.0).step(1.0).sec(b).track(vec![gray, c]));
        params.push(s(&format!("{k}_l"), "Luminance", -100.0, 100.0, 0.0).step(1.0).sec(b).track(vec![dark, c, light]));
    }
    Effect::new("hsl", "HSL", Category::Adjust, |input, p, ctx| {
        let mut u = vec![];
        for b in HSL_BANDS {
            let k = b.to_lowercase();
            u.extend([p.f(&format!("{k}_h")), p.f(&format!("{k}_s")), p.f(&format!("{k}_l"))]);
        }
        ctx.apply("k_hsl", &[input], &u)
    })
    .symbol("circle.lefthalf.filled.righthalf.striped.horizontal")
    .summary("Shift hue, saturation and luminance of 8 colour ranges, smoothly.")
    .section_colors(HSL_BANDS.iter().zip(HSL_BAND_COLORS).map(|(b, c)| (b.to_string(), hex(c))).collect())
    .params(params)
}

pub fn huesat() -> Effect {
    Effect::new("huesat", "Hue / Saturation", Category::Adjust, |input, p, ctx| {
        ctx.apply("k_huesat", &[input], &[p.f("hue"), p.f("saturation"), p.f("lightness"), p.b("colorize"), p.f("colorHue"), p.f("colorSat")])
    })
    .symbol("drop.halffull")
    .summary("Rotate hue, change saturation and lightness, or colorize.")
    .params(vec![
        s("hue", "Hue", -180.0, 180.0, 0.0).step(1.0).unit("°").track(
            ["#00FFFF", "#0000FF", "#FF00FF", "#FF0000", "#FFFF00", "#00FF00", "#00FFFF"].iter().map(|h| hex(h)).collect(),
        ),
        s("saturation", "Saturation", -100.0, 100.0, 0.0).step(1.0).track(vec![hex("#808080"), hex("#FF3B30")]),
        s("lightness", "Lightness", -100.0, 100.0, 0.0).step(1.0).track(vec![Rgba::BLACK, hex("#808080"), Rgba::WHITE]),
        ParamDef::toggle("colorize", "Colorize", false),
        s("colorHue", "Color Hue", 0.0, 360.0, 30.0)
            .step(1.0)
            .unit("°")
            .track(["#FF0000", "#FFFF00", "#00FF00", "#00FFFF", "#0000FF", "#FF00FF", "#FF0000"].iter().map(|h| hex(h)).collect())
            .vis(when_on("colorize")),
        s("colorSat", "Color Saturation", 0.0, 100.0, 40.0).step(1.0).vis(when_on("colorize")),
    ])
}

pub fn bw() -> Effect {
    Effect::new("bw", "Black & White", Category::Adjust, |input, p, ctx| {
        let u: Vec<f32> = ["reds", "yellows", "greens", "cyans", "blues", "magentas", "contrast"].iter().map(|k| p.f(k)).collect();
        ctx.apply("k_bw", &[input], &u)
    })
    .symbol("circle.lefthalf.filled")
    .summary("Black & white with per-colour mixing. Always perfectly neutral.")
    .params(vec![
        s("reds", "Reds", -200.0, 300.0, 40.0).step(1.0).unit("%"),
        s("yellows", "Yellows", -200.0, 300.0, 60.0).step(1.0).unit("%"),
        s("greens", "Greens", -200.0, 300.0, 40.0).step(1.0).unit("%"),
        s("cyans", "Cyans", -200.0, 300.0, 60.0).step(1.0).unit("%"),
        s("blues", "Blues", -200.0, 300.0, 20.0).step(1.0).unit("%"),
        s("magentas", "Magentas", -200.0, 300.0, 80.0).step(1.0).unit("%"),
        s("contrast", "Contrast", -100.0, 100.0, 0.0).step(1.0),
    ])
}

pub fn invert() -> Effect {
    Effect::new("invert", "Invert", Category::Adjust, |input, p, ctx| {
        let mode = match p.choice_or("mode", "all").as_str() { "luma" => 1.0, "color" => 2.0, "r" => 3.0, "g" => 4.0, "b" => 5.0, _ => 0.0 };
        ctx.apply("k_invert", &[input], &[mode, p.fd("amount", 100.0) / 100.0])
    })
        .symbol("circle.righthalf.filled.inverse")
        .summary("Negative image — everything, only the brightness, only the colours or one channel.")
        .params(vec![
            ParamDef::choice("mode", "Invert", &[("all", "Everything"), ("luma", "Brightness Only"), ("color", "Colors Only"), ("r", "Red Channel"), ("g", "Green Channel"), ("b", "Blue Channel")], "all"),
            s("amount", "Amount", 0.0, 100.0, 100.0).step(1.0).unit("%"),
        ])
}

pub fn posterize() -> Effect {
    Effect::new("posterize", "Posterize", Category::Adjust, |input, p, ctx| ctx.apply("k_posterize", &[input], &[p.fd("levels", 4.0)]))
        .symbol("square.stack.3d.down.forward")
        .summary("Reduce the number of tones per channel.")
        .params(vec![s("levels", "Levels", 2.0, 16.0, 4.0).step(1.0)])
}

pub fn threshold() -> Effect {
    Effect::new("threshold", "Threshold", Category::Adjust, |input, p, ctx| {
        let mut u = vec![p.fd("level", 50.0) / 100.0, p.f("softness") / 100.0];
        u.extend(p.color("dark").floats());
        u.extend(p.color_or("light", Rgba::WHITE).floats());
        ctx.apply("k_threshold", &[input], &u)
    })
    .symbol("circle.and.line.horizontal")
    .summary("Two-colour cut at a brightness level.")
    .params(vec![
        s("level", "Level", 0.0, 100.0, 50.0).step(1.0).unit("%"),
        s("softness", "Softness", 0.0, 50.0, 0.0).step(1.0).unit("%"),
        ParamDef::color("dark", "Dark", Rgba::BLACK),
        ParamDef::color("light", "Light", Rgba::WHITE),
    ])
}
