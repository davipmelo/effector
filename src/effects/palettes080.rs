//! Ready-made colour combinations for effects with colour pickers (one click sets every colour of
//! the set), plus the extra gradient / palette groups added in 0.8.0. Curated per effect.

use crate::model::*;

/// Colours that are picked together (e.g. Light Leak A + B) and the named combinations offered.
pub struct ColorSet {
    pub keys: &'static [&'static str],
    /// (group, name, one hex per key)
    pub entries: Vec<(&'static str, &'static str, Vec<&'static str>)>,
}

fn set(keys: &'static [&'static str], entries: &[(&'static str, &'static str, &[&'static str])]) -> ColorSet {
    ColorSet { keys, entries: entries.iter().map(|(g, n, h)| (*g, *n, h.to_vec())).collect() }
}

/// Ink on paper (dark, light) combos shared by print-like effects.
const INK: &[(&str, &str, &[&str])] = &[
    ("Classic", "Black & White", &["#000000", "#FFFFFF"]),
    ("Classic", "Paper & Ink", &["#1B1B1B", "#EEE7D7"]),
    ("Classic", "Newsprint", &["#1C1C1C", "#EDE8DC"]),
    ("Classic", "Receipt", &["#2E2B28", "#F3F0E8"]),
    ("Classic", "Kraft Paper", &["#2A1B10", "#C9A77C"]),
    ("Riso", "Riso Blue", &["#3255A4", "#F4F0E6"]),
    ("Riso", "Riso Fluo Pink", &["#FF48B0", "#F4F0E6"]),
    ("Riso", "Riso Teal", &["#00838A", "#F4F0E6"]),
    ("Riso", "Riso Red", &["#E8364F", "#F7F1E3"]),
    ("Riso", "Riso Green", &["#00A95C", "#F4F0E6"]),
    ("Riso", "Riso Purple", &["#765BA7", "#F4F0E6"]),
    ("Screens", "Amber Monitor", &["#1A0D00", "#FFB000"]),
    ("Screens", "Green Phosphor", &["#001A00", "#33FF33"]),
    ("Screens", "Game Boy", &["#0F380F", "#9BBC0F"]),
    ("Screens", "LCD Watch", &["#2B3326", "#A7B597"]),
    ("Screens", "Blueprint", &["#0D3B78", "#E6F0FF"]),
    ("Screens", "Cyanotype", &["#0A1E3C", "#E8F0F4"]),
    ("Pop", "Neon Pink", &["#0A0A12", "#FF2E97"]),
    ("Pop", "Acid", &["#101010", "#C6FF00"]),
    ("Pop", "Yellow Effector", &["#0A0A0A", "#FFBB00"]),
    ("Pop", "Ultramarine", &["#0B0B3B", "#C9D6FF"]),
    ("Pop", "Sepia", &["#2B1A0E", "#E8D6B5"]),
];

fn swap(v: &[(&'static str, &'static str, &'static [&'static str])]) -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    v.iter().map(|(g, n, h)| (*g, *n, h.iter().rev().copied().collect())).collect()
}

