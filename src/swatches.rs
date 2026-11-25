//! Factory palettes and gradients, k-means palette extraction, and the user swatch library.
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::model::*;

pub fn thermal_iron() -> GradientSpec {
    GradientSpec::from("Thermal · Iron", &["#000000", "#1B0A5C", "#6B0F8E", "#C02A5A", "#F0602A", "#FFB52E", "#FFF7B0"])
}

pub fn one_bit() -> PaletteSpec { PaletteSpec::from("1-bit", &["#000000", "#FFFFFF"]) }

pub fn gradient_groups() -> &'static Vec<(&'static str, Vec<GradientSpec>)> {
    static G: OnceLock<Vec<(&'static str, Vec<GradientSpec>)>> = OnceLock::new();
    G.get_or_init(|| {
        let mut v = vec![
        ("Thermal", vec![
            thermal_iron(),
            GradientSpec::from("Thermal · Rainbow", &["#000010", "#1E0B8C", "#0060FF", "#00D8D0", "#3CE840", "#FFF000", "#FF7A00", "#FF0000", "#FFFFFF"]),
            GradientSpec::from("Thermal · White Hot", &["#000000", "#FFFFFF"]),
            GradientSpec::from("Thermal · Black Hot", &["#FFFFFF", "#000000"]),
            GradientSpec::from("Thermal · Arctic", &["#04071E", "#0C2A6B", "#1E78C8", "#7FD3F0", "#F4F2DC", "#F8B04C", "#E8502A"]),
            GradientSpec::from("Thermal · Lava", &["#000000", "#3A0000", "#9A0A00", "#FF4A00", "#FFB000", "#FFFFFF"]),
            GradientSpec::from("Thermal · Plasma", &["#0D0887", "#6A00A8", "#B12A90", "#E16462", "#FCA636", "#F0F921"]),
            GradientSpec::from("Thermal · Medical", &["#000000", "#0000FF", "#00FFFF", "#00FF00", "#FFFF00", "#FF0000", "#FFFFFF"]),
            GradientSpec::from("Night Vision", &["#000A00", "#0C3A0C", "#3CB43C", "#C8FFB4"]),
        ]),
        ("Duotone", vec![
            GradientSpec::from("Duotone · Riso", &["#1F2A6B", "#FF5A7A"]),
            GradientSpec::from("Duotone · Sunset", &["#2B0F3A", "#FF8A3D"]),
            GradientSpec::from("Duotone · Mint", &["#0E2A2A", "#9CF0C8"]),
            GradientSpec::from("Duotone · Yellow", &["#111111", "#FFBB00"]),
            GradientSpec::from("Duotone · Ultramarine", &["#0B0B3B", "#3C6BFF", "#E8EEFF"]),
            GradientSpec::from("Duotone · Bubblegum", &["#2A0F2F", "#FF9CDA"]),
            GradientSpec::from("Duotone · Lime", &["#10230F", "#C6FF3D"]),
            GradientSpec::from("Duotone · Coral", &["#1E1B3A", "#FF6F59"]),
            GradientSpec::from("Duotone · Teal & Orange", &["#0B3C49", "#FF9F43"]),
            GradientSpec::from("Duotone · Red Riso", &["#2A0A0A", "#FF3B30", "#FFF4EE"]),
        ]),
        ("Photographic Tone", vec![
            GradientSpec::from("Cyanotype", &["#0A1E3C", "#1F4F8C", "#E8F0F4"]),
            GradientSpec::from("Sepia", &["#1A0F08", "#704A2C", "#D8B888", "#F8F0E0"]),
            GradientSpec::from("Selenium", &["#0F0E12", "#4A4550", "#B8B2B5", "#F2EFEA"]),
            GradientSpec::from("Platinum", &["#1C1712", "#6E6254", "#CFC3AE", "#F7F1E6"]),
            GradientSpec::from("Tintype", &["#12100C", "#4B4232", "#A39470", "#E9DFC6"]),
            GradientSpec::from("Bleach", &["#0A0A0A", "#5A5F5C", "#CBD0C9", "#FFFFFF"]),
            GradientSpec::from("Receipt", &["#2E2B28", "#8F8A82", "#F3F0E8"]),
        ]),
        ("Retro & Y2K", vec![
            GradientSpec::from("Y2K Chrome", &["#0A0F2C", "#2448A8", "#7FB2F0", "#E6F0FF", "#FFFFFF"]),
            GradientSpec::from("Y2K Ice", &["#0B1E3F", "#3A7BD5", "#00D2FF", "#E0F7FF"]),
            GradientSpec::from("Frutiger Aero", &["#04395E", "#2A9FD6", "#7FE0A8", "#F2FFF0"]),
            GradientSpec::from("Holographic", &["#2B1055", "#7597DE", "#B8F2E6", "#FFC2E2", "#FFFFFF"]),
            GradientSpec::from("Vaporwave", &["#1A0033", "#7A1FA2", "#FF4FD8", "#4FE3FF", "#F4F4FF"]),
            GradientSpec::from("Synthwave", &["#0D0221", "#261447", "#FF3864", "#FFD319"]),
            GradientSpec::from("Gameboy", &["#0F380F", "#306230", "#8BAC0F", "#9BBC0F"]),
            GradientSpec::from("Matrix", &["#000000", "#003B00", "#00A12B", "#9BFF9B"]),
            GradientSpec::from("Dreamcore", &["#2D2A4A", "#8EC5FF", "#C9B6FF", "#FFD6E7", "#FFFDF5"]),
        ]),
        ("Nature", vec![
            GradientSpec::from("Fire", &["#000000", "#7A0000", "#FF3000", "#FFC000", "#FFFFE0"]),
            GradientSpec::from("Ocean", &["#000814", "#003566", "#0077B6", "#48CAE4", "#CAF0F8"]),
            GradientSpec::from("Forest", &["#0B1A0F", "#1F4D2B", "#6FA15A", "#E3F2C1"]),
            GradientSpec::from("Dusk", &["#120A26", "#4B2C69", "#D0587E", "#FFB38A", "#FFE8C8"]),
            GradientSpec::from("Golden Hour", &["#1A0E05", "#7A3B12", "#E08E3C", "#FFD89A", "#FFF6E5"]),
        ]),
    ];
        v.extend(crate::effects::palettes080::extra_gradient_groups());
        v
    })
}

pub fn palette_groups() -> &'static Vec<(&'static str, Vec<PaletteSpec>)> {
    static G: OnceLock<Vec<(&'static str, Vec<PaletteSpec>)>> = OnceLock::new();
    G.get_or_init(|| {
        let mut v = vec![
        ("1-bit & Mono", vec![
            one_bit(),
            PaletteSpec::from("Mac Classic", &["#000000", "#FFFFFF"]),
            PaletteSpec::from("Paper & Ink", &["#1B1B1B", "#EEE7D7"]),
            PaletteSpec::from("Receipt", &["#2E2B28", "#F3F0E8"]),
            PaletteSpec::from("Newsprint", &["#1C1C1C", "#EDE8DC"]),
            PaletteSpec::from("Blueprint", &["#0D3B78", "#E6F0FF"]),
            PaletteSpec::from("Amber Monitor", &["#1A0D00", "#FFB000"]),
            PaletteSpec::from("Green Phosphor", &["#001A00", "#33FF33"]),
            PaletteSpec::from("Yellow Effector", &["#0A0A0A", "#FFBB00"]),
            PaletteSpec::from("Grayscale 4", &["#000000", "#555555", "#AAAAAA", "#FFFFFF"]),
            PaletteSpec::from("Grayscale 8", &["#000000", "#242424", "#494949", "#6D6D6D", "#929292", "#B6B6B6", "#DBDBDB", "#FFFFFF"]),
        ]),
        ("Game Consoles", vec![
            PaletteSpec::from("Game Boy", &["#0F380F", "#306230", "#8BAC0F", "#9BBC0F"]),
            PaletteSpec::from("Game Boy Pocket", &["#000000", "#555555", "#AAAAAA", "#FFFFFF"]),
            PaletteSpec::from("Virtual Boy", &["#000000", "#550000", "#AA0000", "#FF0000"]),
            PaletteSpec::from("Ice Cream GB", &["#7C3F58", "#EB6B6F", "#F9A875", "#FFF6D3"]),
            PaletteSpec::from("Kirokaze GB", &["#332C50", "#46878F", "#94E344", "#E2F3E4"]),
            PaletteSpec::from("Mist GB", &["#2D1B00", "#1E606E", "#5AB9A8", "#C4F0C2"]),
            PaletteSpec::from("Rustic GB", &["#2C2137", "#764462", "#A96868", "#EDB4A1"]),
            PaletteSpec::from("Nostalgia", &["#405010", "#708028", "#A0A840", "#D0D058"]),
            PaletteSpec::from("NES (subset)", &["#000000", "#FCFCFC", "#BCBCBC", "#7C7C7C", "#A4E4FC", "#3CBCFC", "#0078F8", "#0000FC", "#B8B8F8", "#6888FC", "#0058F8", "#F8B8F8", "#F878F8", "#D800CC", "#F8A4C0", "#F85898", "#F0D0B0", "#FCA044", "#E45C10", "#F8D878", "#F8B800", "#AC7C00", "#B8F8B8", "#58D854", "#00A800"]),
        ]),
        ("Computers", vec![
            PaletteSpec::from("CGA 1", &["#000000", "#55FFFF", "#FF55FF", "#FFFFFF"]),
            PaletteSpec::from("CGA 2", &["#000000", "#55FF55", "#FF5555", "#FFFF55"]),
            PaletteSpec::from("EGA", &["#000000", "#0000AA", "#00AA00", "#00AAAA", "#AA0000", "#AA00AA", "#AA5500", "#AAAAAA", "#555555", "#5555FF", "#55FF55", "#55FFFF", "#FF5555", "#FF55FF", "#FFFF55", "#FFFFFF"]),
            PaletteSpec::from("Windows 16", &["#000000", "#800000", "#008000", "#808000", "#000080", "#800080", "#008080", "#C0C0C0", "#808080", "#FF0000", "#00FF00", "#FFFF00", "#0000FF", "#FF00FF", "#00FFFF", "#FFFFFF"]),
            PaletteSpec::from("C64", &["#000000", "#FFFFFF", "#880000", "#AAFFEE", "#CC44CC", "#00CC55", "#0000AA", "#EEEE77", "#DD8855", "#664400", "#FF7777", "#333333", "#777777", "#AAFF66", "#0088FF", "#BBBBBB"]),
            PaletteSpec::from("ZX Spectrum", &["#000000", "#0000D7", "#D70000", "#D700D7", "#00D700", "#00D7D7", "#D7D700", "#D7D7D7"]),
            PaletteSpec::from("Apple II", &["#000000", "#DD0033", "#000099", "#DD22DD", "#007722", "#555555", "#2222FF", "#66AAFF", "#885500", "#FF6600", "#AAAAAA", "#FF9988", "#11DD00", "#FFFF00", "#44FF99", "#FFFFFF"]),
            PaletteSpec::from("Teletext", &["#000000", "#FF0000", "#00FF00", "#FFFF00", "#0000FF", "#FF00FF", "#00FFFF", "#FFFFFF"]),
        ]),
        ("Pixel Art", vec![
            PaletteSpec::from("PICO-8", &["#000000", "#1D2B53", "#7E2553", "#008751", "#AB5236", "#5F574F", "#C2C3C7", "#FFF1E8", "#FF004D", "#FFA300", "#FFEC27", "#00E436", "#29ADFF", "#83769C", "#FF77A8", "#FFCCAA"]),
            PaletteSpec::from("Sweetie 16", &["#1A1C2C", "#5D275D", "#B13E53", "#EF7D57", "#FFCD75", "#A7F070", "#38B764", "#257179", "#29366F", "#3B5DC9", "#41A6F6", "#73EFF7", "#F4F4F4", "#94B0C2", "#566C86", "#333C57"]),
            PaletteSpec::from("Endesga 16", &["#E4A672", "#B86F50", "#743F39", "#3F2832", "#9E2835", "#E53B44", "#FB922B", "#FFE762", "#63C64D", "#327345", "#193D3F", "#4F6781", "#AFBFD2", "#FFFFFF", "#2CE8F4", "#0484D1"]),
            PaletteSpec::from("Oil 6", &["#272744", "#494D7E", "#8B6D9C", "#C69FA5", "#F2D3AB", "#FBF5EF"]),
            PaletteSpec::from("Hollow", &["#0F0F1B", "#565A75", "#C6B7BE", "#FAFBF6"]),
        ]),
        ("Print & Riso", vec![
            PaletteSpec::from("Riso Blue/Pink", &["#F4F0E6", "#3255A4", "#FF48B0"]),
            PaletteSpec::from("Riso Fluoro", &["#F4F0E6", "#FF48B0", "#FFE800", "#0078BF", "#1A1A1A"]),
            PaletteSpec::from("Riso Teal/Orange", &["#F4F0E6", "#00838A", "#FF6C2F"]),
            PaletteSpec::from("CMYK", &["#FFFFFF", "#00AEEF", "#EC008C", "#FFF200", "#231F20"]),
            PaletteSpec::from("Xerox", &["#151515", "#8C8C8C", "#F0EFEA"]),
            PaletteSpec::from("Thermal Iron", &["#000000", "#3A0D6B", "#A21E6A", "#F0602A", "#FFC83A", "#FFF7B0"]),
        ]),
        ("Moods", vec![
            PaletteSpec::from("Y2K Blue", &["#050B26", "#1B3FAE", "#4A8CFF", "#B5D4FF", "#F0F6FF"]),
            PaletteSpec::from("Frutiger Aero", &["#0E4C6E", "#3BA4D9", "#8FE3B0", "#F4FFF7"]),
            PaletteSpec::from("Dreamcore", &["#2D2A4A", "#8EC5FF", "#C9B6FF", "#FFD6E7", "#F6E7FF"]),
            PaletteSpec::from("Pastel", &["#FFD1DC", "#FFE5B4", "#FDFD96", "#C1E1C1", "#AEC6CF", "#CBAACB"]),
            PaletteSpec::from("Earth", &["#2B1D14", "#5C4033", "#8B6B4A", "#C8A97E", "#EFE3C8"]),
            PaletteSpec::from("Neon", &["#0A0A12", "#FF00A0", "#00F0FF", "#B6FF00", "#FFFFFF"]),
            PaletteSpec::from("Cyberpunk", &["#0D0221", "#2DE2E6", "#F706CF", "#FD1D53", "#F9C80E"]),
        ]),
    ];
        v.extend(crate::effects::palettes080::extra_palette_groups());
        v
    })
}

pub fn all_gradients() -> Vec<GradientSpec> { gradient_groups().iter().flat_map(|g| g.1.clone()).collect() }
pub fn all_palettes() -> Vec<PaletteSpec> { palette_groups().iter().flat_map(|g| g.1.clone()).collect() }
pub fn gradient_group_of(name: &str) -> Option<&'static str> {
    gradient_groups().iter().find(|g| g.1.iter().any(|x| x.name == name)).map(|g| g.0)
}
pub fn palette_group_of(name: &str) -> Option<&'static str> {
    palette_groups().iter().find(|g| g.1.iter().any(|x| x.name == name)).map(|g| g.0)
}
pub fn palette_named(name: &str) -> PaletteSpec { all_palettes().into_iter().find(|p| p.name == name).unwrap_or_else(one_bit) }
pub fn gradient_named(name: &str) -> GradientSpec {
    all_gradients().into_iter().find(|g| g.name == name).unwrap_or_else(thermal_iron)
}

