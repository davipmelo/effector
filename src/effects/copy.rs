use super::*;

pub fn photocopy() -> Effect {
    Effect::new("xerox", "Photocopy", Category::Stylize, |input, p, ctx| {
        let mut img = ctx.apply("k_dither_pre", &[input], &[0.0, p.fd("contrast", 25.0), 1.0, 1.0]);
        let wear_on = p.bool_or("wearOn", true);
        let wear: f32 = if wear_on { 1.0 } else { 0.0 };
        let gens = if wear_on { p.num("generations", 1.0) as i32 } else { 1 };
        for g in 0..gens.max(0) {
            img = ctx.blur(&img, ctx.px(p.num("blur", 0.8)) * if g == 0 { 1.0 } else { 0.7 });
            let mut u = vec![
                p.fd("seed", 1.0) * 1.9 + g as f32 * 11.0,
                ctx.scale as f32,
                p.fd("threshold", 62.0) / 100.0,
                p.fd("softness", 6.0) / 100.0,
                p.fd("noise", 35.0) / 100.0 * wear,
                p.fd("streaks", 35.0) / 100.0 * wear,
                p.fd("edge", 45.0) / 100.0 * 0.8 * wear,
                p.fd("uneven", 40.0) / 100.0 * wear,
            ];
            u.extend(p.color_or("toner", hex("#111111")).floats());
            u.extend(p.color_or("paper", hex("#F2F2F2")).floats());
            u.extend([p.fd("specks", 30.0) / 100.0 * wear, p.b("dither")]);
            img = ctx.apply("k_xerox", &[&img], &u);
        }
        img
    })
    .hidden()
    .symbol("doc.on.doc")
    .summary("High-contrast toner copy with streaks, specks and dark edges (zines, flyers).")
    .params(vec![
        s("threshold", "Threshold", 0.0, 100.0, 62.0).step(1.0).unit("%").sec("Toner"),
        s("softness", "Softness", 0.0, 30.0, 6.0).step(0.5).unit("%").sec("Toner"),
        s("contrast", "Contrast", -100.0, 100.0, 25.0).step(1.0).sec("Toner"),
        s("blur", "Blur", 0.0, 6.0, 0.8).step(0.05).curve(1.6).sec("Toner"),
        ParamDef::toggle("dither", "Dither", false).sec("Toner"),
        ParamDef::section_switch("wearOn", "Wear", true),
        s("noise", "Toner Noise", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Wear"),
        s("uneven", "Uneven Density", 0.0, 100.0, 40.0).step(1.0).unit("%").sec("Wear"),
        s("streaks", "Drum Streaks", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Wear"),
        s("edge", "Edge Shadow", 0.0, 100.0, 45.0).step(1.0).unit("%").sec("Wear"),
        s("specks", "Background Specks", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Wear"),
        s("generations", "Copy Generations", 1.0, 6.0, 1.0).step(1.0).sec("Wear"),
        ParamDef::color("toner", "Toner", hex("#111111")).sec("Colors"),
        ParamDef::color("paper", "Paper", hex("#F2F2F2")).sec("Colors"),
        ParamDef::seed("seed").sec("Colors"),
    ])
}

pub fn thermal() -> Effect {
    Effect::new("receipt", "Thermal Receipt", Category::Stylize, |input, p, ctx| {
        let mut dp = LayerNode::effect("dither", None, None).params;
        dp.set("algorithm", ParamValue::Choice(p.choice_or("algorithm", "atkinson")));
        dp.set("pixelSize", ParamValue::Number(p.num("dots", 1.4)));
        dp.set("brightness", ParamValue::Number(p.num("brightness", 10.0)));
        dp.set("contrast", ParamValue::Number(p.num("contrast", 20.0)));
        dp.set("mode", ParamValue::Choice("mono".into()));
        dp.set("levels", ParamValue::Number(2.0));
        dp.set("dark", ParamValue::Color(Rgba::BLACK));
        dp.set("light", ParamValue::Color(Rgba::WHITE));
        let d = super::dither::apply_dither(input, &dp, ctx);
        let mut u = vec![p.fd("seed", 1.0) * 2.3, ctx.scale as f32];
        u.extend(p.color_or("ink", hex("#232323")).floats());
        u.extend(p.color_or("paper", hex("#F4F4F4")).floats());
        if p.bool_or("wearOn", true) {
            u.extend([p.fd("fade", 25.0) / 100.0, p.fd("dead", 25.0) / 100.0, p.fd("banding", 35.0) / 100.0, p.fd("gradient", 15.0) / 100.0]);
        } else {
            u.extend([0.0; 4]);
        }
        ctx.apply("k_thermal", &[&d], &u)
    })
    .hidden()
    .symbol("scroll")
    .summary("Receipt printer: 1-bit dots, faded thermal ink, dead print-head lines.")
    .params(vec![
        s("dots", "Dot Size", 0.5, 6.0, 1.4).step(0.1).curve(1.5).sec("Print"),
        ParamDef::choice("algorithm", "Dither", &[("atkinson", "Atkinson"), ("floyd", "Floyd–Steinberg"), ("bayer4", "Ordered")], "atkinson")
            .sec("Print"),
        s("brightness", "Brightness", -100.0, 100.0, 10.0).step(1.0).sec("Print"),
        s("contrast", "Contrast", -100.0, 100.0, 20.0).step(1.0).sec("Print"),
        ParamDef::section_switch("wearOn", "Wear", true),
        s("fade", "Fade", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Wear"),
        s("banding", "Banding", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Wear"),
        s("gradient", "Fade Down the Roll", 0.0, 100.0, 15.0).step(1.0).unit("%").sec("Wear"),
        s("dead", "Dead Lines", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Wear"),
        ParamDef::color("ink", "Ink", hex("#232323")).sec("Colors"),
        ParamDef::color("paper", "Paper", hex("#F4F4F4")).sec("Colors"),
        ParamDef::seed("seed").sec("Colors"),
    ])
}
