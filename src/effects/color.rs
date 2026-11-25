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