pub fn color_sets(effect: &str) -> Vec<ColorSet> {
    match effect {
        "threshold" => vec![ColorSet { keys: &["dark", "light"], entries: INK.iter().map(|(g, n, h)| (*g, *n, h.to_vec())).collect() }],
        "dither" => vec![ColorSet { keys: &["dark", "light"], entries: INK.iter().map(|(g, n, h)| (*g, *n, h.to_vec())).collect() }],
        "ascii" => vec![ColorSet { keys: &["fg", "bg"], entries: swap(INK) }],
        "halftone" => vec![ColorSet { keys: &["paper", "ink"], entries: swap(INK) }],
        "printwear" => vec![set(&["paper", "toner"], &[
            ("Copier", "Office Copy", &["#F2F2F2", "#111111"]),
            ("Copier", "Recycled Paper", &["#E6E0D2", "#1E1E1E"]),
            ("Copier", "Old Fax", &["#E9ECEC", "#2B3540"]),
            ("Copier", "Carbon Copy", &["#F3F1EC", "#22306B"]),
            ("Copier", "Thermal Receipt", &["#F4F4F4", "#3A3A3A"]),
            ("Zine", "Pink Zine", &["#F9C9D6", "#141414"]),
            ("Zine", "Yellow Flyer", &["#FFE45C", "#141414"]),
            ("Zine", "Blue Flyer", &["#A9D3F5", "#141414"]),
            ("Zine", "Riso Red", &["#F7F1E3", "#E8364F"]),
            ("Zine", "Riso Blue", &["#F4F0E6", "#3255A4"]),
            ("Zine", "Brown Kraft", &["#C9A77C", "#22160D"]),
            ("Zine", "Inverted", &["#141414", "#EDEDED"]),
        ])],
        "leak" => vec![set(&["colorA", "colorB"], &[
            ("Warm", "Amber", &["#FF4A1C", "#FFC23A"]),
            ("Warm", "Kodak Red", &["#E0100F", "#FF5A1A"]),
            ("Warm", "Sunset", &["#FF2E63", "#FFB347"]),
            ("Warm", "Golden", &["#FF8A00", "#FFE7A3"]),
            ("Warm", "Expired Orange", &["#E8501A", "#F7C08A"]),
            ("Wild", "Magenta Burn", &["#FF1E6E", "#FF8A3D"]),
            ("Wild", "Lomo Purple", &["#8A2BE2", "#FF6EC7"]),
            ("Wild", "Cross Process", &["#00C2A8", "#E8FF5A"]),
            ("Wild", "Infrared", &["#FF0040", "#FF9AD5"]),
            ("Cool", "Cool Haze", &["#3AA0FF", "#C6E8FF"]),
            ("Cool", "Fuji Green", &["#1FB37A", "#C9F5B0"]),
            ("Cool", "Teal Dawn", &["#007C91", "#9FF0E6"]),
        ])],
        "halation" => vec![set(&["inner", "color"], &[
            ("Film", "CineStill Red", &["#FF7A2E", "#FF2A0A"]),
            ("Film", "Classic Orange", &["#FFB066", "#FF5A00"]),
            ("Film", "Warm Amber", &["#FFC27A", "#FF7A1A"]),
            ("Film", "Deep Red", &["#FF5A3A", "#B00000"]),
            ("Stylized", "Magenta Bloom", &["#FF8AD8", "#C21E8F"]),
            ("Stylized", "Blue Night", &["#9FC4FF", "#2A5BFF"]),
            ("Stylized", "Gold", &["#FFE7A0", "#E0A100"]),
            ("Stylized", "Toxic Green", &["#C8FF9A", "#2BB800"]),
        ])],
        "trail" => vec![set(&["colorA", "colorB"], &[
            ("Prism", "Pink → Blue", &["#FF3D7F", "#3DA9FF"]),
            ("Prism", "Red → Cyan", &["#FF2A2A", "#2AF0FF"]),
            ("Prism", "Fire → Ice", &["#FF7A00", "#7FD8FF"]),
            ("Prism", "Gold → Violet", &["#FFC400", "#7A3DFF"]),
            ("Prism", "Aurora", &["#3DFFB0", "#7A3DFF"]),
            ("Soft", "Mint → Coral", &["#7FFFD4", "#FF7F7F"]),
            ("Soft", "Peach → Lilac", &["#FFC9A3", "#C8A8FF"]),
            ("Soft", "Vaporwave", &["#FF4FD8", "#4FE3FF"]),
            ("Mono", "White → White", &["#FFFFFF", "#FFFFFF"]),
            ("Mono", "Warm Light", &["#FFE7C2", "#FF9A3D"]),
        ])],
        "led" => vec![set(&["color", "bg"], &[
            ("Signs", "Cyan Billboard", &["#38C6FF", "#04080F"]),
            ("Signs", "Bus Sign Amber", &["#FFA51F", "#0D0700"]),
            ("Signs", "Stadium Red", &["#FF2E2E", "#0B0202"]),
            ("Signs", "Matrix Green", &["#39FF6A", "#020A04"]),
            ("Signs", "Ice White", &["#EAF6FF", "#05070A"]),
            ("Signs", "Vapor Pink", &["#FF4FD8", "#0C0410"]),
            ("Signs", "Tokyo Purple", &["#B36BFF", "#06030D"]),
            ("Signs", "Gold", &["#FFD54A", "#0A0700"]),
        ])],
        "relief" => vec![set(&["nearColor", "farColor"], &[
            ("Depth", "Heat", &["#FFD23D", "#3A0CA3"]),
            ("Depth", "Warm Near, Cool Far", &["#FFB070", "#5A7BFF"]),
            ("Depth", "Infrared", &["#FF4D6D", "#1B1B3A"]),
            ("Depth", "Ocean", &["#9CF0E0", "#03045E"]),
            ("Depth", "Forest", &["#D9F99D", "#14532D"]),
            ("Depth", "Fog", &["#FFFFFF", "#6B7280"]),
            ("Depth", "Neon", &["#FF00A0", "#00F0FF"]),
        ])],
        "anamorphic" => vec![set(&["color"], &[
            ("Streaks", "Classic Blue", &["#3D7CFF"]),
            ("Streaks", "Cyan", &["#3DE0FF"]),
            ("Streaks", "Amber", &["#FF9A3D"]),
            ("Streaks", "Magenta", &["#FF3DC8"]),
            ("Streaks", "Gold", &["#FFC83D"]),
            ("Streaks", "Teal", &["#2AD1B0"]),
            ("Streaks", "Violet", &["#8A5CFF"]),
            ("Streaks", "White", &["#FFFFFF"]),
        ])],
        "lensemu" => vec![set(&["streakColor"], &[
            ("Streaks", "Classic Blue", &["#3D7CFF"]),
            ("Streaks", "Amber", &["#FF9A3D"]),
            ("Streaks", "Magenta", &["#FF3DC8"]),
            ("Streaks", "Gold", &["#FFC83D"]),
        ]), set(&["rendering"], &[
            ("Rendering", "Neutral", &["#FFFFFF"]),
            ("Rendering", "Warm Vintage", &["#FFE3B8"]),
            ("Rendering", "Soviet Yellow", &["#FFE9A0"]),
            ("Rendering", "Cool Modern", &["#DDE9FF"]),
            ("Rendering", "Green Cast", &["#E6FFD6"]),
            ("Rendering", "Rose", &["#FFDDE6"]),
        ])],
        "star" => vec![set(&["tint"], &[
            ("Tint", "White", &["#FFFFFF"]),
            ("Tint", "Warm", &["#FFD9A8"]),
            ("Tint", "Cool", &["#C9DCFF"]),
            ("Tint", "Rose", &["#FFC2D9"]),
            ("Tint", "Gold", &["#FFE38A"]),
            ("Tint", "Mint", &["#C7FFE8"]),
        ])],
        "glow" | "bloom" => vec![set(&["tint"], &[
            ("Tint", "Neutral", &["#FFFFFF"]),
            ("Tint", "Warm Haze", &["#FFC48A"]),
            ("Tint", "Golden", &["#FFCC80"]),
            ("Tint", "Night Blue", &["#A9C4FF"]),
            ("Tint", "Pink Dream", &["#FFB3D9"]),
            ("Tint", "Mint", &["#B8FFE0"]),
            ("Tint", "Red Room", &["#FF6A5A"]),
        ])],
        "vignette" => vec![set(&["color"], &[
            ("Edge", "Black", &["#000000"]),
            ("Edge", "White", &["#FFFFFF"]),
            ("Edge", "Warm Brown", &["#3B2414"]),
            ("Edge", "Deep Blue", &["#0B1838"]),
            ("Edge", "Burgundy", &["#3A0A16"]),
            ("Edge", "Cream", &["#F1E7D0"]),
        ])],
        "camcorder" => vec![set(&["nightTint"], &[
            ("NightShot", "Classic Green", &["#8CFFD2"]),
            ("NightShot", "Phosphor Green", &["#5CFF6A"]),
            ("NightShot", "Cyan", &["#8CE8FF"]),
            ("NightShot", "Mono White", &["#E8F0F0"]),
            ("NightShot", "Amber", &["#FFC26A"]),
        ]), set(&["dateColor"], &[
            ("Date", "White", &["#FFFFFF"]),
            ("Date", "LED Orange", &["#FF8A1E"]),
            ("Date", "Amber", &["#FFB000"]),
            ("Date", "Yellow", &["#FFE600"]),
            ("Date", "Red", &["#FF3B30"]),
            ("Date", "Green", &["#4CFF4C"]),
        ])],
        "frame" => vec![set(&["color", "barColor"], &[
            ("Frames", "Gallery White", &["#F5F4F0", "#000000"]),
            ("Frames", "Pure White", &["#FFFFFF", "#000000"]),
            ("Frames", "Black Box", &["#0A0A0A", "#000000"]),
            ("Frames", "Cream Polaroid", &["#F1EBDD", "#111111"]),
            ("Frames", "Kraft", &["#C9A77C", "#2A1B10"]),
            ("Frames", "Pastel Pink", &["#F6D6DE", "#1B1B1B"]),
            ("Frames", "Sky Blue", &["#D3E7F5", "#0B1D2E"]),
        ])],
        "blob" => vec![set(&["color"], &[
            ("Markers", "White", &["#FFFFFF"]),
            ("Markers", "Target Red", &["#FF3B30"]),
            ("Markers", "Terminal Green", &["#39FF6A"]),
            ("Markers", "Cyan", &["#2AE6FF"]),
            ("Markers", "Yellow", &["#FFD400"]),
            ("Markers", "Magenta", &["#FF2EC8"]),
        ])],
        _ => vec![],
    }
}

