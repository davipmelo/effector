use super::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::overlay::{self, Weight};
use crate::swatches;

pub const CHARSETS: [(&str, &str, &str); 13] = [
    ("standard", "Standard", " .:-=+*#%@"),
    ("detailed", "Detailed", " .'`^\",:;Il!i><~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$"),
    ("blocks", "Blocks", " ░▒▓█"),
    ("dots", "Dots", " ·•●"),
    ("binary", "Binary", " 01"),
    ("hex", "Hex", " 0123456789ABCDEF"),
    ("slashes", "Slashes", " /\\X"),
    ("braille", "Braille", " ⠁⠃⠇⠏⠟⠿⡿⣿"),
    ("geometric", "Geometric", " ·○◎●■"),
    ("letters", "Letters", " ilEFFECTOR"),
    ("matrix", "Matrix", " ｱｲｳｴｵｶｷｸｹｺ"),
    ("emoji", "Shapes", " ▫◽◻⬜"),
    ("custom", "Custom…", ""),
];

const CELL_W: usize = 40;
const CELL_H: usize = 72;

/// Builds and caches glyph atlases sorted by ink density.
fn atlas(chars: &str, bold: bool, font: Option<&str>) -> (T, usize) {
    static CACHE: OnceLock<Mutex<HashMap<String, (T, usize)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = format!("{chars}{}|{}", if bold { "|b" } else { "|r" }, font.unwrap_or(""));
    let mut c = cache.lock().unwrap();
    if let Some(v) = c.get(&key) {
        return v.clone();
    }
    if c.len() > 32 {
        c.clear(); // custom character sets change while typing
    }
    let (gw, gh) = (CELL_W, CELL_H);
    let size = gh as f64 * 0.78;
    let weight = if bold { Weight::Heavy } else { Weight::Medium };
    let (_, asc, desc) = overlay::measure("Hg", size as f32, true, weight, font, 1.0, 0.0);
    let mut rendered: Vec<(f64, Vec<u8>)> = vec![];
    for g in chars.chars() {
        let s = g.to_string();
        let mut ov = overlay::OverlayCanvas::new(gw, gh);
        let (wdt, _, _) = overlay::measure(&s, size as f32, true, weight, font, 1.0, 0.0);
        // block elements fill the whole cell: stretch them
        let is_block = (0x2580..=0x259F).contains(&(g as u32));
        if is_block {
            let sx = gw as f64 / (wdt as f64).max(1.0);
            let sy = gh as f64 / (asc + desc) as f64;
            ov.text_scaled(&s, 0.0, 0.0, size, sx, sy, weight);
        } else {
            // baseline at 80 % of the cell height (CoreText: 20 % from the bottom)
            let top = gh as f64 * 0.8 - asc as f64;
            ov.text(&s, (gw as f64 - wdt as f64) / 2.0, top, size, Rgba::WHITE, weight, true, false, None, 0.0, font, 1.0, None, 0.0);
        }
        let buf: Vec<u8> = (0..gw * gh).map(|i| ov.data()[i * 4 + 3]).collect();
        let density = buf.iter().map(|v| *v as f64).sum::<f64>() / (gw * gh) as f64 / 255.0;
        rendered.push((density, buf));
    }
    rendered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let n = rendered.len().max(1);
    let mut px = vec![0u8; n * gw * gh * 4];
    for (i, r) in rendered.iter().enumerate() {
        for y in 0..gh {
            for x in 0..gw {
                let v = r.1[y * gw + x];
                let j = (y * n * gw + i * gw + x) * 4;
                px[j] = v;
                px[j + 1] = v;
                px[j + 2] = v;
                px[j + 3] = 255;
            }
        }
    }
    let t = crate::gpu::gpu().texture_from_rgba8(&px, n * gw, gh);
    c.insert(key, (t.clone(), n));
    (t, n)
}

fn thermal_palette() -> PaletteSpec {
    swatches::all_palettes().into_iter().find(|p| p.name == "Thermal Iron").unwrap_or_else(swatches::one_bit)
}

