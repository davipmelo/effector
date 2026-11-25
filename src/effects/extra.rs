//! Effects added in 0.7: Split Tone, Reeded Glass, LED Display and Print Wear.

use super::*;

pub fn split_tone() -> Effect {
    Effect::new("splittone", "Split Tone", Category::Adjust, |input, p, ctx| {
        let mut u = vec![];
        u.extend(p.color_or("shadow", hex("#1F4E9E")).floats());
        u.extend(p.color_or("highlight", hex("#FFB46E")).floats());
        u.extend([
            p.fd("shadowAmt", 50.0) / 100.0,
            p.fd("highlightAmt", 30.0) / 100.0,
            p.f("balance") / 100.0,
            p.b("preserve"),
        ]);
        ctx.apply("k_splittone", &[input], &u)
    })
    .symbol("circle.lefthalf.filled")
    .summary("Tints shadows and highlights with two colours (cool shadows, warm highlights…).")
    .params(vec![
        ParamDef::color("shadow", "Shadow Color", hex("#1F4E9E")).sec("Shadows"),
        s("shadowAmt", "Amount", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Shadows"),
        ParamDef::color("highlight", "Highlight Color", hex("#FFB46E")).sec("Highlights"),
        s("highlightAmt", "Amount", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Highlights"),
        s("balance", "Balance", -100.0, 100.0, 0.0).step(1.0).sec("Balance"),
        ParamDef::toggle("preserve", "Preserve Brightness", true).sec("Balance"),
    ])
}

pub fn reeded_glass() -> Effect {
    Effect::new("reeded", "Reeded Glass", Category::Distort, |input, p, ctx| {
        let mut src = input.clone();
        // what is behind the glass is out of focus, smeared across the ribs
        let frost = p.num("frost", 2.5);
        if frost > 0.05 {
            src = ctx.blur(&src, ctx.px(frost));
        }
        let smear = ctx.px(p.num("smear", 24.0));
        if smear > 0.5 {
            let a = p.n("angle").to_radians();
            let (dx, dy) = ((a.cos() * smear) as f32, (a.sin() * smear) as f32);
            src = ctx.apply("k_dirblur", &[&src], &[dx, dy, 1.0, 0.0]);
        }
        let depth_amt = if p.bool("depthAware") { p.fd("depthAmount", 70.0) / 100.0 } else { 0.0 };
        let d = if depth_amt > 0.0 { ctx.depth() } else { None };
        let depth_amt = if d.is_some() { depth_amt } else { 0.0 };
        let dt = d.unwrap_or_else(|| input.clone());
        let kind = match p.choice_or("profile", "round").as_str() { "concave" => 1.0, "prism" => 2.0, "wave" => 3.0, _ => 0.0 };
        let u = vec![
            ctx.px(p.num("width", 12.0)).max(1.0) as f32,
            p.fd("refraction", 100.0) / 100.0,
            p.f("angle"),
            p.fd("dispersion", 25.0) / 100.0,
            p.fd("shading", 45.0) / 100.0,
            p.f("wave") / 100.0,
            ctx.px(p.num("waveScale", 60.0)).max(1.0) as f32,
            p.fd("seed", 1.0),
            kind,
            p.fd("highlight", 30.0) / 100.0,
            p.fd("groove", 35.0) / 100.0,
            p.fd("irregular", 15.0) / 100.0,
            depth_amt,
        ];
        ctx.apply("k_reeded", &[&src, &dt], &u)
    })
    .symbol("rectangle.split.3x1")
    .summary("Fluted / ribbed glass: every rib is a little cylindrical lens that squeezes and repeats what is behind it.")
    .depth(|p| p.bool("depthAware") || crate::effects::uses_depth_bias(p))
    .params(vec![
        ParamDef::segmented("profile", "Rib Shape", &[("round", "Round"), ("concave", "Fluted"), ("prism", "Prism"), ("wave", "Wave")], "round").sec("Glass"),
        s("width", "Rib Width", 2.0, 200.0, 12.0).step(0.5).curve(1.8).sec("Glass"),
        s("refraction", "Refraction", -200.0, 200.0, 100.0).step(1.0).unit("%").sec("Glass"),
        s("angle", "Angle", -90.0, 90.0, 0.0).step(1.0).unit("°").sec("Glass"),
        s("irregular", "Irregular Ribs", 0.0, 100.0, 15.0).step(1.0).unit("%").sec("Glass"),
        s("smear", "Smear Across Ribs", 0.0, 80.0, 24.0).step(0.5).curve(1.5).sec("Behind the Glass"),
        s("frost", "Defocus", 0.0, 20.0, 2.5).step(0.1).curve(1.6).sec("Behind the Glass"),
        s("dispersion", "Dispersion", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Light"),
        s("shading", "Rib Shading", 0.0, 100.0, 45.0).step(1.0).unit("%").sec("Light"),
        s("highlight", "Highlight", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Light"),
        s("groove", "Groove", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Light"),
        ParamDef::section_switch("depthAware", "Depth", false),
        s("depthAmount", "Far Things Distort More", 0.0, 100.0, 70.0).step(1.0).unit("%").sec("Depth"),
        s("wave", "Melt", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Melt"),
        s("waveScale", "Melt Scale", 5.0, 400.0, 60.0).step(1.0).curve(1.6).sec("Melt"),
        ParamDef::seed("seed").sec("Melt"),
    ])
    .looks(vec![
        look("window", "Bathroom Window", vals![]),
        look("fine", "Fine Reeds", vals!["width" => 6.0, "refraction" => 45.0, "smear" => 6.0, "groove" => 25.0]),
        look("wide", "Wide Flutes", vals!["width" => 40.0, "profile" => "concave", "refraction" => 80.0, "smear" => 18.0, "shading" => 55.0]),
        look("shower", "Shower Door", vals!["width" => 10.0, "refraction" => 120.0, "smear" => 40.0, "frost" => 5.0, "highlight" => 45.0]),
        look("close", "Pressed to the Glass", vals!["depthAware" => true, "depthAmount" => 100.0]),
        look("prism", "Prism Glass", vals!["profile" => "prism", "width" => 22.0, "refraction" => 70.0, "dispersion" => 60.0, "smear" => 4.0]),
        look("diagonal", "Diagonal Reeds", vals!["angle" => 35.0, "width" => 12.0]),
        look("melted", "Melted Glass", vals!["wave" => 60.0, "waveScale" => 90.0, "irregular" => 50.0, "profile" => "wave"]),
        look("subtle", "Subtle Overlay", vals!["refraction" => 30.0, "smear" => 6.0, "shading" => 25.0, "groove" => 15.0, "highlight" => 15.0, "frost" => 0.0]),
    ])
}

pub fn led_display() -> Effect {
    Effect::new("led", "LED Display", Category::Glitch, |input, p, ctx| {
        let cell = ctx.px(p.num("cell", 8.0)).max(1.5);
        let (w, h) = (input.width(), input.height());
        let (gw, gh) = (((w as f64 / cell).ceil() as usize).max(1), ((h as f64 / cell).ceil() as usize).max(1));
        let grid = ctx.resize(input, gw, gh, false);
        let mut u = vec![
            cell as f32,
            p.fd("dotSize", 70.0) / 100.0,
            p.fd("dotHeight", 100.0) / 100.0,
            p.f("threshold") / 100.0,
            p.fd("levels", 6.0),
            if p.choice_or("mode", "mono") == "color" { 1.0 } else { 0.0 },
            p.fd("unlit", 12.0) / 100.0,
            p.f("contrast") / 100.0,
            gw as f32,
            gh as f32,
        ];
        u.extend(p.color_or("color", hex("#38C6FF")).floats());
        u.extend(p.color_or("bg", hex("#04080F")).floats());
        let leds = ctx.apply_sized("k_led", &[&grid], &u, None, w, h);
        let glow = p.num("glow", 80.0) / 100.0;
        if glow > 0.005 {
            super::light::glow_pipeline(&leds, ctx, 0.15, 0.2, p.num("glowRadius", 10.0), glow as f32, Rgba::WHITE, false, 1.1, 1.0, true, false)
        } else {
            leds
        }
    })
    .symbol("circle.grid.3x3.fill")
    .summary("Dot-matrix LED screen: the image rebuilt from glowing LEDs.")
    .params(vec![
        s("cell", "LED Size", 2.0, 40.0, 8.0).step(0.5).curve(1.5).sec("LEDs"),
        s("dotSize", "Dot Size", 20.0, 100.0, 70.0).step(1.0).unit("%").sec("LEDs"),
        s("dotHeight", "Dot Height", 100.0, 400.0, 100.0).step(5.0).unit("%").sec("LEDs"),
        s("threshold", "Threshold", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("LEDs"),
        s("levels", "Brightness Levels", 2.0, 16.0, 6.0).step(1.0).sec("LEDs"),
        s("contrast", "Contrast", -100.0, 100.0, 0.0).step(1.0).sec("LEDs"),
        ParamDef::choice("mode", "Color", &[("mono", "Single Color"), ("color", "Image Colors")], "mono").sec("Color"),
        ParamDef::color("color", "LED Color", hex("#38C6FF")).sec("Color"),
        ParamDef::color("bg", "Panel", hex("#04080F")).sec("Color"),
        s("unlit", "Unlit LEDs", 0.0, 100.0, 12.0).step(1.0).unit("%").sec("Color"),
        s("glow", "Glow", 0.0, 200.0, 80.0).step(1.0).unit("%").sec("Glow"),
        s("glowRadius", "Glow Radius", 1.0, 60.0, 10.0).step(1.0).curve(1.6).sec("Glow"),
    ])
}

pub fn print_wear() -> Effect {
    Effect::new("printwear", "Print Wear", Category::Stylize, |input, p, ctx| {
        let mut u = vec![
            p.fd("seed", 1.0) * 2.7,
            ctx.scale as f32,
            p.f("fade") / 100.0,
            p.f("uneven") / 100.0,
            p.f("roll") / 100.0,
            p.f("streaks") / 100.0,
            p.f("banding") / 100.0,
            p.f("dead") / 100.0,
            p.f("noise") / 100.0,
            p.f("specks") / 100.0,
            p.f("edge") / 100.0,
        ];
        u.extend(p.color_or("paper", hex("#F2F2F2")).floats());
        u.extend(p.color_or("toner", hex("#111111")).floats());
        ctx.apply("k_printwear", &[input], &u)
    })
    .symbol("doc.on.doc")
    .summary("Worn printing: faded toner, streaks, dead print-head lines, specks, edge shadow.")
    .params(vec![
        s("fade", "Fade", 0.0, 100.0, 15.0).step(1.0).unit("%").sec("Density"),
        s("uneven", "Uneven Density", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Density"),
        s("roll", "Fade Down the Page", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Density"),
        s("streaks", "Drum Streaks", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Lines"),
        s("banding", "Banding", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Lines"),
        s("dead", "Dead Lines", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Lines"),
        s("noise", "Toner Noise", 0.0, 100.0, 25.0).step(1.0).unit("%").sec("Dirt"),
        s("specks", "Specks", 0.0, 100.0, 20.0).step(1.0).unit("%").sec("Dirt"),
        s("edge", "Edge Shadow", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Dirt"),
        ParamDef::color("paper", "Paper", hex("#F2F2F2")).sec("Colors"),
        ParamDef::color("toner", "Toner", hex("#111111")).sec("Colors"),
        ParamDef::seed("seed").sec("Colors"),
    ])
}
