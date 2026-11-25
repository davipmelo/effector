use super::light::glow_pipeline;
use super::*;

#[derive(Clone)]
pub struct FilmStock {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub bw: bool,
    pub saturation: f32,
    /// S-curve strength (1 = none)
    pub contrast: f32,
    /// black lift
    pub fade: f32,
    /// highlight ceiling
    pub white_clip: f32,
    /// per-channel gamma (colour cast)
    pub gamma: [f32; 3],
    pub shadow_tint: [f32; 3],
    pub highlight_tint: [f32; 3],
    pub bw_weights: [f32; 3],
    pub grain: f32,
    pub grain_size: f32,
    pub halation: f32,
    /// Channel mixing (rows = output r, g, b) for false-colour / cross stocks.
    pub matrix: [f32; 9],
}

impl FilmStock {
    const fn base(id: &'static str, name: &'static str) -> FilmStock {
        FilmStock {
            id,
            name,
            group: "Color Negative",
            bw: false,
            saturation: 1.0,
            contrast: 1.1,
            fade: 0.02,
            white_clip: 0.98,
            gamma: [1.0, 1.0, 1.0],
            shadow_tint: [0.0; 3],
            highlight_tint: [0.0; 3],
            bw_weights: [0.3, 0.59, 0.11],
            grain: 1.0,
            grain_size: 1.0,
            halation: 0.5,
            matrix: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }
}

pub const FILM_GROUPS: [&str; 6] = ["Color Negative", "Slide", "Cinema", "Instant", "Creative", "Black & White"];

fn list() -> Vec<FilmStock> {
    vec![
        FilmStock { saturation: 0.9, contrast: 1.04, fade: 0.025, gamma: [0.97, 1.0, 1.03], shadow_tint: [0.0, 0.004, 0.012], highlight_tint: [0.025, 0.012, -0.012], grain: 0.5, halation: 0.4, ..FilmStock::base("portra160", "Kodak Portra 160") },
        FilmStock { saturation: 0.94, contrast: 1.08, fade: 0.03, gamma: [0.96, 0.995, 1.03], shadow_tint: [0.0, 0.006, 0.014], highlight_tint: [0.03, 0.014, -0.016], grain: 0.8, halation: 0.5, ..FilmStock::base("portra400", "Kodak Portra 400") },
        FilmStock { saturation: 0.98, contrast: 1.1, fade: 0.035, gamma: [0.95, 0.99, 1.04], shadow_tint: [0.0, 0.006, 0.016], highlight_tint: [0.035, 0.016, -0.02], grain: 1.2, halation: 0.6, ..FilmStock::base("portra800", "Kodak Portra 800") },
        FilmStock { saturation: 1.28, contrast: 1.22, fade: 0.008, gamma: [0.98, 1.0, 0.99], shadow_tint: [-0.006, 0.0, 0.012], highlight_tint: [0.02, 0.006, -0.01], grain: 0.35, halation: 0.3, ..FilmStock::base("ektar100", "Kodak Ektar 100") },
        FilmStock { saturation: 1.12, contrast: 1.12, fade: 0.03, gamma: [0.94, 0.98, 1.06], shadow_tint: [0.008, 0.002, -0.004], highlight_tint: [0.045, 0.028, -0.035], grain: 0.9, halation: 0.6, ..FilmStock::base("gold200", "Kodak Gold 200") },
        FilmStock { saturation: 1.15, contrast: 1.16, fade: 0.03, gamma: [0.95, 0.98, 1.05], shadow_tint: [0.006, 0.004, 0.0], highlight_tint: [0.04, 0.02, -0.03], grain: 1.0, halation: 0.55, ..FilmStock::base("ultramax400", "Kodak UltraMax 400") },
        FilmStock { saturation: 1.05, contrast: 1.08, fade: 0.045, gamma: [0.93, 0.98, 1.08], shadow_tint: [0.01, 0.004, -0.006], highlight_tint: [0.05, 0.03, -0.04], grain: 1.0, halation: 0.6, ..FilmStock::base("colorplus200", "Kodak ColorPlus 200") },
        FilmStock { saturation: 0.88, contrast: 1.0, fade: 0.04, gamma: [1.02, 0.97, 0.98], shadow_tint: [-0.01, 0.018, 0.016], highlight_tint: [0.012, 0.01, 0.006], grain: 0.8, halation: 0.4, ..FilmStock::base("pro400h", "Fuji Pro 400H") },
        FilmStock { saturation: 1.08, contrast: 1.15, fade: 0.025, gamma: [1.01, 0.96, 0.99], shadow_tint: [-0.012, 0.012, 0.018], highlight_tint: [0.01, 0.012, -0.012], grain: 1.0, halation: 0.5, ..FilmStock::base("superia400", "Fuji Superia 400") },
        FilmStock { saturation: 1.0, contrast: 1.1, fade: 0.035, gamma: [1.02, 0.97, 0.98], shadow_tint: [-0.01, 0.014, 0.012], highlight_tint: [0.0, 0.012, 0.0], grain: 0.9, halation: 0.45, ..FilmStock::base("c200", "Fuji C200") },
        FilmStock { saturation: 1.18, contrast: 1.14, fade: 0.03, gamma: [0.97, 1.0, 1.02], shadow_tint: [0.0, 0.0, 0.02], highlight_tint: [0.03, -0.005, 0.0], grain: 0.9, halation: 0.5, ..FilmStock::base("vista200", "Agfa Vista 200") },
        FilmStock { saturation: 1.3, contrast: 1.25, fade: 0.04, gamma: [0.95, 1.0, 1.02], shadow_tint: [-0.01, 0.012, 0.03], highlight_tint: [0.04, 0.02, -0.02], grain: 1.5, halation: 0.8, ..FilmStock::base("lomo800", "Lomography 800") },
        FilmStock { saturation: 0.8, contrast: 0.95, fade: 0.09, white_clip: 0.94, gamma: [0.92, 1.0, 1.1], shadow_tint: [0.03, -0.01, 0.04], highlight_tint: [0.05, 0.03, -0.03], grain: 1.6, halation: 0.6, ..FilmStock::base("expired", "Expired Drugstore Film") },
        FilmStock { group: "Slide", saturation: 1.5, contrast: 1.35, fade: 0.0, white_clip: 1.0, gamma: [1.0, 1.01, 0.96], shadow_tint: [0.0, -0.004, 0.018], highlight_tint: [0.01, 0.0, -0.01], grain: 0.3, halation: 0.25, ..FilmStock::base("velvia50", "Fuji Velvia 50") },
        FilmStock { group: "Slide", saturation: 1.2, contrast: 1.22, fade: 0.005, white_clip: 1.0, gamma: [1.0, 0.99, 0.98], shadow_tint: [0.0, 0.0, 0.012], grain: 0.3, halation: 0.25, ..FilmStock::base("provia100f", "Fuji Provia 100F") },
        FilmStock { group: "Slide", saturation: 1.2, contrast: 1.25, fade: 0.01, white_clip: 1.0, gamma: [1.0, 0.99, 0.96], shadow_tint: [0.0, 0.0, 0.02], highlight_tint: [0.0, 0.005, 0.01], grain: 0.3, halation: 0.25, ..FilmStock::base("e100", "Kodak Ektachrome E100") },
        FilmStock { group: "Slide", saturation: 1.25, contrast: 1.3, fade: 0.01, gamma: [0.96, 1.0, 1.04], shadow_tint: [0.012, 0.0, -0.01], highlight_tint: [0.03, 0.015, -0.02], grain: 0.4, halation: 0.3, ..FilmStock::base("kodachrome64", "Kodachrome 64") },
        FilmStock { group: "Cinema", saturation: 1.02, contrast: 1.12, fade: 0.03, gamma: [1.06, 1.0, 0.92], shadow_tint: [-0.01, 0.006, 0.03], highlight_tint: [0.02, 0.0, -0.01], grain: 1.1, halation: 1.6, ..FilmStock::base("cinestill800t", "CineStill 800T") },
        FilmStock { group: "Cinema", saturation: 1.05, contrast: 1.1, fade: 0.02, gamma: [0.98, 1.0, 1.01], highlight_tint: [0.015, 0.008, -0.005], grain: 0.4, halation: 1.3, ..FilmStock::base("cinestill50d", "CineStill 50D") },
        FilmStock { group: "Cinema", saturation: 0.95, contrast: 1.02, fade: 0.04, gamma: [0.98, 1.0, 1.0], shadow_tint: [-0.01, 0.008, 0.016], highlight_tint: [0.02, 0.01, -0.01], grain: 0.5, halation: 0.6, ..FilmStock::base("vision250d", "Kodak Vision3 250D") },
        FilmStock { group: "Cinema", saturation: 0.95, contrast: 1.06, fade: 0.04, gamma: [1.04, 1.0, 0.95], shadow_tint: [-0.012, 0.008, 0.024], highlight_tint: [0.015, 0.0, -0.01], grain: 0.8, halation: 1.0, ..FilmStock::base("vision500t", "Kodak Vision3 500T") },
        FilmStock { group: "Cinema", saturation: 0.45, contrast: 1.45, fade: 0.02, gamma: [1.0, 0.99, 0.98], grain: 0.7, halation: 0.3, ..FilmStock::base("bleach", "Bleach Bypass") },
        FilmStock { group: "Instant", saturation: 0.85, contrast: 0.92, fade: 0.07, white_clip: 0.93, gamma: [0.96, 1.0, 1.06], shadow_tint: [0.0, 0.02, 0.03], highlight_tint: [0.04, 0.03, -0.02], grain: 0.4, halation: 0.4, ..FilmStock::base("polaroid600", "Polaroid 600") },
        FilmStock { group: "Instant", saturation: 0.75, contrast: 0.9, fade: 0.09, white_clip: 0.9, gamma: [0.92, 0.98, 1.1], shadow_tint: [0.03, 0.0, -0.01], highlight_tint: [0.06, 0.04, -0.03], grain: 0.5, halation: 0.5, ..FilmStock::base("sx70", "Polaroid SX-70") },
        FilmStock { group: "Instant", saturation: 1.05, contrast: 1.0, fade: 0.05, white_clip: 0.96, gamma: [1.0, 0.98, 0.97], shadow_tint: [-0.01, 0.01, 0.02], highlight_tint: [0.01, 0.02, 0.02], grain: 0.3, halation: 0.3, ..FilmStock::base("instax", "Instax Mini") },
        FilmStock { group: "Creative", saturation: 1.35, contrast: 1.35, fade: 0.03, gamma: [1.05, 0.92, 1.25], shadow_tint: [-0.02, 0.02, 0.05], highlight_tint: [0.05, 0.06, -0.08], grain: 1.0, halation: 0.5, ..FilmStock::base("crossprocess", "Cross Process (E-6 → C-41)") },
        FilmStock { group: "Creative", saturation: 1.15, contrast: 1.15, fade: 0.03, gamma: [0.98, 1.0, 1.0], grain: 1.1, halation: 0.6, matrix: [0.7, 0.35, -0.05, 0.05, 0.25, 0.7, 0.0, 0.6, 0.4], ..FilmStock::base("lomopurple", "LomoChrome Purple") },
        FilmStock { group: "Creative", saturation: 1.2, contrast: 1.18, fade: 0.02, grain: 0.9, halation: 0.6, matrix: [0.3, 0.8, -0.1, 0.85, 0.1, 0.05, 0.0, 0.3, 0.7], ..FilmStock::base("aerochrome", "Infrared Aerochrome") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.38, fade: 0.015, bw_weights: [0.27, 0.6, 0.13], grain: 1.4, grain_size: 1.2, halation: 0.3, ..FilmStock::base("trix400", "Kodak Tri-X 400") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.15, fade: 0.03, bw_weights: [0.3, 0.59, 0.11], grain: 1.2, grain_size: 1.1, halation: 0.3, ..FilmStock::base("hp5", "Ilford HP5 Plus") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.2, fade: 0.012, bw_weights: [0.28, 0.6, 0.12], grain: 0.6, grain_size: 0.9, halation: 0.2, ..FilmStock::base("fp4", "Ilford FP4 Plus 125") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.32, fade: 0.0, bw_weights: [0.26, 0.62, 0.12], grain: 0.3, grain_size: 0.75, halation: 0.15, ..FilmStock::base("panf50", "Ilford Pan F Plus 50") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.08, fade: 0.025, bw_weights: [0.3, 0.58, 0.12], grain: 0.35, grain_size: 0.8, halation: 0.25, ..FilmStock::base("xp2", "Ilford XP2 Super") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.3, fade: 0.01, bw_weights: [0.62, 0.35, 0.03], grain: 1.0, grain_size: 1.0, halation: 0.3, ..FilmStock::base("sfx200", "Ilford SFX 200 (Red Filter)") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.22, fade: 0.008, bw_weights: [0.25, 0.62, 0.13], grain: 0.4, grain_size: 0.8, halation: 0.2, ..FilmStock::base("tmax100", "Kodak T-Max 100") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.3, fade: 0.05, bw_weights: [0.28, 0.6, 0.12], grain: 2.0, grain_size: 1.5, halation: 0.4, ..FilmStock::base("tmax3200", "Kodak T-Max P3200") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.25, fade: 0.06, bw_weights: [0.32, 0.58, 0.1], grain: 2.2, grain_size: 1.6, halation: 0.4, ..FilmStock::base("delta3200", "Ilford Delta 3200") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.28, fade: 0.0, bw_weights: [0.24, 0.63, 0.13], grain: 0.35, grain_size: 0.8, halation: 0.2, ..FilmStock::base("acros100", "Fuji Acros 100") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.18, fade: 0.04, bw_weights: [0.32, 0.56, 0.12], grain: 1.6, grain_size: 1.3, halation: 0.35, ..FilmStock::base("fomapan400", "Fomapan 400") },
        FilmStock { group: "Black & White", bw: true, contrast: 1.4, fade: 0.0, bw_weights: [0.45, 0.5, 0.05], grain: 0.5, grain_size: 0.85, halation: 0.2, ..FilmStock::base("retro80s", "Rollei Retro 80S") },
    ]
}

/// Stocks ordered by group (the picker shows one section per group).
pub fn all_stocks() -> &'static Vec<FilmStock> {
    static S: std::sync::OnceLock<Vec<FilmStock>> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        let l = list();
        FILM_GROUPS.iter().flat_map(|g| l.iter().filter(|s| s.group == *g).cloned().collect::<Vec<_>>()).collect()
    })
}