/// Deterministic PRNG used by palette extraction and the "randomize layer" action.
pub struct SplitMix {
    pub state: u64,
}

impl SplitMix {
    pub fn new(seed: u64) -> SplitMix { SplitMix { state: seed } }
    pub fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn unit(&mut self) -> f64 { (self.next() % 10_000_000) as f64 / 10_000_000.0 }
}

/// K-means palette extraction from an image (rgba8, straight alpha).
pub fn extract_palette(rgba: &[u8], width: usize, height: usize, count: usize) -> PaletteSpec {
    let size = 96usize;
    let (w, h) = (size, ((size as f64 * height as f64 / width.max(1) as f64) as usize).max(1));
    // box-sample down to w×h
    let mut pts: Vec<[f64; 3]> = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let x0 = x * width / w;
            let x1 = ((x + 1) * width / w).max(x0 + 1).min(width);
            let y0 = y * height / h;
            let y1 = ((y + 1) * height / h).max(y0 + 1).min(height);
            let mut acc = [0.0; 3];
            let mut n = 0.0;
            for yy in y0..y1 {
                for xx in x0..x1 {
                    let i = (yy * width + xx) * 4;
                    acc[0] += rgba[i] as f64;
                    acc[1] += rgba[i + 1] as f64;
                    acc[2] += rgba[i + 2] as f64;
                    n += 1.0;
                }
            }
            pts.push([acc[0] / n / 255.0, acc[1] / n / 255.0, acc[2] / n / 255.0]);
        }
    }
    let d2 = |a: &[f64; 3], b: &[f64; 3]| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2);
    let k = count.clamp(2, 32);
    let mut rng = SplitMix::new(42);
    let mut centers = vec![pts[pts.len() / 2]];
    while centers.len() < k {
        let dists: Vec<f64> = pts.iter().map(|p| centers.iter().map(|c| d2(p, c)).fold(f64::INFINITY, f64::min)).collect();
        let total: f64 = dists.iter().sum();
        if total <= 0.0 {
            centers.push(pts[(rng.next() % pts.len() as u64) as usize]);
            continue;
        }
        let mut r = (rng.next() % 1_000_000) as f64 / 1_000_000.0 * total;
        let mut idx = 0;
        for (i, d) in dists.iter().enumerate() {
            r -= d;
            if r <= 0.0 {
                idx = i;
                break;
            }
        }
        centers.push(pts[idx]);
    }
    for _ in 0..12 {
        let mut sums = vec![[0.0; 3]; k];
        let mut counts = vec![0usize; k];
        for p in &pts {
            let mut best = 0;
            let mut bd = f64::INFINITY;
            for (j, c) in centers.iter().enumerate() {
                let d = d2(p, c);
                if d < bd {
                    bd = d;
                    best = j;
                }
            }
            for c in 0..3 {
                sums[best][c] += p[c];
            }
            counts[best] += 1;
        }
        for j in 0..k {
            if counts[j] > 0 {
                let n = counts[j] as f64;
                centers[j] = [sums[j][0] / n, sums[j][1] / n, sums[j][2] / n];
            }
        }
    }
    let l = |c: &[f64; 3]| c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
    centers.sort_by(|a, b| l(a).partial_cmp(&l(b)).unwrap());
    PaletteSpec { name: crate::l10n::tr("Extracted"), colors: centers.iter().map(|c| Rgba::new(c[0], c[1], c[2])).collect() }
}

