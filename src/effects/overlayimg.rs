//! Image Overlay (0.9.1): a texture from the built-in library (dust, grunge, glass, paper,
//! plastic…) or one of the user's images, placed over the photo. The effect outputs the placed
//! image with its own transparency; the layer's blend mode (Screen for a new layer) and opacity
//! mix it in. See `crate::textures` for the library and the embedded user images.

use super::*;

pub fn image_overlay() -> Effect {
    let mut e = Effect::new("imageOverlay", "Image Overlay", Category::Overlay, |input, p, ctx| {
        let (w, h) = (input.width(), input.height());
        let tex = p.image("image").and_then(|r| crate::textures::texture(&r));
        let Some(tex) = tex else {
            // nothing to draw yet (library still downloading): a fully transparent layer
            return ctx.apply_sized("k_imgoverlay", &[], &[0.0], None, w, h);
        };
        let fit = match p.choice_or("fit", "fill").as_str() {
            "fit" => 1.0,
            "stretch" => 2.0,
            "tile" => 3.0,
            _ => 0.0,
        };
        let scale = p.num("scale", 100.0) / 100.0;
        let rot = p.n("rotation").to_radians();
        let (ia, ta) = (w as f64 / h.max(1) as f64, tex.width() as f64 / tex.height().max(1) as f64);
        // height of the placed image on the output, in pixels (mirrors the kernel)
        let bh = match fit as i32 {
            0 => {
                let (c, s) = (rot.cos().abs(), rot.sin().abs());
                let (nw, nh) = (ia * c + s, ia * s + c);
                if ta > nw / nh { nh } else { nw / ta }
            }
            1 => if ta > ia { ia / ta } else { 1.0 },
            2 => 1.0,
            _ => 0.5,
        };
        let shown = bh * scale * h as f64 * 1.25;
        // shrink a large image to about the size it is shown at (no shimmering grain), then blur
        let mut src = tex.clone();
        if shown < tex.height() as f64 * 0.8 {
            let k = (shown / tex.height() as f64).max(16.0 / tex.height() as f64);
            src = ctx.resize(&tex, ((tex.width() as f64 * k).round() as usize).max(1), ((tex.height() as f64 * k).round() as usize).max(1), false);
        }
        let blur = ctx.px(p.n("blur")) * src.height() as f64 / (bh * scale * h as f64).max(1.0);
        if blur > 0.3 {
            src = ctx.blur(&src, blur);
        }
        let tint = p.color_or("tint", hex("#FFB070"));
        let u = [
            1.0,
            fit as f32,
            scale as f32,
            p.f("posX") / 100.0,
            -p.f("posY") / 100.0,
            rot as f32,
            p.b("flipH"),
            p.b("flipV"),
            ta as f32,
            p.f("perspH") / 100.0,
            p.f("perspV") / 100.0,
            p.f("skewX") / 100.0,
            p.f("skewY") / 100.0,
            p.f("bulge") / 100.0,
            p.f("wave") / 100.0,
            p.fd("waveSize", 4.0),
            p.b("invert"),
            p.f("black") / 100.0,
            p.fd("white", 100.0) / 100.0,
            p.fd("midtones", 100.0) / 100.0,
            p.fd("saturation", 100.0) / 100.0,
            tint.r as f32,
            tint.g as f32,
            tint.b as f32,
            p.f("tintAmt") / 100.0,
            1.0,
        ];
        ctx.apply_sized("k_imgoverlay", &[&src], &u, None, w, h)
    })
    .symbol("photo.stack")
    .summary("Places a texture (dust, grunge, glass, paper, plastic…) or your own image over the photo. Screen by default; change the blend mode in the layers panel.")
    .blend(BlendMode::Screen)
    .params(vec![
        ParamDef::image("image", "Image", ImageRef::builtin(crate::textures::DEFAULT_TEXTURE.0, crate::textures::DEFAULT_TEXTURE.1)).sec("Image"),
        ParamDef::segmented("fit", "Fit", &[("fill", "Fill"), ("fit", "Fit"), ("stretch", "Stretch"), ("tile", "Tile")], "fill").sec("Placement"),
        s("scale", "Scale", 10.0, 400.0, 100.0).step(1.0).unit("%").curve(1.6).sec("Placement"),
        s("posX", "Position X", -100.0, 100.0, 0.0).step(0.5).unit("%").sec("Placement"),
        s("posY", "Position Y", -100.0, 100.0, 0.0).step(0.5).unit("%").sec("Placement"),
        s("rotation", "Rotation", -180.0, 180.0, 0.0).step(0.5).unit("°").sec("Placement"),
        ParamDef::toggle("flipH", "Flip Horizontal", false).sec("Placement"),
        ParamDef::toggle("flipV", "Flip Vertical", false).sec("Placement"),
        s("perspH", "Perspective Horizontal", -100.0, 100.0, 0.0).step(1.0).unit("%").sec("Distortion"),
        s("perspV", "Perspective Vertical", -100.0, 100.0, 0.0).step(1.0).unit("%").sec("Distortion"),
        s("skewX", "Skew Horizontal", -100.0, 100.0, 0.0).step(1.0).unit("%").sec("Distortion"),
        s("skewY", "Skew Vertical", -100.0, 100.0, 0.0).step(1.0).unit("%").sec("Distortion"),
        s("bulge", "Bulge / Pinch", -100.0, 100.0, 0.0).step(1.0).unit("%").sec("Distortion"),
        s("wave", "Wave", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Distortion"),
        s("waveSize", "Wave Count", 0.5, 20.0, 4.0).step(0.1).curve(1.5).sec("Distortion").vis(Arc::new(|p: &Params| p.n("wave") > 0.0)),
        ParamDef::toggle("invert", "Invert", false).sec("Tone"),
        s("black", "Black Point", 0.0, 95.0, 0.0).step(1.0).unit("%").sec("Tone"),
        s("white", "White Point", 5.0, 100.0, 100.0).step(1.0).unit("%").sec("Tone"),
        s("midtones", "Midtones", 25.0, 400.0, 100.0).step(1.0).unit("%").curve(1.5).sec("Tone"),
        s("saturation", "Saturation", 0.0, 200.0, 100.0).step(1.0).unit("%").sec("Tone"),
        ParamDef::color("tint", "Tint Color", hex("#FFB070")).sec("Tone"),
        s("tintAmt", "Tint", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Tone"),
        s("blur", "Blur", 0.0, 50.0, 0.0).step(0.5).curve(1.6).sec("Tone"),
    ])
    .depth_bias()
    .looks(vec![
        look_g("natural", "Natural", "Tone", vals![]),
        look_g("subtle", "Subtle", "Tone", vals!["black" => 20.0, "midtones" => 80.0]),
        look_g("strong", "Strong", "Tone", vals!["white" => 65.0, "midtones" => 120.0]),
        look_g("warm", "Warm Tint", "Tone", vals!["tintAmt" => 70.0, "tint" => hex("#FFB070")]),
        look_g("cool", "Cool Tint", "Tone", vals!["tintAmt" => 70.0, "tint" => hex("#8FB8FF")]),
        look_g("soft", "Soft Focus", "Tone", vals!["blur" => 4.0, "black" => 10.0]),
        look_g("inverted", "Inverted (for Multiply)", "Tone", vals!["invert" => true]),
    ]);
    e.produces_alpha = true;
    e
}