pub fn stock(id: &str) -> FilmStock { all_stocks().iter().find(|s| s.id == id).cloned().unwrap_or_else(|| all_stocks()[1].clone()) }

pub fn film() -> Effect {
    Effect::new("film", "Film Emulation", Category::Film, |input, p, ctx| {
        let st = stock(&p.choice_or("stock", "portra400"));
        let intensity = p.fd("intensity", 100.0) / 100.0;
        let fade = st.fade + p.f("fade") / 100.0 * 0.25;
        let mut u: Vec<f32> = vec![if st.bw { 1.0 } else { 0.0 }, st.saturation, st.contrast, fade, st.white_clip];
        u.extend(st.gamma);
        u.extend(st.shadow_tint);
        u.extend(st.highlight_tint);
        u.extend(st.bw_weights);
        u.extend([intensity, p.f("exposure"), if st.bw { 0.0 } else { p.f("warmth") }]);
        u.extend(st.matrix);
        // Halation from the original highlights, before tone mapping
        let mut img = input.clone();
        let hal = if p.bool_or("halationOn", true) { p.fd("halation", 30.0) / 100.0 * st.halation * intensity } else { 0.0 };
        if hal > 0.001 {
            let tint = if st.bw { Rgba::WHITE } else { hex("#FF4418") };
            img = glow_pipeline(&img, ctx, 0.68, 0.15, 9.0, hal * 1.2, tint, true, 1.0, 0.0, true, st.bw);
        }
        img = ctx.apply("k_film", &[&img], &u);
        let g = if p.bool_or("grainOn", true) { p.num("grain", 35.0) / 100.0 * st.grain as f64 * intensity as f64 } else { 0.0 };
        if g > 0.001 {
            let mut gp = Params::default();
            gp.set("amount", ParamValue::Number(g * 100.0));
            gp.set("size", ParamValue::Number(p.num("grainSize", 1.0) * st.grain_size as f64));
            gp.set("roughness", ParamValue::Number(if st.bw { 55.0 } else { 40.0 }));
            gp.set("color", ParamValue::Bool(!st.bw));
            gp.set("colorAmount", ParamValue::Number(25.0));
            gp.set("seed", ParamValue::Number(p.num("seed", 1.0)));
            gp.set("shadows", ParamValue::Number(80.0));
            gp.set("midtones", ParamValue::Number(100.0));
            gp.set("highlights", ParamValue::Number(35.0));
            let gu = super::grain::uniforms(&gp, ctx, 1.0);
            img = ctx.apply("k_grain", &[&img], &gu);
        }
        img
    })
    .symbol("film")
    .summary("Famous film stocks: colour negative, slide, cinema, instant, creative and B&W.")
    .params(vec![
        ParamDef::choice_opts(
            "stock",
            "Film Stock",
            all_stocks().iter().map(|s| ChoiceOption { id: s.id.into(), label: s.name.into(), group: Some(s.group.into()) }).collect(),
            "portra400",
        )
        .sec("Film"),
        s("intensity", "Intensity", 0.0, 100.0, 100.0).step(1.0).unit("%").sec("Film"),
        s("exposure", "Exposure", -2.0, 2.0, 0.0).step(0.05).unit(" EV").sec("Film"),
        s("warmth", "Warmth", -100.0, 100.0, 0.0).step(1.0).sec("Film"),
        s("fade", "Fade", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Film"),
        ParamDef::section_switch("grainOn", "Grain", true),
        s("grain", "Amount", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Grain"),
        s("grainSize", "Size", 0.25, 4.0, 1.0).step(0.05).curve(1.8).sec("Grain"),
        ParamDef::seed("seed").sec("Grain"),
        ParamDef::section_switch("halationOn", "Halation", true),
        s("halation", "Amount", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Halation"),
    ])
}
