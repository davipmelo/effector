//! Built-in Looks added in 0.8.0 for every effect that had none (3–6 each), merged into the
//! registry by `effects::all()`.

use super::*;

fn n(pairs: &[(&str, f64)]) -> Vec<(String, ParamValue)> { pairs.iter().map(|(k, v)| (k.to_string(), ParamValue::Number(*v))).collect() }

fn with(mut v: Vec<(String, ParamValue)>, extra: Vec<(String, ParamValue)>) -> Vec<(String, ParamValue)> {
    v.extend(extra);
    v
}

fn curve(master: &[(f64, f64)], r: &[(f64, f64)], g: &[(f64, f64)], b: &[(f64, f64)]) -> ParamValue {
    let pts = |v: &[(f64, f64)]| if v.is_empty() { CurveSet::identity() } else { v.iter().map(|(x, y)| CurvePoint { x: *x, y: *y }).collect() };
    ParamValue::Curves(CurveSet { master: pts(master), red: pts(r), green: pts(g), blue: pts(b) })
}

fn grad(name: &str, hexes: &[&str]) -> ParamValue { ParamValue::Gradient(GradientSpec::from(name, hexes)) }
fn pal(name: &str, hexes: &[&str]) -> ParamValue { ParamValue::Palette(PaletteSpec::from(name, hexes)) }

