use std::sync::Arc;

use super::*;
use crate::swatches;

pub const DIFFUSION_KINDS: [&str; 7] = ["floyd", "atkinson", "jjn", "stucki", "sierra", "sierralite", "burkes"];

fn ordered_kind(a: &str) -> Option<f32> {
    Some(match a {
        "bayer2" => 0.0,
        "bayer4" => 1.0,
        "bayer8" => 2.0,
        "threshold" => 3.0,
        "random" => 4.0,
        "bluenoise" => 5.0,
        "lines" => 6.0,
        "cluster" => 7.0,
        _ => return None,
    })
}

pub fn apply_dither(input: &T, p: &Params, ctx: &mut Ctx) -> T {
    let mono = p.choice_or("mode", "mono") == "mono";
    // whole render pixels per dither pixel, so blocks are uniform (no seams)
    let pix = (ctx.px(p.num("pixelSize", 2.0)).round() as i64).max(1) as usize;
    let sw = input.width().div_ceil(pix).max(1);
    let sh = input.height().div_ceil(pix).max(1);
    let small = ctx.resize(input, sw, sh, false);
    let pre = ctx.apply("k_dither_pre", &[&small], &[p.f("brightness"), p.f("contrast"), if mono { 1.0 } else { 0.0 }, p.fd("gamma", 1.0)]);
    let palette = p.palette("palette").unwrap_or_else(|| swatches::all_palettes()[1].clone());
    let dark = p.color_or("dark", hex("#0A0A0A"));
    let light = p.color_or("light", hex("#F2F2F2"));
    let levels = p.num("levels", 2.0) as i32;
    let strength = (p.num("strength", 100.0) / 100.0) as f32;
    let algo = p.choice_or("algorithm", "floyd");
    let result = if let Some(kind) = ordered_kind(&algo) {
        let mut ex: Vec<f32> = vec![palette.colors.len() as f32];
        for c in &palette.colors {
            ex.extend(c.floats());
        }
        let mut u = vec![kind, if mono { 1.0 } else { 0.0 }, levels as f32, strength, p.fd("seed", 1.0)];
        u.extend(dark.floats());
        u.extend(light.floats());
        ctx.apply_ex("k_dither_ordered", &[&pre], &u, &ex)
    } else {
        let mut px = ctx.read_floats(&pre);
        error_diffusion(&mut px, sw, sh, &algo, p.bool_or("serpentine", true), strength, mono, levels, dark, light, &palette.colors);
        ctx.upload(&px, sw, sh)
    };
    ctx.upscale_blocks(&result, pix, input.width(), input.height())
}

