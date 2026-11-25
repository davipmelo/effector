use super::*;

pub fn fisheye() -> Effect {
    Effect::new("fisheye", "Fisheye", Category::Distort, |input, p, ctx| {
        let k = (p.fd("amount", 45.0) / 100.0 * 0.9) as f64;
        let asp = input.width() as f64 / input.height() as f64;
        let true_fish = p.choice_or("projection", "barrel") == "fisheye";
        let f = |r: f64| 1.0 + k * r * r + k * 0.5 * r * r * r * r;
        let mut auto = 1.0;
        if !true_fish && p.bool_or("autoZoom", true) {
            if p.bool_or("circle", false) {
                auto = f(0.5 * asp.min(1.0));
            } else if k > 0.0 {
                auto = f((asp * asp / 4.0 + 0.25).sqrt());
            }
        }
        let zoom = (p.num("zoom", 100.0) / 100.0 * auto) as f32;
        let fringe = if p.bool_or("fringeOn", true) { p.fd("fringe", 10.0) / 100.0 * 0.08 } else { 0.0 };
        let out = ctx.apply("k_fisheye", &[input], &[k as f32, zoom, p.b("circle"), fringe, p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0,
            if true_fish { 1.0 } else { 0.0 }, p.fd("fov", 180.0) * std::f32::consts::PI / 180.0,
            if p.choice_or("edge", "mirror") == "mirror" { 1.0 } else { 0.0 }]);
        if !p.bool_or("edgeBlurOn", false) {
            return out;
        }
        let b = ctx.blur(&out, ctx.px(p.num("edgeBlurRadius", 14.0)));
        ctx.apply("k_maskmix", &[&out, &b], &[1.0, p.fd("edgeBlurCenter", 40.0) / 100.0, 0.5, 0.0, p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0,
            p.fd("edgeBlur", 70.0) / 100.0])
    })
    .symbol("circle.dotted")
    .summary("Barrel or true fisheye projection up to extreme angles, with an optional circular frame — skate-video style.")
    .params(vec![
        ParamDef::segmented("projection", "Projection", &[("barrel", "Barrel"), ("fisheye", "True Fisheye")], "barrel").sec("Lens"),
        s("amount", "Distortion", -100.0, 400.0, 45.0).step(1.0).curve(1.3).sec("Lens").vis(when_not("projection", "fisheye")),
        s("fov", "Field of View", 60.0, 340.0, 180.0).step(1.0).unit("°").sec("Lens").vis(when("projection", "fisheye")),
        s("zoom", "Zoom", 20.0, 300.0, 100.0).step(1.0).unit("%").curve(1.3).sec("Lens"),
        ParamDef::toggle("autoZoom", "Auto Fill Frame", true).sec("Lens").vis(when_not("projection", "fisheye")),
        ParamDef::segmented("edge", "Outside", &[("mirror", "Mirror"), ("black", "Black")], "mirror").sec("Lens"),
        s("cx", "Center X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Lens"),
        s("cy", "Center Y", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Lens"),
        ParamDef::section_switch("circle", "Circular Frame", false),
        ParamDef::section_switch("fringeOn", "Color Fringing", true),
        s("fringe", "Amount", 0.0, 100.0, 10.0).step(1.0).unit("%").sec("Color Fringing"),
        ParamDef::section_switch("edgeBlurOn", "Edge Blur", false),
        s("edgeBlur", "Amount", 0.0, 100.0, 70.0).step(1.0).unit("%").sec("Edge Blur"),
        s("edgeBlurRadius", "Radius", 1.0, 80.0, 14.0).step(0.5).curve(1.6).sec("Edge Blur"),
        s("edgeBlurCenter", "Sharp Center Size", 0.0, 90.0, 40.0).step(1.0).unit("%").sec("Edge Blur"),
    ])
    .looks(vec![
        look("vx", "VX1000 Fisheye", vals!["amount" => 70.0, "fringe" => 15.0]),
        look("circle", "Full Circle", vals!["projection" => "fisheye", "fov" => 190.0, "circle" => true]),
        look("extreme", "Extreme 280°", vals!["projection" => "fisheye", "fov" => 280.0, "fringe" => 25.0]),
        look("ball", "Mirror Ball", vals!["projection" => "fisheye", "fov" => 340.0, "circle" => true, "fringe" => 30.0]),
        look("bubble", "Bubble", vals!["amount" => 260.0, "autoZoom" => false, "zoom" => 160.0]),
        look("mild", "Mild Barrel", vals!["amount" => 20.0]),
        look("pincushion", "Pincushion", vals!["amount" => -40.0, "zoom" => 95.0]),
    ])
}

pub fn edge_distort() -> Effect {
    Effect::new("edgewarp", "Edge Distort", Category::Distort, |input, p, ctx| {
        let mode = match p.choice_or("mode", "bend").as_str() { "stretch" => 1.0, "swirl" => 2.0, "ripple" => 3.0, "liquid" => 4.0, "squeeze" => 5.0, _ => 0.0 };
        let blur_on = p.bool_or("blurOn", false);
        let blurred = if blur_on { ctx.blur(input, ctx.px(p.num("blurRadius", 12.0))) } else { input.clone() };
        let u = [
            mode, p.fd("strength", 50.0) / 100.0, p.fd("protect", 35.0) / 100.0, p.fd("softness", 45.0) / 100.0,
            if p.choice_or("shape", "round") == "frame" { 1.0 } else { 0.0 }, p.fd("cx", 50.0) / 100.0, p.fd("cy", 50.0) / 100.0,
            if p.bool_or("fringeOn", true) { p.fd("fringe", 30.0) / 100.0 * 0.08 } else { 0.0 }, p.fd("seed", 1.0) * 3.1, ctx.scale as f32,
            p.fd("scale", 60.0), if blur_on { p.fd("blur", 50.0) / 100.0 } else { 0.0 },
            if p.bool_or("darkOn", false) { p.fd("darken", 35.0) / 100.0 } else { 0.0 },
        ];
        ctx.apply("k_edgewarp", &[input, &blurred], &u)
    })
    .symbol("circle.dashed.inset.filled")
    .summary("Keeps the centre intact and warps only the borders: bend, stretch, swirl, ripple, liquid or squeeze.")
    .params(vec![
        ParamDef::choice("mode", "Warp", &[("bend", "Bend (Fisheye Edges)"), ("stretch", "Stretch (Warp Speed)"), ("swirl", "Swirl"), ("ripple", "Ripple"), ("liquid", "Liquid"), ("squeeze", "Squeeze")], "bend").sec("Warp"),
        s("strength", "Strength", -100.0, 200.0, 50.0).step(1.0).unit("%").sec("Warp"),
        s("scale", "Pattern Size", 4.0, 400.0, 60.0).step(1.0).curve(1.6).sec("Warp").vis(when_any("mode", &["ripple", "liquid"])),
        ParamDef::seed("seed").sec("Warp"),
        ParamDef::segmented("shape", "Shape", &[("round", "Round"), ("frame", "Frame")], "round").sec("Protected Center"),
        s("protect", "Center Size", 0.0, 90.0, 35.0).step(1.0).unit("%").sec("Protected Center"),
        s("softness", "Transition", 2.0, 100.0, 45.0).step(1.0).unit("%").sec("Protected Center"),
        s("cx", "Center X", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Protected Center"),
        s("cy", "Center Y", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Protected Center"),
        ParamDef::section_switch("fringeOn", "Color Fringing", true),
        s("fringe", "Amount", 0.0, 100.0, 30.0).step(1.0).unit("%").sec("Color Fringing"),
        ParamDef::section_switch("blurOn", "Edge Blur", false),
        s("blur", "Amount", 0.0, 100.0, 50.0).step(1.0).unit("%").sec("Edge Blur"),
        s("blurRadius", "Radius", 1.0, 80.0, 12.0).step(0.5).curve(1.6).sec("Edge Blur"),
        ParamDef::section_switch("darkOn", "Edge Darkening", false),
        s("darken", "Amount", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Edge Darkening"),
    ])
    .looks(vec![
        look("fishedges", "Fisheye Edges", vals!["mode" => "bend", "strength" => 90.0, "protect" => 30.0]),
        look("warp", "Warp Speed", vals!["mode" => "stretch", "strength" => 120.0, "protect" => 40.0, "fringeOn" => true, "fringe" => 50.0]),
        look("vortex", "Edge Vortex", vals!["mode" => "swirl", "strength" => 80.0, "protect" => 35.0]),
        look("dream", "Dream Edges", vals!["mode" => "liquid", "strength" => 120.0, "scale" => 120.0, "blurOn" => true, "blur" => 40.0]),
        look("tunnel", "Tunnel Vision", vals!["mode" => "stretch", "strength" => 80.0, "protect" => 25.0, "blurOn" => true, "darkOn" => true, "darken" => 50.0]),
        look("ripple", "Rippled Frame", vals!["mode" => "ripple", "shape" => "frame", "strength" => 100.0, "scale" => 40.0, "protect" => 55.0]),
        look("squeeze", "Wide Squeeze", vals!["mode" => "squeeze", "strength" => 100.0, "protect" => 30.0]),
    ])
}

pub fn mirror() -> Effect {
    Effect::new("mirror", "Mirror & Tile", Category::Distort, |input, p, ctx| {
        let mode = match p.choice_or("mode", "lr").as_str() { "rl" => 1.0, "tb" => 2.0, "bt" => 3.0, "quad" => 4.0, "tilesm" => 5.0, "tiles" => 6.0, _ => 0.0 };
        ctx.apply("k_mirror", &[input], &[mode, p.fd("tiles", 3.0), p.fd("position", 50.0) / 100.0])
    })
    .symbol("square.split.2x1")
    .summary("Reflects half of the image onto the other half, four ways, or repeats it as tiles.")
    .params(vec![
        ParamDef::choice("mode", "Mode", &[("lr", "Left → Right"), ("rl", "Right → Left"), ("tb", "Top → Bottom"), ("bt", "Bottom → Top"), ("quad", "Four-way"), ("tilesm", "Mirrored Tiles"), ("tiles", "Tiles")], "lr"),
        s("position", "Mirror Line", 5.0, 95.0, 50.0).step(1.0).unit("%").vis(when_any("mode", &["lr", "rl", "tb", "bt"])),
        s("tiles", "Tiles", 1.0, 12.0, 3.0).step(1.0).vis(when_any("mode", &["tiles", "tilesm"])),
    ])
}

pub fn stretch() -> Effect {
    Effect::new("stretch", "Pixel Stretch", Category::Distort, |input, p, ctx| {
        let dir = match p.choice_or("dir", "right").as_str() { "left" => 1.0, "down" => 2.0, "up" => 3.0, _ => 0.0 };
        ctx.apply("k_stretch", &[input], &[dir, p.fd("position", 60.0) / 100.0, p.fd("jitter", 15.0) / 100.0, p.fd("seed", 1.0) * 2.9,
            ctx.px(p.num("lane", 3.0)) as f32])
    })
    .symbol("arrow.right.to.line")
    .summary("Smears one line of pixels across the rest of the image — a classic glitch-art move.")
    .params(vec![
        ParamDef::choice("dir", "Direction", &[("right", "To the Right"), ("left", "To the Left"), ("down", "Down"), ("up", "Up")], "right"),
        s("position", "Start Line", 0.0, 100.0, 60.0).step(0.5).unit("%"),
        s("jitter", "Jitter", 0.0, 100.0, 15.0).step(1.0).unit("%"),
        s("lane", "Jitter Lane Size", 0.5, 40.0, 3.0).step(0.5).curve(1.6),
        ParamDef::seed("seed"),
    ])
}
