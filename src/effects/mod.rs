//! Effect registry. Each effect is declarative: id, name, category, parameter definitions
//! (which drive the inspector), optional Looks and an `apply` function producing the output.

use std::sync::OnceLock;

pub use std::sync::Arc;

pub use crate::gpu::{Ctx, T};
pub use crate::model::*;
pub use crate::vals;

pub mod adjust;
pub mod ascii;
pub mod blob;
pub mod blur;
pub mod camcorder;
pub mod color;
pub mod copy;
pub mod digital;
pub mod distort;
pub mod dither;
pub mod extra;
pub mod film;
pub mod frame;
pub mod grain;
pub mod lens;
pub mod lensfilters;
pub mod light;
pub mod print;
pub mod texture;
pub mod vhs;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Adjust,
    Stylize,
    Film,
    Lens,
    Blur,
    Distort,
    Glitch,
    Overlay,
}

impl Category {
    pub const ALL: [Category; 8] = [
        Category::Adjust, Category::Stylize, Category::Film, Category::Lens, Category::Blur, Category::Distort, Category::Glitch,
        Category::Overlay,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            Category::Adjust => "Adjust & Color",
            Category::Stylize => "Print & Pixel",
            Category::Film => "Film & Light",
            Category::Lens => "Lens Filters",
            Category::Blur => "Blur",
            Category::Distort => "Distort",
            Category::Glitch => "Glitch & Retro",
            Category::Overlay => "Overlays & Tracking",
        }
    }
    pub fn symbol(&self) -> &'static str {
        match self {
            Category::Adjust => "slider.horizontal.3",
            Category::Stylize => "circle.grid.3x3",
            Category::Film => "film",
            Category::Lens => "camera.aperture",
            Category::Blur => "aqi.medium",
            Category::Distort => "water.waves",
            Category::Glitch => "tv",
            Category::Overlay => "viewfinder",
        }
    }
}

/// A named starting point for an effect's parameters (shown as "Look" in the inspector).
#[derive(Clone)]
pub struct Look {
    pub id: String,
    pub name: String,
    pub group: Option<String>,
    pub values: Vec<(String, ParamValue)>,
}

pub type ApplyFn = fn(&T, &Params, &mut Ctx) -> T;

pub struct Effect {
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub symbol: &'static str,
    pub summary: &'static str,
    pub params: Vec<ParamDef>,
    pub looks: Vec<Look>,
    /// Colour dot shown next to section titles (e.g. HSL bands).
    pub section_colors: Vec<(String, Rgba)>,
    pub produces_alpha: bool,
    /// Kept only so older projects / presets still render; not offered in the browser.
    pub hidden: bool,
    /// Does this effect (with these params) read the depth map? (`ctx.depth()`)
    pub uses_depth: fn(&Params) -> bool,
    pub apply: ApplyFn,
}