/// Gradient groups added in 0.8.0 (appended to the factory library).
pub fn extra_gradient_groups() -> Vec<(&'static str, Vec<GradientSpec>)> {
    vec![
        ("Cinema", vec![
            GradientSpec::from("Teal & Orange", &["#06141B", "#0E4D5C", "#7A9E9F", "#E8A86B", "#FFE9CF"]),
            GradientSpec::from("Bleach Bypass", &["#0B0C0C", "#3B4141", "#9DA3A0", "#E9ECE6"]),
            GradientSpec::from("Neon Noir", &["#05010F", "#2B0B5C", "#C2185B", "#00E5FF", "#F2FFFF"]),
            GradientSpec::from("Matrix Green", &["#000A04", "#0B3D1C", "#3FA65B", "#D9FFD6"]),
            GradientSpec::from("Day for Night", &["#020814", "#0F2A55", "#4C6FA8", "#C9D8F0"]),
            GradientSpec::from("Sodium Street", &["#0A0602", "#4A2306", "#D9741A", "#FFD08A"]),
        ]),
        ("Infrared & Iridescent", vec![
            GradientSpec::from("Aerochrome", &["#0A0208", "#5C0A2E", "#E0285A", "#FF9AC0", "#FFF3E8"]),
            GradientSpec::from("Infrared Mono", &["#000000", "#2E2E2E", "#BDBDBD", "#FFFFFF"]),
            GradientSpec::from("Oil Slick", &["#0B0420", "#3A0CA3", "#2EC4B6", "#F9C80E", "#F86624", "#EA3546"]),
            GradientSpec::from("Opal", &["#E8F4FF", "#BDE0FE", "#FFC8DD", "#CDB4DB", "#FFFFFF"]),
            GradientSpec::from("Pearl", &["#2B2B33", "#8A8FA3", "#E0D6E8", "#F7F1F8"]),
            GradientSpec::from("Beetle", &["#06120E", "#0B5D4A", "#2BA84A", "#8FD14F", "#F0F3A8"]),
        ]),
        ("Riso Duotones", vec![
            GradientSpec::from("Riso Blue & Yellow", &["#3255A4", "#FFE800"]),
            GradientSpec::from("Riso Pink & Teal", &["#00838A", "#FF48B0"]),
            GradientSpec::from("Riso Purple & Orange", &["#765BA7", "#FF6C2F"]),
            GradientSpec::from("Riso Green & Pink", &["#00A95C", "#FF48B0"]),
            GradientSpec::from("Riso Burgundy", &["#3A0A16", "#E8364F", "#F7F1E3"]),
        ]),
        ("Fashion", vec![
            GradientSpec::from("Hot Pink", &["#0A0006", "#B0005E", "#FF4FA3", "#FFE3F1"]),
            GradientSpec::from("Lilac Haze", &["#1C1430", "#6D5BA8", "#C9B8F0", "#F6F0FF"]),
            GradientSpec::from("Chrome Silver", &["#0A0A0A", "#5A5A60", "#C8CAD0", "#FFFFFF", "#9EA2AA"]),
            GradientSpec::from("Champagne", &["#1E140A", "#7A5A36", "#D9BE91", "#FFF6E6"]),
            GradientSpec::from("Cobalt", &["#020617", "#1E3A8A", "#3B82F6", "#DBEAFE"]),
        ]),
    ]
}