// MARK: - User library

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct SwatchStore {
    #[serde(default)]
    pub palettes: Vec<PaletteSpec>,
    #[serde(default)]
    pub gradients: Vec<GradientSpec>,
}

pub fn library() -> &'static Mutex<SwatchStore> {
    static L: OnceLock<Mutex<SwatchStore>> = OnceLock::new();
    L.get_or_init(|| {
        let s = std::fs::read(crate::settings::support_dir().join("swatches.json"))
            .ok()
            .and_then(|d| serde_json::from_slice(&d).ok())
            .unwrap_or_default();
        Mutex::new(s)
    })
}

fn persist(s: &SwatchStore) {
    if let Ok(d) = serde_json::to_vec(s) {
        crate::settings::write_atomic(&crate::settings::support_dir().join("swatches.json"), &d);
    }
}

pub fn save_palette(p: PaletteSpec) {
    let mut s = library().lock().unwrap();
    s.palettes.retain(|x| x.name != p.name);
    s.palettes.insert(0, p);
    persist(&s);
}
pub fn delete_palette(p: &PaletteSpec) {
    let mut s = library().lock().unwrap();
    s.palettes.retain(|x| x != p);
    persist(&s);
}
pub fn save_gradient(g: GradientSpec) {
    let mut s = library().lock().unwrap();
    s.gradients.retain(|x| x.name != g.name);
    s.gradients.insert(0, g);
    persist(&s);
}
pub fn delete_gradient(g: &GradientSpec) {
    let mut s = library().lock().unwrap();
    s.gradients.retain(|x| x != g);
    persist(&s);
}