pub fn ascii() -> Effect {
    Effect::new("ascii", "ASCII", Category::Stylize, |input, p, ctx| {
        // whole render pixels per cell so every glyph is drawn identically
        let cw = ctx.px(p.num("cell", 8.0)).round().max(2.0);
        let ch = (cw * p.num("aspect", 180.0) / 100.0).round().max(2.0);
        let gw = ((input.width() as f64 / cw).ceil() as usize).max(1);
        let gh = ((input.height() as f64 / ch).ceil() as usize).max(1);
        let grid = ctx.resize(input, gw, gh, false);
        let cs = p.choice_or("charset", "standard");
        let mut set: String = CHARSETS.iter().find(|c| c.0 == cs).map(|c| c.2.to_string()).unwrap_or_else(|| " .:-=+*#%@".into());
        if cs == "custom" {
            // unique characters, always including a blank so dark areas can stay empty
            let mut seen = std::collections::HashSet::new();
            let chars: String = format!(" {}", p.text_or("customChars", " .oO@")).chars().filter(|c| *c != '\n' && seen.insert(*c)).collect();
            set = if chars.chars().count() >= 2 { chars } else { " .oO@".into() };
        }
        let font = overlay::font_param(p);
        let (atl, n) = atlas(&set, p.bool("bold"), font.as_deref());
        let mode = match p.choice_or("mode", "mono").as_str() { "palette" => 1.0, "original" => 2.0, _ => 0.0 };
        let mut pal = p.palette("palette").unwrap_or_else(swatches::one_bit);
        pal.colors.sort_by(|a, b| a.luma().partial_cmp(&b.luma()).unwrap());
        let mut ex: Vec<f32> = vec![pal.colors.len() as f32];
        for c in &pal.colors {
            ex.extend(c.floats());
        }
        let mut u = vec![cw as f32, ch as f32, n as f32, mode];
        u.extend(p.color_or("fg", hex("#F2F2F2")).floats());
        u.extend(p.color_or("bg", hex("#0A0A0A")).floats());
        u.extend([p.b("invert"), p.f("contrast"), p.f("brightness")]);
        ctx.apply_sized("k_ascii", &[&grid, &atl], &u, Some(&ex), input.width(), input.height())
    })
    .symbol("textformat.characters")
    .summary("Rebuilds the image from characters.")
    .params(vec![
        ParamDef::choice_opts("charset", "Characters", CHARSETS.iter().map(|c| opt(c.0, c.1)).collect(), "standard").sec("Characters"),
        ParamDef::text("customChars", "Your Characters (dark → light order is automatic)", " .oO@")
            .sec("Characters")
            .vis(when("charset", "custom")),
        s("cell", "Cell Size", 2.0, 40.0, 8.0).step(0.5).curve(1.6).sec("Characters"),
        s("aspect", "Cell Height", 100.0, 250.0, 180.0).step(1.0).unit("%").sec("Characters"),
        ParamDef::toggle("bold", "Bold", false).sec("Characters"),
        overlay::font_def("Characters"),
        ParamDef::segmented("mode", "Color", &[("mono", "Mono"), ("palette", "Palette"), ("original", "Original")], "mono").sec("Color"),
        ParamDef::color("fg", "Characters", hex("#F2F2F2")).sec("Color").vis(when("mode", "mono")),
        ParamDef::color("bg", "Background", hex("#0A0A0A")).sec("Color"),
        ParamDef::palette("palette", "Palette", thermal_palette()).sec("Color").vis(when("mode", "palette")),
        ParamDef::toggle("invert", "Invert", false).sec("Tone"),
        s("contrast", "Contrast", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
        s("brightness", "Brightness", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
    ])
    .looks(vec![
        look("terminal", "Green Terminal", vals!["charset" => "standard", "fg" => hex("#5CFF7A"), "bg" => hex("#04110A")]),
        look("amber", "Amber Terminal", vals!["charset" => "detailed", "cell" => 6.0, "fg" => hex("#FFB000"), "bg" => hex("#140A00")]),
        look("typewriter", "Typewriter", vals!["charset" => "detailed", "cell" => 6.0, "invert" => true, "fg" => hex("#1A1A1A"), "bg" => hex("#F2F2F2")]),
        look("matrix", "Matrix", vals!["charset" => "matrix", "cell" => 9.0, "fg" => hex("#3BFF6E"), "bg" => hex("#000000"), "contrast" => 40.0]),
        look("color", "Color Text", vals!["charset" => "detailed", "cell" => 5.0, "mode" => "original", "bg" => hex("#000000")]),
        look("blocks", "Block Art", vals!["charset" => "blocks", "cell" => 7.0, "aspect" => 100.0, "mode" => "original"]),
        look("braille", "Braille Fine", vals!["charset" => "braille", "cell" => 4.0, "aspect" => 150.0]),
        look("binary", "Binary Code", vals!["charset" => "binary", "cell" => 8.0, "fg" => hex("#29ADFF"), "bg" => hex("#020812")]),
        look("thermal", "Thermal Text", vals!["charset" => "standard", "mode" => "palette", "cell" => 7.0, "palette" => thermal_palette()]),
        look("yellow", "EFFECTOR", vals!["charset" => "letters", "cell" => 8.0, "fg" => hex("#FFBB00"), "bg" => hex("#0A0A0A")]),
    ])
}

// SWIFT extra members of AsciiEffect:
// static let charsets: [(String, String, String)] = [