/// Palette groups added in 0.8.0.
pub fn extra_palette_groups() -> Vec<(&'static str, Vec<PaletteSpec>)> {
    vec![
        ("Risograph Inks", vec![
            PaletteSpec::from("Riso Blue/Yellow", &["#F4F0E6", "#3255A4", "#FFE800"]),
            PaletteSpec::from("Riso Purple/Orange", &["#F4F0E6", "#765BA7", "#FF6C2F"]),
            PaletteSpec::from("Riso Green/Red", &["#F4F0E6", "#00A95C", "#E8364F"]),
            PaletteSpec::from("Riso Black/Fluo", &["#F4F0E6", "#1A1A1A", "#FF48B0", "#FFE800"]),
            PaletteSpec::from("Riso Burgundy/Aqua", &["#F7F1E3", "#914E72", "#5EC8E5"]),
        ]),
        ("Pop & Comics", vec![
            PaletteSpec::from("Pop Art", &["#FFFFFF", "#000000", "#FFE600", "#FF1F1F", "#0057FF"]),
            PaletteSpec::from("Comic CMY", &["#FFFDF2", "#1A1A1A", "#00AEEF", "#EC008C", "#FFF200"]),
            PaletteSpec::from("Bauhaus", &["#F2EDE4", "#111111", "#D12F1F", "#2D58A6", "#F2B807"]),
            PaletteSpec::from("Memphis", &["#FFFFFF", "#111111", "#FF5DA2", "#2BD1C4", "#FFD23F", "#6E44FF"]),
            PaletteSpec::from("Hi-Vis", &["#111111", "#DFFF00", "#FF6B00", "#F2F2F2"]),
        ]),
        ("Screens & Terminals", vec![
            PaletteSpec::from("Amber Terminal", &["#140A00", "#6B3A00", "#C57A00", "#FFB000"]),
            PaletteSpec::from("Green Terminal", &["#001200", "#0A4D0A", "#1FA31F", "#5CFF5C"]),
            PaletteSpec::from("IBM 5153 Blue", &["#00002A", "#0000AA", "#5555FF", "#AAAAFF"]),
            PaletteSpec::from("LCD Pager", &["#2B3326", "#55634A", "#7F8E6E", "#A7B597"]),
            PaletteSpec::from("Plasma TV", &["#0A0010", "#6A00A8", "#E16462", "#FCA636", "#F0F921"]),
        ]),
        ("Film & Photo", vec![
            PaletteSpec::from("Kodachrome", &["#1B1A17", "#7A3A1C", "#D9A441", "#3F6E8C", "#E9E2D0"]),
            PaletteSpec::from("Polaroid", &["#2A2A2A", "#8E6E53", "#D7B58E", "#8FB3B0", "#F3EAD7"]),
            PaletteSpec::from("Infrared", &["#000000", "#5C0A2E", "#E0285A", "#FF9AC0", "#FFFFFF"]),
            PaletteSpec::from("Cyanotype Tones", &["#0A1E3C", "#1F4F8C", "#6E9BC8", "#E8F0F4"]),
            PaletteSpec::from("Sepia Tones", &["#1A0F08", "#704A2C", "#B58A5A", "#F8F0E0"]),
        ]),
    ]
}