/// Palette file text (.gpl when `gpl`, else .hex).
pub fn palette_file_text(p: &PaletteSpec, gpl: bool) -> String {
    if gpl {
        let mut t = format!("GIMP Palette\nName: {}\nColumns: 8\n#\n", p.name);
        let lines: Vec<String> = p
            .colors
            .iter()
            .map(|c| format!("{:3} {:3} {:3}\t{}", (c.r * 255.0) as i32, (c.g * 255.0) as i32, (c.b * 255.0) as i32, c.to_hex()))
            .collect();
        t += &lines.join("\n");
        t + "\n"
    } else {
        p.colors.iter().map(|c| c.to_hex()[1..].to_string()).collect::<Vec<_>>().join("\n") + "\n"
    }
}

/// Parses .hex / .gpl / .txt / .json palette files.
pub fn parse_palette_file(name: &str, ext: &str, text: &str) -> Option<PaletteSpec> {
    let mut colors: Vec<Rgba> = vec![];
    if ext.eq_ignore_ascii_case("json") {
        if let Ok(p) = serde_json::from_str::<PaletteSpec>(text) {
            colors = p.colors;
        }
    } else {
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with("GIMP") || l.starts_with("Name") || l.starts_with("Columns") || l.starts_with(';') {
                continue;
            }
            if l.starts_with('#') && l.chars().count() < 7 {
                continue;
            }
            let parts: Vec<&str> = l.split([' ', '\t']).filter(|s| !s.is_empty()).collect();
            if parts.len() >= 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>(), parts[2].parse::<i32>()) {
                    colors.push(Rgba::new(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0));
                    continue;
                }
            }
            if let Some(first) = parts.first() {
                let h = first.trim_start_matches('#');
                if (h.len() == 6 || h.len() == 8) && h.chars().all(|c| c.is_ascii_hexdigit()) {
                    colors.push(Rgba::hex(&h[h.len() - 6..]));
                }
            }
        }
    }
    if colors.is_empty() {
        return None;
    }
    colors.truncate(64);
    Some(PaletteSpec { name: name.into(), colors })
}

pub fn parse_gradient_file(data: &[u8]) -> Vec<GradientSpec> {
    if let Ok(gs) = serde_json::from_slice::<Vec<GradientSpec>>(data) {
        return gs;
    }
    serde_json::from_slice::<GradientSpec>(data).map(|g| vec![g]).unwrap_or_default()
}