pub fn for_effect(id: &str) -> Vec<Look> {
    match id {
        "basic" => vec![
            look("bright", "Bright & Airy", n(&[("exposure", 0.35), ("contrast", -10.0), ("highlights", -30.0), ("shadows", 35.0), ("whites", 10.0), ("vibrance", 15.0)])),
            look("punchy", "Punchy", n(&[("contrast", 30.0), ("highlights", -15.0), ("shadows", 10.0), ("blacks", -15.0), ("vibrance", 30.0)])),
            look("matte", "Soft Matte", n(&[("contrast", -20.0), ("blacks", 30.0), ("highlights", -20.0), ("saturation", -10.0)])),
            look("moody", "Moody", n(&[("exposure", -0.3), ("contrast", 20.0), ("highlights", -40.0), ("shadows", -10.0), ("saturation", -20.0), ("temperature", -10.0)])),
            look("warm", "Warm Afternoon", n(&[("temperature", 30.0), ("tint", 8.0), ("vibrance", 15.0), ("highlights", -10.0)])),
            look("hdr", "Recovered Detail", n(&[("highlights", -80.0), ("shadows", 70.0), ("whites", -10.0), ("blacks", -10.0), ("contrast", 10.0)])),
        ],
        "curves" => vec![
            look("scurve", "Gentle S-Curve", vec![("curves".into(), curve(&[(0.0, 0.0), (0.25, 0.2), (0.75, 0.82), (1.0, 1.0)], &[], &[], &[]))]),
            look("strong", "Strong Contrast", vec![("curves".into(), curve(&[(0.0, 0.0), (0.25, 0.14), (0.75, 0.88), (1.0, 1.0)], &[], &[], &[]))]),
            look("fade", "Faded Film", vec![("curves".into(), curve(&[(0.0, 0.12), (0.3, 0.32), (0.75, 0.78), (1.0, 0.94)], &[], &[], &[]))]),
            look("cross", "Cross Process", vec![("curves".into(), curve(&[], &[(0.0, 0.0), (0.3, 0.22), (0.75, 0.85), (1.0, 1.0)], &[(0.0, 0.05), (0.5, 0.55), (1.0, 0.95)], &[(0.0, 0.15), (0.5, 0.45), (1.0, 0.8)]))]),
            look("teal", "Teal Shadows", vec![("curves".into(), curve(&[], &[(0.0, 0.0), (0.3, 0.25), (1.0, 1.0)], &[(0.0, 0.04), (1.0, 1.0)], &[(0.0, 0.1), (0.35, 0.38), (1.0, 0.95)]))]),
            look("negative", "Inverted", vec![("curves".into(), curve(&[(0.0, 1.0), (1.0, 0.0)], &[], &[], &[]))]),
        ],
        "hsl" => vec![
            look("teal_orange", "Teal & Orange", n(&[("orange_s", 20.0), ("orange_l", 8.0), ("blue_h", -25.0), ("aqua_h", 15.0), ("blue_s", 15.0), ("green_s", -40.0), ("yellow_s", -20.0)])),
            look("autumn", "Autumn", n(&[("green_h", -45.0), ("yellow_h", -20.0), ("green_s", -20.0), ("orange_s", 25.0), ("red_s", 10.0)])),
            look("muted_greens", "Muted Greens", n(&[("green_s", -60.0), ("yellow_s", -30.0), ("green_l", -10.0), ("aqua_s", -30.0)])),
            look("blue_sky", "Deep Blue Sky", n(&[("blue_s", 35.0), ("blue_l", -30.0), ("aqua_h", 20.0), ("aqua_s", 20.0)])),
            look("pastel", "Pastel Pop", n(&[("red_l", 20.0), ("orange_l", 15.0), ("blue_l", 25.0), ("purple_l", 20.0), ("magenta_l", 20.0), ("green_s", -25.0)])),
            look("mono_red", "Red Only", n(&[("orange_s", -100.0), ("yellow_s", -100.0), ("green_s", -100.0), ("aqua_s", -100.0), ("blue_s", -100.0), ("purple_s", -100.0), ("magenta_s", -60.0), ("red_s", 30.0)])),
        ],
        "huesat" => vec![
            look("muted", "Muted", n(&[("saturation", -40.0)])),
            look("vivid", "Vivid", n(&[("saturation", 40.0), ("lightness", -5.0)])),
            look("shift", "Hue Shift", n(&[("hue", 40.0)])),
            look("alien", "Alien World", n(&[("hue", 150.0), ("saturation", 20.0)])),
            look("sepia", "Sepia", with(n(&[("colorHue", 32.0), ("colorSat", 35.0)]), vec![("colorize".into(), ParamValue::Bool(true))])),
            look("blueprint", "Cyan Mono", with(n(&[("colorHue", 200.0), ("colorSat", 50.0), ("lightness", -5.0)]), vec![("colorize".into(), ParamValue::Bool(true))])),
        ],
        "bw" => vec![
            look("neutral", "Neutral", n(&[("reds", 40.0), ("yellows", 60.0), ("greens", 40.0), ("cyans", 60.0), ("blues", 20.0), ("magentas", 80.0)])),
            look("red", "Red Filter", n(&[("reds", 120.0), ("yellows", 110.0), ("greens", -20.0), ("cyans", -40.0), ("blues", -60.0), ("magentas", 80.0), ("contrast", 15.0)])),
            look("yellow", "Yellow Filter", n(&[("reds", 70.0), ("yellows", 110.0), ("greens", 50.0), ("cyans", 20.0), ("blues", -10.0), ("magentas", 60.0)])),
            look("green", "Green Filter", n(&[("reds", 10.0), ("yellows", 80.0), ("greens", 120.0), ("cyans", 60.0), ("blues", 10.0), ("magentas", 0.0)])),
            look("blue", "Blue Filter", n(&[("reds", -20.0), ("yellows", 0.0), ("greens", 30.0), ("cyans", 110.0), ("blues", 140.0), ("magentas", 60.0)])),
            look("ir", "Infrared", n(&[("reds", 60.0), ("yellows", 200.0), ("greens", 260.0), ("cyans", -50.0), ("blues", -100.0), ("magentas", 40.0), ("contrast", 25.0)])),
        ],
        "gradientmap" => vec![
            look("iron", "Thermal Iron", vec![("gradient".into(), ParamValue::Gradient(crate::swatches::thermal_iron()))]),
            look("cyan", "Cyanotype", vec![("gradient".into(), grad("Cyanotype", &["#0A1E3C", "#1F4F8C", "#E8F0F4"]))]),
            look("sepia", "Sepia", vec![("gradient".into(), grad("Sepia", &["#1A0F08", "#704A2C", "#D8B888", "#F8F0E0"]))]),
            look("riso", "Riso Duotone", vec![("gradient".into(), grad("Duotone · Riso", &["#1F2A6B", "#FF5A7A"]))]),
            look("vapor", "Vaporwave", vec![("gradient".into(), grad("Vaporwave", &["#1A0033", "#7A1FA2", "#FF4FD8", "#4FE3FF", "#F4F4FF"]))]),
            look("soft", "Soft Golden Tint", with(vec![("gradient".into(), grad("Golden Hour", &["#1A0E05", "#7A3B12", "#E08E3C", "#FFD89A", "#FFF6E5"]))], n(&[("amount", 40.0)]))),
        ],
        "lut" => vec![
            look("full", "Full Strength", n(&[("amount", 100.0)])),
            look("half", "Half Strength", n(&[("amount", 50.0)])),
            look("hint", "Just a Hint", n(&[("amount", 25.0)])),
        ],
        "sharpen" => vec![
            look("crisp", "Crisp", n(&[("amount", 60.0), ("radius", 1.0), ("threshold", 3.0)])),
            look("detail", "Fine Detail", n(&[("amount", 140.0), ("radius", 0.6), ("threshold", 2.0)])),
            look("clarity", "Clarity", n(&[("amount", 40.0), ("radius", 12.0), ("threshold", 0.0)])),
            look("over", "Oversharpened", n(&[("amount", 400.0), ("radius", 2.0), ("threshold", 0.0)])),
        ],
        "invert" => vec![
            look("negative", "Negative", vec![]),
            look("luma", "Inverted Light", vec![("mode".into(), ParamValue::Choice("luma".into()))]),
            look("colors", "Opposite Colors", vec![("mode".into(), ParamValue::Choice("color".into()))]),
            look("half", "Half Negative", n(&[("amount", 50.0)])),
            look("red", "Red Channel Swap", vec![("mode".into(), ParamValue::Choice("r".into()))]),
        ],
        "posterize" => vec![
            look("2", "Two Tones", n(&[("levels", 2.0)])),
            look("3", "Screen Print", n(&[("levels", 3.0)])),
            look("5", "Comic", n(&[("levels", 5.0)])),
            look("8", "8-bit Banding", n(&[("levels", 8.0)])),
        ],
        "threshold" => vec![
            look("bw", "Black & White", vec![]),
            look("soft", "Soft Ink", n(&[("level", 45.0), ("softness", 18.0)])),
            look("bright", "Blown Out", n(&[("level", 30.0)])),
            look("dark", "Shadows Only", n(&[("level", 70.0)])),
            look("riso", "Riso Blue", vec![("dark".into(), ParamValue::Color(hex("#2A3BB7"))), ("light".into(), ParamValue::Color(hex("#F4F0E6")))]),
            look("amber", "Amber Terminal", vec![("dark".into(), ParamValue::Color(hex("#1A0D00"))), ("light".into(), ParamValue::Color(hex("#FFB000")))]),
        ],
        "dither" => vec![
            look("mac", "Classic Mac", with(n(&[("pixelSize", 2.0)]), vec![("algorithm".into(), ParamValue::Choice("atkinson".into()))])),
            look("gameboy", "Game Boy", with(n(&[("pixelSize", 3.0)]), vec![
                ("algorithm".into(), ParamValue::Choice("bayer4".into())), ("mode".into(), ParamValue::Choice("palette".into())),
                ("palette".into(), pal("Game Boy", &["#0F380F", "#306230", "#8BAC0F", "#9BBC0F"])),
            ])),
            look("newspaper", "Newspaper", with(n(&[("pixelSize", 1.5)]), vec![("algorithm".into(), ParamValue::Choice("cluster".into())), ("dark".into(), ParamValue::Color(hex("#1C1C1C"))), ("light".into(), ParamValue::Color(hex("#EDE8DC")))])),
            look("cga", "CGA Arcade", with(n(&[("pixelSize", 3.0)]), vec![
                ("algorithm".into(), ParamValue::Choice("bayer8".into())), ("mode".into(), ParamValue::Choice("palette".into())),
                ("palette".into(), pal("CGA 1", &["#000000", "#55FFFF", "#FF55FF", "#FFFFFF"])),
            ])),
            look("pico", "PICO-8", with(n(&[("pixelSize", 3.0)]), vec![
                ("algorithm".into(), ParamValue::Choice("floyd".into())), ("mode".into(), ParamValue::Choice("palette".into())),
                ("palette".into(), pal("PICO-8", &["#000000", "#1D2B53", "#7E2553", "#008751", "#AB5236", "#5F574F", "#C2C3C7", "#FFF1E8", "#FF004D", "#FFA300", "#FFEC27", "#00E436", "#29ADFF", "#83769C", "#FF77A8", "#FFCCAA"])),
            ])),
            look("riso", "Riso Print", with(n(&[("pixelSize", 2.0)]), vec![
                ("algorithm".into(), ParamValue::Choice("bluenoise".into())), ("mode".into(), ParamValue::Choice("palette".into())),
                ("palette".into(), pal("Riso Blue/Pink", &["#F4F0E6", "#3255A4", "#FF48B0"])),
            ])),
        ],
        "halftone" => vec![
            look("news", "Newsprint", vec![("paper".into(), ParamValue::Color(hex("#EDE8DC"))), ("ink".into(), ParamValue::Color(hex("#1C1C1C")))]),
            look("pop", "Pop Art", with(n(&[("cell", 14.0)]), vec![("mode".into(), ParamValue::Choice("cmyk".into()))])),
            look("lines", "Engraved Lines", with(n(&[("cell", 6.0), ("angle", 30.0)]), vec![("shape".into(), ParamValue::Choice("line".into()))])),
            look("fine", "Fine Screen", n(&[("cell", 4.0), ("smooth", 60.0)])),
            look("big", "Big Dots", n(&[("cell", 28.0), ("dotScale", 110.0)])),
            look("riso", "Riso Pink", vec![("paper".into(), ParamValue::Color(hex("#F4F0E6"))), ("ink".into(), ParamValue::Color(hex("#FF48B0")))]),
        ],
        "printwear" => vec![
            look("office", "Office Copy", vec![]),
            look("fax", "Old Fax", n(&[("fade", 35.0), ("streaks", 50.0), ("banding", 40.0), ("noise", 45.0)])),
            look("zine", "Zine", n(&[("uneven", 55.0), ("specks", 50.0), ("edge", 40.0), ("roll", 20.0)])),
            look("dying", "Dying Toner", n(&[("fade", 70.0), ("uneven", 70.0), ("dead", 40.0), ("streaks", 60.0)])),
            look("clean", "Clean Laser", n(&[("fade", 5.0), ("uneven", 10.0), ("streaks", 5.0), ("noise", 10.0), ("specks", 5.0)])),
        ],
        "film" => vec![
            look("portra", "Portra 400", vec![("stock".into(), ParamValue::Choice("portra400".into()))]),
            look("cinestill", "CineStill 800T", vec![("stock".into(), ParamValue::Choice("cinestill800t".into()))]),
            look("gold", "Gold 200 Summer", with(n(&[("warmth", 15.0)]), vec![("stock".into(), ParamValue::Choice("gold200".into()))])),
            look("velvia", "Velvia Landscape", vec![("stock".into(), ParamValue::Choice("velvia50".into()))]),
            look("expired", "Expired Roll", with(n(&[("fade", 35.0), ("grain", 55.0)]), vec![("stock".into(), ParamValue::Choice("expired".into()))])),
            look("trix", "Tri-X Pushed", with(n(&[("grain", 60.0), ("grainSize", 1.4), ("exposure", 0.3)]), vec![("stock".into(), ParamValue::Choice("trix400".into()))])),
        ],
        "grain" => vec![
            look("fine", "Fine 100 ISO", n(&[("amount", 18.0), ("size", 0.7), ("roughness", 30.0)])),
            look("classic", "Classic 400", n(&[("amount", 35.0), ("size", 1.0)])),
            look("pushed", "Pushed 3200", n(&[("amount", 70.0), ("size", 1.8), ("roughness", 70.0), ("highlights", 70.0)])),
            look("color", "Color Negative", with(n(&[("amount", 40.0), ("colorAmount", 45.0)]), vec![("color".into(), ParamValue::Bool(true))])),
            look("shadow", "Shadow Grain", n(&[("amount", 45.0), ("shadows", 100.0), ("midtones", 50.0), ("highlights", 10.0)])),
        ],
        "glow" => vec![
            look("soft", "Soft Glow", vec![("mode".into(), ParamValue::Choice("soft".into()))]),
            look("dreamy", "Dreamy", n(&[("intensity", 90.0), ("radius", 60.0), ("threshold", 20.0)])),
            look("neon", "Neon Lights", with(n(&[("intensity", 140.0), ("radius", 25.0), ("threshold", 65.0), ("saturation", 160.0)]), vec![("mode".into(), ParamValue::Choice("add".into()))])),
            look("warm", "Warm Haze", with(n(&[("intensity", 70.0), ("radius", 80.0), ("threshold", 35.0)]), vec![("tint".into(), ParamValue::Color(hex("#FFC48A")))])),
            look("night", "Night Bloom", with(n(&[("intensity", 110.0), ("radius", 40.0), ("threshold", 75.0)]), vec![("tint".into(), ParamValue::Color(hex("#A9C4FF")))])),
        ],
        "bloom" => vec![
            look("subtle", "Subtle", n(&[("intensity", 50.0), ("radius", 20.0), ("threshold", 80.0)])),
            look("cinema", "Cinematic", n(&[("intensity", 100.0), ("radius", 45.0), ("threshold", 72.0)])),
            look("blown", "Blown Out", n(&[("intensity", 220.0), ("radius", 80.0), ("threshold", 55.0), ("knee", 30.0)])),
            look("warm", "Golden Bloom", with(n(&[("intensity", 120.0), ("radius", 50.0)]), vec![("tint".into(), ParamValue::Color(hex("#FFCC80")))])),
        ],
        "halation" => vec![
            look("cinestill", "CineStill Red", vec![]),
            look("subtle", "Subtle Fringe", n(&[("intensity", 55.0), ("radius", 12.0), ("threshold", 88.0)])),
            look("heavy", "Heavy Halation", n(&[("intensity", 200.0), ("radius", 40.0), ("spread", 80.0), ("threshold", 75.0), ("bloom", 30.0)])),
            look("amber", "Warm Amber", vec![("inner".into(), ParamValue::Color(hex("#FFC27A"))), ("color".into(), ParamValue::Color(hex("#FF7A1A")))]),
            look("magenta", "Magenta Bloom", vec![("inner".into(), ParamValue::Color(hex("#FF8AD8"))), ("color".into(), ParamValue::Color(hex("#C21E8F")))]),
        ],
        "chromab" => vec![
            look("subtle", "Subtle Fringe", n(&[("amount", 3.0)])),
            look("cheap", "Cheap Lens", n(&[("amount", 10.0), ("falloff", 1.2)])),
            look("glitch", "Glitch Split", with(n(&[("amount", 25.0)]), vec![("mode".into(), ParamValue::Choice("linear".into()))])),
            look("3d", "Anaglyph 3D", with(n(&[("amount", 18.0), ("angle", 0.0)]), vec![("mode".into(), ParamValue::Choice("linear".into()))])),
        ],
        "vignette" => vec![
            look("subtle", "Subtle", n(&[("amount", 25.0), ("feather", 70.0)])),
            look("heavy", "Heavy", n(&[("amount", 75.0), ("midpoint", 35.0)])),
            look("round", "Round Lens", n(&[("amount", 60.0), ("roundness", 100.0), ("feather", 40.0)])),
            look("white", "White Fade", with(n(&[("amount", 50.0)]), vec![("color".into(), ParamValue::Color(Rgba::WHITE))])),
            look("tunnel", "Tunnel Blur", with(n(&[("amount", 50.0), ("blur", 20.0)]), vec![("blurOn".into(), ParamValue::Bool(true))])),
        ],
        "motionblur" => vec![
            look("pan", "Panning", n(&[("distance", 80.0)])),
            look("vertical", "Vertical Drop", n(&[("angle", 90.0), ("distance", 70.0)])),
            look("edges", "Speed Edges", with(n(&[("distance", 120.0), ("protect", 40.0)]), vec![("area".into(), ParamValue::Choice("edges".into()))])),
            look("trailing", "Trailing", with(n(&[("distance", 100.0)]), vec![("centered".into(), ParamValue::Bool(false))])),
            look("diagonal", "Diagonal Rush", n(&[("angle", 35.0), ("distance", 90.0)])),
        ],
        "mirror" => vec![
            look("lr", "Left → Right", vec![]),
            look("tb", "Top → Bottom", vec![("mode".into(), ParamValue::Choice("tb".into()))]),
            look("quad", "Four-Way", vec![("mode".into(), ParamValue::Choice("quad".into()))]),
            look("tiles", "Mirrored Tiles", with(n(&[("tiles", 3.0)]), vec![("mode".into(), ParamValue::Choice("tilesm".into()))])),
            look("grid", "Repeat Grid", with(n(&[("tiles", 4.0)]), vec![("mode".into(), ParamValue::Choice("tiles".into()))])),
        ],
        "stretch" => vec![
            look("right", "Slit Scan Right", vec![]),
            look("down", "Melting Down", with(n(&[("position", 45.0)]), vec![("dir".into(), ParamValue::Choice("down".into()))])),
            look("glitch", "Glitchy Lanes", n(&[("jitter", 70.0), ("lane", 8.0)])),
            look("clean", "Clean Smear", n(&[("jitter", 0.0), ("lane", 1.0)])),
        ],
        "led" => vec![
            look("cyan", "Cyan Billboard", vec![]),
            look("amber", "Bus Sign", vec![("color".into(), ParamValue::Color(hex("#FFA51F"))), ("bg".into(), ParamValue::Color(hex("#0D0700")))]),
            look("stadium", "Stadium Screen", with(n(&[("cell", 6.0), ("levels", 12.0), ("glow", 60.0)]), vec![("mode".into(), ParamValue::Choice("color".into()))])),
            look("matrix", "Green Dots", with(n(&[("cell", 10.0), ("dotSize", 60.0)]), vec![("color".into(), ParamValue::Color(hex("#39FF6A"))), ("bg".into(), ParamValue::Color(hex("#020A04")))])),
            look("big", "Giant Pixels", with(n(&[("cell", 24.0), ("dotSize", 80.0), ("glow", 120.0)]), vec![("mode".into(), ParamValue::Choice("color".into()))])),
        ],
        "blob" => vec![
            look("track", "Motion Tracker", vec![]),
            look("minimal", "Minimal Corners", with(n(&[("maxBlobs", 8.0)]), vec![("shape".into(), ParamValue::Choice("corners".into())), ("connectOn".into(), ParamValue::Bool(false)), ("labelsOn".into(), ParamValue::Bool(false))])),
            look("target", "Target Lock", with(n(&[("maxBlobs", 4.0), ("dimOutside", 40.0)]), vec![("shape".into(), ParamValue::Choice("brackets".into())), ("color".into(), ParamValue::Color(hex("#FF3B30")))])),
            look("data", "Data Nodes", with(n(&[("maxBlobs", 30.0)]), vec![("shape".into(), ParamValue::Choice("circle".into())), ("connect".into(), ParamValue::Choice("all".into())), ("label".into(), ParamValue::Choice("coords".into()))])),
            look("saliency", "Face Finder", with(n(&[("maxBlobs", 6.0)]), vec![("detect".into(), ParamValue::Choice("saliency".into()))])),
        ],
        _ => vec![],
    }
}
