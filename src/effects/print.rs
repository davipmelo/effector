use super::*;

pub fn halftone() -> Effect {
    Effect::new("halftone", "Halftone", Category::Stylize, |input, p, ctx| {
        let cell = ctx.px(p.num("cell", 8.0)).max(2.0);
        let toned = ctx.apply("k_dither_pre", &[input], &[p.f("brightness"), p.f("contrast"), 0.0, 1.0]);
        let smoothed = ctx.blur(&toned, cell * 0.5 * p.num("smooth", 40.0) / 100.0);
        let mode = match p.choice_or("mode", "bw").as_str() { "cmyk" => 1.0, "color" => 2.0, _ => 0.0 };
        let shape = match p.choice_or("shape", "circle").as_str() {
            "square" => 1.0, "diamond" => 2.0, "line" => 3.0, "ellipse" => 4.0, "cross" => 5.0, _ => 0.0,
        };
        let mut u = vec![cell as f32, p.fd("angle", 45.0), shape, mode, p.fd("dotScale", 100.0) / 100.0, p.fd("softness", 10.0) / 100.0 * 0.5];
        u.extend(p.color_or("paper", hex("#FFFFFF")).floats());
        u.extend(p.color_or("ink", hex("#111111")).floats());
        u.extend([p.fd("angC", 15.0), p.fd("angM", 75.0), p.fd("angY", 0.0), p.fd("angK", 45.0), p.fd("gcr", 80.0) / 100.0]);
        ctx.apply("k_halftone", &[&smoothed], &u)
    })
    .fine_pattern()
    .symbol("circle.hexagongrid")
    .summary("Print screen dots: B&W, CMYK or colour, with angles and shapes.")
    .params(vec![
        ParamDef::segmented("mode", "Mode", &[("bw", "B&W"), ("cmyk", "CMYK"), ("color", "Color")], "bw").sec("Screen"),
        ParamDef::choice("shape", "Dot Shape", &[("circle", "Circle"), ("square", "Square"), ("diamond", "Diamond"), ("line", "Line"), ("ellipse", "Ellipse"), ("cross", "Cross")], "circle").sec("Screen"),
        s("cell", "Cell Size", 2.0, 80.0, 8.0).step(0.5).unit("").curve(1.8).sec("Screen"),
        s("angle", "Angle", 0.0, 180.0, 45.0).step(1.0).unit("°").sec("Screen").vis(when_not("mode", "cmyk")),
        s("dotScale", "Dot Size", 20.0, 200.0, 100.0).step(1.0).unit("%").sec("Screen"),
        s("softness", "Softness", 0.0, 100.0, 10.0).step(1.0).unit("%").sec("Screen"),
        s("smooth", "Detail Smoothing", 0.0, 100.0, 40.0).step(1.0).unit("%").sec("Screen"),
        s("angC", "Cyan Angle", 0.0, 180.0, 15.0).step(1.0).unit("°").sec("CMYK Angles").vis(when("mode", "cmyk")),
        s("angM", "Magenta Angle", 0.0, 180.0, 75.0).step(1.0).unit("°").sec("CMYK Angles").vis(when("mode", "cmyk")),
        s("angY", "Yellow Angle", 0.0, 180.0, 0.0).step(1.0).unit("°").sec("CMYK Angles").vis(when("mode", "cmyk")),
        s("angK", "Black Angle", 0.0, 180.0, 45.0).step(1.0).unit("°").sec("CMYK Angles").vis(when("mode", "cmyk")),
        s("gcr", "Black Generation", 0.0, 100.0, 80.0).step(1.0).unit("%").sec("CMYK Angles").vis(when("mode", "cmyk")),
        ParamDef::color("paper", "Paper", hex("#FFFFFF")).sec("Colors"),
        ParamDef::color("ink", "Ink", hex("#111111")).sec("Colors").vis(when("mode", "bw")),
        s("contrast", "Contrast", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
        s("brightness", "Brightness", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
    ])
}
