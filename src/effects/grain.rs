use super::*;

pub fn uniforms(p: &Params, ctx: &Ctx, amount_scale: f64) -> Vec<f32> {
    vec![
        (p.num("amount", 35.0) / 100.0 * amount_scale) as f32,
        ctx.px(p.num("size", 1.0)).max(0.35) as f32,
        p.fd("roughness", 45.0) / 100.0,
        p.b("color"),
        p.fd("shadows", 70.0) / 100.0,
        p.fd("midtones", 100.0) / 100.0,
        p.fd("highlights", 40.0) / 100.0,
        p.fd("seed", 1.0) * 13.7,
        p.fd("colorAmount", 50.0) / 100.0,
    ]
}

pub fn grain() -> Effect {
    Effect::new("grain", "Grain", Category::Film, |input, p, ctx| {
        let u = uniforms(p, ctx, 1.0);
        ctx.apply("k_grain", &[input], &u)
    })
    .symbol("aqi.high")
    .summary("Film-like grain that responds to shadows, midtones and highlights.")
    .params(vec![
        s("amount", "Amount", 0.0, 100.0, 35.0).step(1.0).unit("%").curve(1.4).sec("Grain"),
        s("size", "Size", 0.25, 6.0, 1.0).step(0.05).curve(2.2).sec("Grain"),
        s("roughness", "Roughness", 0.0, 100.0, 45.0).step(1.0).unit("%").sec("Grain"),
        ParamDef::toggle("color", "Color Grain", false).sec("Grain"),
        s("colorAmount", "Color Amount", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Grain").vis(when_on("color")),
        ParamDef::seed("seed").sec("Grain"),
        s("shadows", "Shadows", 0.0, 100.0, 70.0).step(1.0).unit("%").sec("Response"),
        s("midtones", "Midtones", 0.0, 100.0, 100.0).step(1.0).unit("%").sec("Response"),
        s("highlights", "Highlights", 0.0, 100.0, 40.0).step(1.0).unit("%").sec("Response"),
    ])
}