impl Effect {
    pub fn new(id: &'static str, name: &'static str, category: Category, apply: ApplyFn) -> Effect {
        Effect {
            id,
            name,
            category,
            symbol: category.symbol(),
            summary: "",
            params: vec![],
            looks: vec![],
            section_colors: vec![],
            produces_alpha: false,
            hidden: false,
            uses_depth: uses_depth_bias,
            apply,
        }
    }
    pub fn symbol(mut self, s: &'static str) -> Self { self.symbol = s; self }
    pub fn summary(mut self, s: &'static str) -> Self { self.summary = s; self }
    pub fn params(mut self, p: Vec<ParamDef>) -> Self { self.params = p; self }
    pub fn looks(mut self, l: Vec<Look>) -> Self { self.looks = l; self }
    pub fn section_colors(mut self, c: Vec<(String, Rgba)>) -> Self { self.section_colors = c; self }
    pub fn hidden(mut self) -> Self { self.hidden = true; self }
    pub fn depth(mut self, f: fn(&Params) -> bool) -> Self { self.uses_depth = f; self }
    /// Adds the standard "Depth" section (stronger in the background or the foreground).
    pub fn depth_bias(mut self) -> Self {
        self.params.extend(depth_bias_params());
        self
    }

    pub fn id(&self) -> &'static str { self.id }
    pub fn name(&self) -> &'static str { self.name }
    pub fn params_list(&self) -> &[ParamDef] { &self.params }
}

// `LayerNode::effect` calls `e.params()` — keep that spelling available.
pub trait EffectParams {
    fn params(&self) -> &[ParamDef];
}
impl EffectParams for Effect {
    fn params(&self) -> &[ParamDef] { &self.params }
}

// MARK: - literal → ParamValue (Swift `paramValue(Any)`)

impl From<f64> for ParamValue { fn from(v: f64) -> Self { ParamValue::Number(v) } }
impl From<i32> for ParamValue { fn from(v: i32) -> Self { ParamValue::Number(v as f64) } }
impl From<bool> for ParamValue { fn from(v: bool) -> Self { ParamValue::Bool(v) } }
impl From<&str> for ParamValue { fn from(v: &str) -> Self { ParamValue::Choice(v.into()) } }
impl From<String> for ParamValue { fn from(v: String) -> Self { ParamValue::Choice(v) } }
impl From<Rgba> for ParamValue { fn from(v: Rgba) -> Self { ParamValue::Color(v) } }
impl From<PaletteSpec> for ParamValue { fn from(v: PaletteSpec) -> Self { ParamValue::Palette(v) } }
impl From<GradientSpec> for ParamValue { fn from(v: GradientSpec) -> Self { ParamValue::Gradient(v) } }

/// `vals!["key" => 1.0, "on" => true, "mode" => "x"]`
#[macro_export]
macro_rules! vals {
    () => { Vec::<(String, $crate::model::ParamValue)>::new() };
    ($($k:expr => $v:expr),+ $(,)?) => {
        vec![$(($k.to_string(), $crate::model::ParamValue::from($v))),+]
    };
}

pub fn look(id: &str, name: &str, values: Vec<(String, ParamValue)>) -> Look {
    Look { id: id.into(), name: name.into(), group: None, values }
}
pub fn look_g(id: &str, name: &str, group: &str, values: Vec<(String, ParamValue)>) -> Look {
    Look { id: id.into(), name: name.into(), group: Some(group.into()), values }
}

pub fn hex(s: &str) -> Rgba { Rgba::hex(s) }

pub fn s(key: &str, label: &str, min: f64, max: f64, def: f64) -> ParamDef { ParamDef::slider(key, label, min, max, def) }

/// Standard per-effect depth weighting, applied by the renderer (see `renderer::depth_bias`).
pub fn depth_bias_params() -> Vec<ParamDef> {
    vec![
        ParamDef::segmented("depthBias", "Where", &[("off", "Everywhere"), ("far", "Background"), ("near", "Foreground")], "off").sec("Depth"),
        s("depthBiasAmount", "Amount", 0.0, 100.0, 80.0).unit("%").step(1.0).sec("Depth").vis(when_not("depthBias", "off")),
    ]
}

pub fn uses_depth_bias(p: &Params) -> bool { matches!(p.choice("depthBias").as_str(), "far" | "near") }

static REGISTRY: OnceLock<Vec<Effect>> = OnceLock::new();

pub fn all() -> &'static [Effect] {
    REGISTRY.get_or_init(|| {
        vec![
            // Adjust & color
            adjust::basic(), adjust::curves(), adjust::hsl(), adjust::huesat(), adjust::bw(), extra::split_tone(), color::gradient_map(), color::lut(),
            lens::sharpen(), adjust::invert(), adjust::posterize(), adjust::threshold(),
            // Print & pixel
            dither::dither(), print::halftone(), ascii::ascii(), extra::print_wear(), copy::photocopy(), copy::thermal(),
            // Film & light
            film::film(), grain::grain(), light::glow(), light::bloom(), light::halation(), texture::light_leak(),
            // Lens filters
            lensfilters::anamorphic(), lensfilters::star(), lensfilters::prism(), lensfilters::halo(),
            lens::chromatic(), lens::vignette(),
            // Blur
            blur::blur(), blur::radial(), blur::motion(), lensfilters::lens_blur(),
            // Distort
            distort::fisheye(), distort::edge_distort(), extra::reeded_glass(), distort::mirror(), distort::stretch(),
            // Glitch & retro
            camcorder::camcorder(), vhs::vhs(), lens::crt(), extra::led_display(), digital::digicam(), light::trail(),
            // Overlays & tracking
            texture::dust(), frame::frame(), blob::blob_tracker(),
        ]
    })
}

pub fn effect(id: &str) -> Option<&'static Effect> { all().iter().find(|e| e.id == id) }
