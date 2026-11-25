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
__STOCKS__
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