pub fn dither() -> Effect {
    Effect::new("dither", "Dither", Category::Stylize, apply_dither)
        .symbol("circle.grid.3x3.fill")
        .summary("Classic dithering (ordered and error diffusion) with palettes and pixel size.")
        .params(vec![
            ParamDef::choice(
                "algorithm",
                "Algorithm",
                &[
                    ("floyd", "Floyd–Steinberg"),
                    ("atkinson", "Atkinson"),
                    ("jjn", "Jarvis–Judice–Ninke"),
                    ("stucki", "Stucki"),
                    ("sierra", "Sierra"),
                    ("sierralite", "Sierra Lite"),
                    ("burkes", "Burkes"),
                    ("bayer2", "Bayer 2×2"),
                    ("bayer4", "Bayer 4×4"),
                    ("bayer8", "Bayer 8×8"),
                    ("cluster", "Clustered Dot"),
                    ("bluenoise", "Blue Noise"),
                    ("random", "Random"),
                    ("lines", "Lines"),
                    ("threshold", "Threshold"),
                ],
                "floyd",
            )
            .sec("Dither"),
            s("pixelSize", "Pixel Size", 0.5, 24.0, 2.0).step(0.1).curve(1.8).sec("Dither"),
            s("strength", "Strength", 0.0, 100.0, 100.0).step(1.0).unit("%").sec("Dither"),
            ParamDef::toggle("serpentine", "Serpentine", true)
                .sec("Dither")
                .vis(Arc::new(|p: &Params| DIFFUSION_KINDS.contains(&p.choice("algorithm").as_str()))),
            ParamDef::seed("seed").sec("Dither"),
            ParamDef::segmented("mode", "Color Mode", &[("mono", "Mono"), ("palette", "Palette")], "mono").sec("Color"),
            s("levels", "Levels", 2.0, 8.0, 2.0).step(1.0).sec("Color").vis(when("mode", "mono")),
            ParamDef::color("dark", "Dark", hex("#0A0A0A")).sec("Color").vis(when("mode", "mono")),
            ParamDef::color("light", "Light", hex("#F2F2F2")).sec("Color").vis(when("mode", "mono")),
            ParamDef::palette("palette", "Palette", swatches::all_palettes()[1].clone()).sec("Color").vis(when("mode", "palette")),
            s("brightness", "Brightness", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
            s("contrast", "Contrast", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
            s("gamma", "Gamma", 0.2, 3.0, 1.0).step(0.01).sec("Tone"),
        ])
}

fn taps(k: &str) -> Vec<(i64, i64, f32)> {
    let mk = |l: &[(i64, i64, f32)], d: f32| l.iter().map(|t| (t.0, t.1, t.2 / d)).collect();
    match k {
        "atkinson" => mk(&[(1, 0, 1.0), (2, 0, 1.0), (-1, 1, 1.0), (0, 1, 1.0), (1, 1, 1.0), (0, 2, 1.0)], 8.0),
        "jjn" => mk(
            &[
                (1, 0, 7.0), (2, 0, 5.0), (-2, 1, 3.0), (-1, 1, 5.0), (0, 1, 7.0), (1, 1, 5.0), (2, 1, 3.0), (-2, 2, 1.0), (-1, 2, 3.0),
                (0, 2, 5.0), (1, 2, 3.0), (2, 2, 1.0),
            ],
            48.0,
        ),
        "stucki" => mk(
            &[
                (1, 0, 8.0), (2, 0, 4.0), (-2, 1, 2.0), (-1, 1, 4.0), (0, 1, 8.0), (1, 1, 4.0), (2, 1, 2.0), (-2, 2, 1.0), (-1, 2, 2.0),
                (0, 2, 4.0), (1, 2, 2.0), (2, 2, 1.0),
            ],
            42.0,
        ),
        "sierra" => mk(
            &[(1, 0, 5.0), (2, 0, 3.0), (-2, 1, 2.0), (-1, 1, 4.0), (0, 1, 5.0), (1, 1, 4.0), (2, 1, 2.0), (-1, 2, 2.0), (0, 2, 3.0), (1, 2, 2.0)],
            32.0,
        ),
        "sierralite" => mk(&[(1, 0, 2.0), (-1, 1, 1.0), (0, 1, 1.0)], 4.0),
        "burkes" => mk(&[(1, 0, 8.0), (2, 0, 4.0), (-2, 1, 2.0), (-1, 1, 4.0), (0, 1, 8.0), (1, 1, 4.0), (2, 1, 2.0)], 32.0),
        _ => mk(&[(1, 0, 7.0), (-1, 1, 3.0), (0, 1, 5.0), (1, 1, 1.0)], 16.0),
    }
}

/// In-place error diffusion on an RGBA float buffer.
#[allow(clippy::too_many_arguments)]
pub fn error_diffusion(
    b: &mut [f32], w: usize, h: usize, kernel: &str, serpentine: bool, strength: f32, mono: bool, levels: i32, dark: Rgba, light: Rgba,
    palette: &[Rgba],
) {
    let taps = taps(kernel);
    let pal: Vec<[f32; 3]> = palette.iter().map(|c| c.floats()).collect();
    let ll = (levels.max(2) - 1) as f32;
    let d = dark.floats();
    let l = light.floats();
    let (wi, hi) = (w as i64, h as i64);
    if mono {
        // single channel (r) — grey already neutral
        let mut g: Vec<f32> = (0..w * h).map(|i| b[i * 4]).collect();
        for y in 0..hi {
            let rev = serpentine && y % 2 == 1;
            for xi in 0..wi {
                let x = if rev { wi - 1 - xi } else { xi };
                let i = (y * wi + x) as usize;
                let old = g[i];
                let q = (old * ll).round().clamp(0.0, ll) / ll;
                g[i] = q;
                let err = (old - q) * strength;
                for t in &taps {
                    let nx = x + if rev { -t.0 } else { t.0 };
                    let ny = y + t.1;
                    if nx >= 0 && nx < wi && ny < hi {
                        g[(ny * wi + nx) as usize] += err * t.2;
                    }
                }
            }
        }
        for i in 0..w * h {
            let q = g[i].clamp(0.0, 1.0);
            for c in 0..3 {
                b[i * 4 + c] = d[c] + (l[c] - d[c]) * q;
            }
        }
    } else {
        for y in 0..hi {
            let rev = serpentine && y % 2 == 1;
            for xi in 0..wi {
                let x = if rev { wi - 1 - xi } else { xi };
                let i = ((y * wi + x) * 4) as usize;
                let old = [b[i], b[i + 1], b[i + 2]];
                let mut best = pal.first().copied().unwrap_or([0.0; 3]);
                let mut bd = f32::INFINITY;
                for c in &pal {
                    let e = [old[0] - c[0], old[1] - c[1], old[2] - c[2]];
                    let dist = e[0] * e[0] * 0.299 + e[1] * e[1] * 0.587 + e[2] * e[2] * 0.114;
                    if dist < bd {
                        bd = dist;
                        best = *c;
                    }
                }
                b[i] = best[0];
                b[i + 1] = best[1];
                b[i + 2] = best[2];
                let err = [(old[0] - best[0]) * strength, (old[1] - best[1]) * strength, (old[2] - best[2]) * strength];
                for t in &taps {
                    let nx = x + if rev { -t.0 } else { t.0 };
                    let ny = y + t.1;
                    if nx >= 0 && nx < wi && ny < hi {
                        let j = ((ny * wi + nx) * 4) as usize;
                        b[j] += err[0] * t.2;
                        b[j + 1] += err[1] * t.2;
                        b[j + 2] += err[2] * t.2;
                    }
                }
            }
        }
    }
}
