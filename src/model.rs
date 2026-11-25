//! Document model: parameter values, layer tree, crop and canvas. JSON layout matches the
//! macOS app's Swift `Codable` output, so projects, presets and sessions are interchangeable.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// MARK: - Basic value types

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    #[serde(default = "one")]
    pub a: f64,
}

fn one() -> f64 { 1.0 }
fn yes() -> bool { true }

impl Default for Rgba {
    fn default() -> Self { Rgba::BLACK }
}

impl Rgba {
    pub const BLACK: Rgba = Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Rgba = Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    pub const fn new(r: f64, g: f64, b: f64) -> Rgba { Rgba { r, g, b, a: 1.0 } }

    pub fn hex(s: &str) -> Rgba {
        let s = s.trim().trim_start_matches('#');
        if s.len() >= 6 {
            if let Ok(v) = u64::from_str_radix(&s[..s.len().min(8)], 16) {
                let v = if s.len() >= 8 { v >> 8 } else { v };
                return Rgba::new(((v >> 16) & 0xFF) as f64 / 255.0, ((v >> 8) & 0xFF) as f64 / 255.0, (v & 0xFF) as f64 / 255.0);
            }
        }
        Rgba::BLACK
    }

    pub fn to_hex(&self) -> String {
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02X}{:02X}{:02X}", c(self.r), c(self.g), c(self.b))
    }

    pub fn floats(&self) -> [f32; 3] { [self.r as f32, self.g as f32, self.b as f32] }
    pub fn luma(&self) -> f64 { 0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b }

    fn hash_into<H: Hasher>(&self, h: &mut H) {
        for v in [self.r, self.g, self.b, self.a] { v.to_bits().hash(h); }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveSet {
    #[serde(default = "CurveSet::identity")]
    pub master: Vec<CurvePoint>,
    #[serde(default = "CurveSet::identity")]
    pub red: Vec<CurvePoint>,
    #[serde(default = "CurveSet::identity")]
    pub green: Vec<CurvePoint>,
    #[serde(default = "CurveSet::identity")]
    pub blue: Vec<CurvePoint>,
}

impl Default for CurveSet {
    fn default() -> Self {
        CurveSet { master: Self::identity(), red: Self::identity(), green: Self::identity(), blue: Self::identity() }
    }
}

impl CurveSet {
    pub fn identity() -> Vec<CurvePoint> { vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 1.0, y: 1.0 }] }
    pub fn channel(&self, i: usize) -> &Vec<CurvePoint> {
        match i { 0 => &self.master, 1 => &self.red, 2 => &self.green, _ => &self.blue }
    }
    pub fn channel_mut(&mut self, i: usize) -> &mut Vec<CurvePoint> {
        match i { 0 => &mut self.master, 1 => &mut self.red, 2 => &mut self.green, _ => &mut self.blue }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub position: f64,
    pub color: Rgba,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientSpec {
    #[serde(default = "custom_name")]
    pub name: String,
    pub stops: Vec<GradientStop>,
}

fn custom_name() -> String { "Custom".into() }

impl GradientSpec {
    pub fn sample(&self, t: f64) -> Rgba {
        let mut s = self.stops.clone();
        s.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap_or(std::cmp::Ordering::Equal));
        let Some(first) = s.first() else { return Rgba::BLACK };
        if t <= first.position { return first.color; }
        for i in 0..s.len().saturating_sub(1) {
            let (a, b) = (&s[i], &s[i + 1]);
            if t >= a.position && t <= b.position {
                let f = if (b.position - a.position) > 1e-6 { (t - a.position) / (b.position - a.position) } else { 0.0 };
                return Rgba::new(
                    a.color.r + (b.color.r - a.color.r) * f,
                    a.color.g + (b.color.g - a.color.g) * f,
                    a.color.b + (b.color.b - a.color.b) * f,
                );
            }
        }
        s.last().unwrap().color
    }

    pub fn from(name: &str, hexes: &[&str]) -> GradientSpec {
        let n = (hexes.len().max(2) - 1) as f64;
        GradientSpec {
            name: name.into(),
            stops: hexes
                .iter()
                .enumerate()
                .map(|(i, h)| GradientStop { id: Uuid::new_v4(), position: i as f64 / n, color: Rgba::hex(h) })
                .collect(),
        }
    }

    fn hash_into<H: Hasher>(&self, h: &mut H) {
        self.name.hash(h);
        for s in &self.stops {
            s.position.to_bits().hash(h);
            s.color.hash_into(h);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaletteSpec {
    #[serde(default = "custom_name")]
    pub name: String,
    pub colors: Vec<Rgba>,
}

impl PaletteSpec {
    pub fn from(name: &str, hexes: &[&str]) -> PaletteSpec {
        PaletteSpec { name: name.into(), colors: hexes.iter().map(|h| Rgba::hex(h)).collect() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LutData {
    pub name: String,
    pub size: usize,
    /// r,g,b triplets, r fastest
    pub values: Arc<Vec<f32>>,
}

impl LutData {
    pub fn is_empty(&self) -> bool { self.size == 0 }
}

// MARK: - Parameter values

#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    Number(f64),
    Bool(bool),
    Choice(String),
    Color(Rgba),
    Curves(CurveSet),
    Gradient(GradientSpec),
    Palette(PaletteSpec),
    Text(String),
    Lut(LutData),
}

/// Swift encodes `case number(Double)` as `{"number":{"_0":1.5}}`.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ParamValueRepr {
    Number { #[serde(rename = "_0")] v: f64 },
    Bool { #[serde(rename = "_0")] v: bool },
    Choice { #[serde(rename = "_0")] v: String },
    Color { #[serde(rename = "_0")] v: Rgba },
    Curves { #[serde(rename = "_0")] v: CurveSet },
    Gradient { #[serde(rename = "_0")] v: GradientSpec },
    Palette { #[serde(rename = "_0")] v: PaletteSpec },
    Text { #[serde(rename = "_0")] v: String },
    Lut { #[serde(rename = "_0")] v: LutData },
}

impl Serialize for ParamValue {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let r = match self.clone() {
            ParamValue::Number(v) => ParamValueRepr::Number { v },
            ParamValue::Bool(v) => ParamValueRepr::Bool { v },
            ParamValue::Choice(v) => ParamValueRepr::Choice { v },
            ParamValue::Color(v) => ParamValueRepr::Color { v },
            ParamValue::Curves(v) => ParamValueRepr::Curves { v },
            ParamValue::Gradient(v) => ParamValueRepr::Gradient { v },
            ParamValue::Palette(v) => ParamValueRepr::Palette { v },
            ParamValue::Text(v) => ParamValueRepr::Text { v },
            ParamValue::Lut(v) => ParamValueRepr::Lut { v },
        };
        r.serialize(s)
    }
}

impl<'de> Deserialize<'de> for ParamValue {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match ParamValueRepr::deserialize(d)? {
            ParamValueRepr::Number { v } => ParamValue::Number(v),
            ParamValueRepr::Bool { v } => ParamValue::Bool(v),
            ParamValueRepr::Choice { v } => ParamValue::Choice(v),
            ParamValueRepr::Color { v } => ParamValue::Color(v),
            ParamValueRepr::Curves { v } => ParamValue::Curves(v),
            ParamValueRepr::Gradient { v } => ParamValue::Gradient(v),
            ParamValueRepr::Palette { v } => ParamValue::Palette(v),
            ParamValueRepr::Text { v } => ParamValue::Text(v),
            ParamValueRepr::Lut { v } => ParamValue::Lut(v),
        })
    }
}

impl ParamValue {
    fn hash_into<H: Hasher>(&self, h: &mut H) {
        std::mem::discriminant(self).hash(h);
        match self {
            ParamValue::Number(v) => v.to_bits().hash(h),
            ParamValue::Bool(v) => v.hash(h),
            ParamValue::Choice(v) | ParamValue::Text(v) => v.hash(h),
            ParamValue::Color(c) => c.hash_into(h),
            ParamValue::Curves(c) => {
                for ch in 0..4 {
                    for p in c.channel(ch) {
                        p.x.to_bits().hash(h);
                        p.y.to_bits().hash(h);
                    }
                    0xFFu8.hash(h);
                }
            }
            ParamValue::Gradient(g) => g.hash_into(h),
            ParamValue::Palette(p) => {
                p.name.hash(h);
                for c in &p.colors { c.hash_into(h); }
            }
            ParamValue::Lut(l) => {
                l.name.hash(h);
                l.size.hash(h);
                (Arc::as_ptr(&l.values) as usize).hash(h);
                l.values.len().hash(h);
            }
        }
    }
}

/// Bag of parameter values with typed getters that fall back to defaults.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Params {
    #[serde(default)]
    pub values: BTreeMap<String, ParamValue>,
}

impl Params {
    pub fn get(&self, k: &str) -> Option<&ParamValue> { self.values.get(k) }
    pub fn set(&mut self, k: &str, v: ParamValue) { self.values.insert(k.to_string(), v); }

    pub fn num(&self, k: &str, d: f64) -> f64 {
        if let Some(ParamValue::Number(v)) = self.values.get(k) { *v } else { d }
    }
    /// `num(k, 0)` as f32 (Swift `p.f(k)`).
    pub fn f(&self, k: &str) -> f32 { self.num(k, 0.0) as f32 }
    pub fn fd(&self, k: &str, d: f64) -> f32 { self.num(k, d) as f32 }
    pub fn n(&self, k: &str) -> f64 { self.num(k, 0.0) }
    pub fn bool(&self, k: &str) -> bool { self.bool_or(k, false) }
    pub fn bool_or(&self, k: &str, d: bool) -> bool {
        if let Some(ParamValue::Bool(v)) = self.values.get(k) { *v } else { d }
    }
    /// 1.0 / 0.0 for shader uniforms.
    pub fn b(&self, k: &str) -> f32 { if self.bool(k) { 1.0 } else { 0.0 } }
    pub fn choice(&self, k: &str) -> String { self.choice_or(k, "") }
    pub fn choice_or(&self, k: &str, d: &str) -> String {
        if let Some(ParamValue::Choice(v)) = self.values.get(k) { v.clone() } else { d.to_string() }
    }
    pub fn is(&self, k: &str, v: &str) -> bool { matches!(self.values.get(k), Some(ParamValue::Choice(c)) if c == v) }
    pub fn color(&self, k: &str) -> Rgba { self.color_or(k, Rgba::BLACK) }
    pub fn color_or(&self, k: &str, d: Rgba) -> Rgba {
        if let Some(ParamValue::Color(v)) = self.values.get(k) { *v } else { d }
    }
    pub fn curves(&self, k: &str) -> CurveSet {
        if let Some(ParamValue::Curves(v)) = self.values.get(k) { v.clone() } else { CurveSet::default() }
    }
    pub fn gradient(&self, k: &str) -> Option<GradientSpec> {
        if let Some(ParamValue::Gradient(v)) = self.values.get(k) { Some(v.clone()) } else { None }
    }
    pub fn palette(&self, k: &str) -> Option<PaletteSpec> {
        if let Some(ParamValue::Palette(v)) = self.values.get(k) { Some(v.clone()) } else { None }
    }
    pub fn text(&self, k: &str) -> String { self.text_or(k, "") }
    pub fn text_or(&self, k: &str, d: &str) -> String {
        if let Some(ParamValue::Text(v)) = self.values.get(k) { v.clone() } else { d.to_string() }
    }
    pub fn lut(&self, k: &str) -> Option<LutData> {
        if let Some(ParamValue::Lut(v)) = self.values.get(k) { Some(v.clone()) } else { None }
    }

    pub fn hash_into<H: Hasher>(&self, h: &mut H) {
        self.values.len().hash(h);
        for (k, v) in &self.values {
            k.hash(h);
            v.hash_into(h);
        }
    }
}

// MARK: - Parameter definitions (drive the inspector UI)

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
    /// Optional section title used to group long lists in pickers.
    pub group: Option<String>,
}

pub fn opt(id: &str, label: &str) -> ChoiceOption { ChoiceOption { id: id.into(), label: label.into(), group: None } }
pub fn opts(pairs: &[(&str, &str)]) -> Vec<ChoiceOption> { pairs.iter().map(|(a, b)| opt(a, b)).collect() }

#[derive(Clone)]
pub enum ParamKind {
    Slider { min: f64, max: f64, def: f64, step: f64, unit: String, curve: f64 },
    Toggle(bool),
    Choice(Vec<ChoiceOption>, String),
    Segmented(Vec<ChoiceOption>, String),
    Color(Rgba),
    Curves,
    Gradient(GradientSpec),
    Palette(PaletteSpec),
    Text(String),
    Lut,
    Seed,
}

pub type Visible = Arc<dyn Fn(&Params) -> bool + Send + Sync>;

#[derive(Clone)]
pub struct ParamDef {
    pub key: String,
    pub label: String,
    pub kind: ParamKind,
    pub section: Option<String>,
    pub visible: Option<Visible>,
    /// Optional colours drawn on the slider track (e.g. hue ramps).
    pub track: Option<Vec<Rgba>>,
    /// A boolean shown as an on/off switch in its section header.
    pub is_section_switch: bool,
}

impl ParamDef {
    pub fn default_value(&self) -> Option<ParamValue> {
        Some(match &self.kind {
            ParamKind::Slider { def, .. } => ParamValue::Number(*def),
            ParamKind::Toggle(d) => ParamValue::Bool(*d),
            ParamKind::Choice(_, d) | ParamKind::Segmented(_, d) => ParamValue::Choice(d.clone()),
            ParamKind::Color(d) => ParamValue::Color(*d),
            ParamKind::Curves => ParamValue::Curves(CurveSet::default()),
            ParamKind::Gradient(d) => ParamValue::Gradient(d.clone()),
            ParamKind::Palette(d) => ParamValue::Palette(d.clone()),
            ParamKind::Text(d) => ParamValue::Text(d.clone()),
            ParamKind::Lut => return None,
            ParamKind::Seed => ParamValue::Number(1.0),
        })
    }

    fn base(key: &str, label: &str, kind: ParamKind) -> ParamDef {
        ParamDef { key: key.into(), label: label.into(), kind, section: None, visible: None, track: None, is_section_switch: false }
    }

    /// Slider. Chain `.step()`, `.unit()`, `.curve()`, `.sec()`, `.track()`, `.vis()`.
    pub fn slider(key: &str, label: &str, min: f64, max: f64, def: f64) -> ParamDef {
        Self::base(key, label, ParamKind::Slider { min, max, def, step: 0.0, unit: String::new(), curve: 1.0 })
    }
    pub fn toggle(key: &str, label: &str, def: bool) -> ParamDef { Self::base(key, label, ParamKind::Toggle(def)) }
    pub fn choice(key: &str, label: &str, options: &[(&str, &str)], def: &str) -> ParamDef {
        Self::base(key, label, ParamKind::Choice(opts(options), def.into()))
    }
    pub fn choice_opts(key: &str, label: &str, options: Vec<ChoiceOption>, def: &str) -> ParamDef {
        Self::base(key, label, ParamKind::Choice(options, def.into()))
    }
    pub fn segmented(key: &str, label: &str, options: &[(&str, &str)], def: &str) -> ParamDef {
        Self::base(key, label, ParamKind::Segmented(opts(options), def.into()))
    }
    pub fn color(key: &str, label: &str, def: Rgba) -> ParamDef { Self::base(key, label, ParamKind::Color(def)) }
    pub fn curves(key: &str, label: &str) -> ParamDef { Self::base(key, label, ParamKind::Curves) }
    pub fn gradient(key: &str, label: &str, def: GradientSpec) -> ParamDef { Self::base(key, label, ParamKind::Gradient(def)) }
    pub fn palette(key: &str, label: &str, def: PaletteSpec) -> ParamDef { Self::base(key, label, ParamKind::Palette(def)) }
    pub fn text(key: &str, label: &str, def: &str) -> ParamDef { Self::base(key, label, ParamKind::Text(def.into())) }
    pub fn lut(key: &str, label: &str) -> ParamDef { Self::base(key, label, ParamKind::Lut) }
    /// On/off switch for a whole section, drawn in the section header.
    pub fn section_switch(key: &str, section: &str, def: bool) -> ParamDef {
        let mut d = Self::base(key, section, ParamKind::Toggle(def));
        d.section = Some(section.into());
        d.is_section_switch = true;
        d
    }
    pub fn seed(key: &str) -> ParamDef { Self::base(key, "Seed", ParamKind::Seed) }

    pub fn step(mut self, s: f64) -> Self {
        if let ParamKind::Slider { step, .. } = &mut self.kind { *step = s; }
        self
    }
    pub fn unit(mut self, u: &str) -> Self {
        if let ParamKind::Slider { unit, .. } = &mut self.kind { *unit = u.into(); }
        self
    }
    pub fn curve(mut self, c: f64) -> Self {
        if let ParamKind::Slider { curve, .. } = &mut self.kind { *curve = c; }
        self
    }
    pub fn sec(mut self, s: &str) -> Self { self.section = Some(s.into()); self }
    pub fn sec_opt(mut self, s: Option<&str>) -> Self { self.section = s.map(|s| s.into()); self }
    pub fn track(mut self, t: Vec<Rgba>) -> Self { self.track = Some(t); self }
    pub fn vis(mut self, v: Visible) -> Self { self.visible = Some(v); self }
    pub fn is_visible(&self, p: &Params) -> bool { self.visible.as_ref().map(|f| f(p)).unwrap_or(true) }
}

/// Visibility helpers for param defs.
pub fn when(key: &str, value: &str) -> Visible {
    let (k, v) = (key.to_string(), value.to_string());
    Arc::new(move |p| p.choice(&k) == v)
}
pub fn when_not(key: &str, value: &str) -> Visible {
    let (k, v) = (key.to_string(), value.to_string());
    Arc::new(move |p| p.choice(&k) != v)
}
pub fn when_on(key: &str) -> Visible {
    let k = key.to_string();
    Arc::new(move |p| p.bool(&k))
}
pub fn when_any(key: &str, values: &[&str]) -> Visible {
    let k = key.to_string();
    let vs: Vec<String> = values.iter().map(|s| s.to_string()).collect();
    Arc::new(move |p| vs.contains(&p.choice(&k)))
}
pub fn when_all(fs: Vec<Visible>) -> Visible { Arc::new(move |p| fs.iter().all(|f| f(p))) }

// MARK: - Blend modes

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    HardLight,
    Darken,
    Lighten,
    Difference,
    Color,
    Luminosity,
    // appended later (shader indices follow declaration order)
    ColorDodge,
    ColorBurn,
    Add,
    LinearBurn,
    Exclusion,
    Hue,
    Saturation,
}

impl BlendMode {
    pub const ALL: [BlendMode; 18] = [
        BlendMode::Normal, BlendMode::Multiply, BlendMode::Screen, BlendMode::Overlay, BlendMode::SoftLight, BlendMode::HardLight,
        BlendMode::Darken, BlendMode::Lighten, BlendMode::Difference, BlendMode::Color, BlendMode::Luminosity, BlendMode::ColorDodge,
        BlendMode::ColorBurn, BlendMode::Add, BlendMode::LinearBurn, BlendMode::Exclusion, BlendMode::Hue, BlendMode::Saturation,
    ];
    pub const MENU_ORDER: [BlendMode; 18] = [
        BlendMode::Normal, BlendMode::Multiply, BlendMode::Darken, BlendMode::ColorBurn, BlendMode::LinearBurn, BlendMode::Screen,
        BlendMode::Lighten, BlendMode::ColorDodge, BlendMode::Add, BlendMode::Overlay, BlendMode::SoftLight, BlendMode::HardLight,
        BlendMode::Difference, BlendMode::Exclusion, BlendMode::Hue, BlendMode::Saturation, BlendMode::Color, BlendMode::Luminosity,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::SoftLight => "Soft Light",
            BlendMode::HardLight => "Hard Light",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
            BlendMode::Difference => "Difference",
            BlendMode::Color => "Color",
            BlendMode::Luminosity => "Luminosity",
            BlendMode::ColorDodge => "Color Dodge",
            BlendMode::ColorBurn => "Color Burn",
            BlendMode::Add => "Add (Linear Dodge)",
            BlendMode::LinearBurn => "Linear Burn",
            BlendMode::Exclusion => "Exclusion",
            BlendMode::Hue => "Hue",
            BlendMode::Saturation => "Saturation",
        }
    }
    pub fn group(&self) -> &'static str {
        use BlendMode::*;
        match self {
            Normal => "Normal",
            Multiply | Darken | ColorBurn | LinearBurn => "Darken",
            Screen | Lighten | ColorDodge | Add => "Lighten",
            Overlay | SoftLight | HardLight => "Contrast",
            Difference | Exclusion => "Inversion",
            Color | Luminosity | Hue | BlendMode::Saturation => "Component",
        }
    }
    pub fn id(&self) -> String { serde_json::to_value(self).unwrap().as_str().unwrap().to_string() }
    pub fn from_id(s: &str) -> Option<BlendMode> { serde_json::from_value(serde_json::Value::String(s.into())).ok() }
    pub fn shader_index(&self) -> f32 { BlendMode::ALL.iter().position(|b| b == self).unwrap() as f32 }
}

// MARK: - Layer tree

/// A node in the layer tree. `children` is ordered top-first (index 0 = applied last).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerNode {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub is_group: bool,
    #[serde(default, rename = "effectID")]
    pub effect_id: String,
    #[serde(default)]
    pub params: Params,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "one")]
    pub opacity: f64,
    #[serde(default)]
    pub blend: BlendMode,
    #[serde(default = "yes")]
    pub expanded: bool,
    #[serde(default)]
    pub children: Vec<LayerNode>,
}

impl LayerNode {
    pub fn effect(id: &str, name: Option<String>, params: Option<Params>) -> LayerNode {
        let eff = crate::effects::effect(id);
        let mut p = params.unwrap_or_default();
        if let Some(e) = eff {
            for def in &e.params {
                if p.get(&def.key).is_none() {
                    if let Some(d) = def.default_value() { p.set(&def.key, d); }
                }
            }
        }
        let name = name.unwrap_or_else(|| eff.map(|e| crate::l10n::tr(e.name())).unwrap_or_else(|| id.to_string()));
        LayerNode {
            id: Uuid::new_v4(),
            name,
            is_group: false,
            effect_id: id.into(),
            params: p,
            enabled: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            expanded: true,
            children: vec![],
        }
    }

    pub fn group(name: &str, children: Vec<LayerNode>) -> LayerNode {
        LayerNode {
            id: Uuid::new_v4(),
            name: name.into(),
            is_group: true,
            effect_id: String::new(),
            params: Params::default(),
            enabled: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            expanded: true,
            children,
        }
    }

    /// Hash of the rendering-relevant state (ignores name / expansion).
    pub fn render_hash<H: Hasher>(&self, h: &mut H) {
        self.is_group.hash(h);
        self.effect_id.hash(h);
        self.params.hash_into(h);
        self.enabled.hash(h);
        self.opacity.to_bits().hash(h);
        self.blend.hash(h);
        for c in &self.children { c.render_hash(h); }
    }

    /// Fresh copy with new ids everywhere (used for duplicate / presets).
    pub fn reidentified(&self) -> LayerNode {
        let mut n = self.clone();
        n.id = Uuid::new_v4();
        n.children = self.children.iter().map(|c| c.reidentified()).collect();
        n
    }
}

/// Read-only tree queries (work on slices and vectors).
pub trait LayerTreeRead {
    fn find(&self, id: Uuid) -> Option<&LayerNode>;
    fn location(&self, id: Uuid) -> Option<(Option<Uuid>, usize)>;
    fn contains_inside(&self, id: Uuid, ancestor: Uuid) -> bool;
    fn flattened(&self) -> Vec<(LayerNode, usize)>;
    fn all_effect_ids(&self) -> Vec<String>;
}

/// Tree edits.
pub trait LayerTree: LayerTreeRead {
    fn update(&mut self, id: Uuid, body: &mut dyn FnMut(&mut LayerNode)) -> bool;
    fn remove_node(&mut self, id: Uuid) -> Option<LayerNode>;
    fn insert_node(&mut self, node: LayerNode, parent: Option<Uuid>, index: usize);
}

fn location_in(nodes: &[LayerNode], id: Uuid, parent: Option<Uuid>) -> Option<(Option<Uuid>, usize)> {
    if let Some(i) = nodes.iter().position(|n| n.id == id) { return Some((parent, i)); }
    for n in nodes {
        if let Some(l) = location_in(&n.children, id, Some(n.id)) { return Some(l); }
    }
    None
}

fn flatten_into(nodes: &[LayerNode], depth: usize, out: &mut Vec<(LayerNode, usize)>) {
    for n in nodes {
        out.push((n.clone(), depth));
        if n.is_group && n.expanded { flatten_into(&n.children, depth + 1, out); }
    }
}

impl LayerTreeRead for [LayerNode] {
    fn find(&self, id: Uuid) -> Option<&LayerNode> {
        for n in self {
            if n.id == id { return Some(n); }
            if let Some(f) = n.children.find(id) { return Some(f); }
        }
        None
    }
    fn location(&self, id: Uuid) -> Option<(Option<Uuid>, usize)> { location_in(self, id, None) }
    fn contains_inside(&self, id: Uuid, ancestor: Uuid) -> bool {
        self.find(ancestor).map(|a| a.children.find(id).is_some()).unwrap_or(false)
    }
    fn flattened(&self) -> Vec<(LayerNode, usize)> {
        let mut v = vec![];
        flatten_into(self, 0, &mut v);
        v
    }
    fn all_effect_ids(&self) -> Vec<String> {
        self.iter().flat_map(|n| if n.is_group { n.children.all_effect_ids() } else { vec![n.effect_id.clone()] }).collect()
    }
}

impl LayerTreeRead for Vec<LayerNode> {
    fn find(&self, id: Uuid) -> Option<&LayerNode> { self.as_slice().find(id) }
    fn location(&self, id: Uuid) -> Option<(Option<Uuid>, usize)> { self.as_slice().location(id) }
    fn contains_inside(&self, id: Uuid, ancestor: Uuid) -> bool { self.as_slice().contains_inside(id, ancestor) }
    fn flattened(&self) -> Vec<(LayerNode, usize)> { self.as_slice().flattened() }
    fn all_effect_ids(&self) -> Vec<String> { self.as_slice().all_effect_ids() }
}

impl LayerTree for Vec<LayerNode> {
    fn update(&mut self, id: Uuid, body: &mut dyn FnMut(&mut LayerNode)) -> bool {
        for n in self.iter_mut() {
            if n.id == id { body(n); return true; }
            if n.children.update(id, body) { return true; }
        }
        false
    }
    fn remove_node(&mut self, id: Uuid) -> Option<LayerNode> {
        if let Some(i) = self.iter().position(|n| n.id == id) { return Some(self.remove(i)); }
        for n in self.iter_mut() {
            if let Some(r) = n.children.remove_node(id) { return Some(r); }
        }
        None
    }
    fn insert_node(&mut self, node: LayerNode, parent: Option<Uuid>, index: usize) {
        match parent {
            Some(p) => {
                let mut node = Some(node);
                self.update(p, &mut |pn| {
                    let i = index.min(pn.children.len());
                    if let Some(n) = node.take() { pn.children.insert(i, n); }
                });
            }
            None => {
                let i = index.min(self.len());
                self.insert(i, node);
            }
        }
    }
}

// MARK: - Document

/// Normalized crop rect (0..1 in source image space, origin top-left).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CropRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Default for CropRect {
    fn default() -> Self { CropRect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 } }
}

impl CropRect {
    pub fn is_full(&self) -> bool { self.x <= 0.0001 && self.y <= 0.0001 && self.w >= 0.9999 && self.h >= 0.9999 }
    /// (x, y, w, h) in pixels.
    pub fn pixel_rect(&self, width: usize, height: usize) -> (f64, f64, f64, f64) {
        (
            (self.x * width as f64).round(),
            (self.y * height as f64).round(),
            (self.w * width as f64).round().max(1.0),
            (self.h * height as f64).round().max(1.0),
        )
    }
    fn hash_into<H: Hasher>(&self, h: &mut H) {
        for v in [self.x, self.y, self.w, self.h] { v.to_bits().hash(h); }
    }
}

pub struct CanvasRatio {
    pub id: &'static str,
    pub label: &'static str,
    pub w: f64,
    pub h: f64,
    pub group: &'static str,
}

/// Fits the (processed) image into a canvas of another aspect ratio, adding borders.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CanvasSpec {
    pub enabled: bool,
    pub ratio: String,
    pub custom_w: f64,
    pub custom_h: f64,
    pub fill: String,
    pub color: Rgba,
    pub color2: Rgba,
    pub padding: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub radius: f64,
    pub shadow: f64,
    pub blur: f64,
}

impl Default for CanvasSpec {
    fn default() -> Self {
        CanvasSpec {
            enabled: true,
            ratio: "4:5".into(),
            custom_w: 4.0,
            custom_h: 5.0,
            fill: "color".into(),
            color: Rgba::hex("#FFFFFF"),
            color2: Rgba::hex("#E9E6DF"),
            padding: 4.0,
            offset_x: 0.0,
            offset_y: 0.0,
            radius: 0.0,
            shadow: 0.0,
            blur: 60.0,
        }
    }
}

pub const CANVAS_RATIOS: &[CanvasRatio] = &[
    CanvasRatio { id: "4:5", label: "4:5 · Instagram Portrait", w: 4.0, h: 5.0, group: "Social" },
    CanvasRatio { id: "1:1", label: "1:1 · Square", w: 1.0, h: 1.0, group: "Social" },
    CanvasRatio { id: "9:16", label: "9:16 · Story / Reels", w: 9.0, h: 16.0, group: "Social" },
    CanvasRatio { id: "3:4", label: "3:4 · Instagram (new)", w: 3.0, h: 4.0, group: "Social" },
    CanvasRatio { id: "1.91:1", label: "1.91:1 · Landscape Post", w: 1.91, h: 1.0, group: "Social" },
    CanvasRatio { id: "16:9", label: "16:9 · Widescreen", w: 16.0, h: 9.0, group: "Screen" },
    CanvasRatio { id: "4:3", label: "4:3", w: 4.0, h: 3.0, group: "Screen" },
    CanvasRatio { id: "21:9", label: "21:9 · Ultrawide", w: 21.0, h: 9.0, group: "Screen" },
    CanvasRatio { id: "2:3", label: "2:3 · Print Portrait", w: 2.0, h: 3.0, group: "Print" },
    CanvasRatio { id: "3:2", label: "3:2 · Print Landscape", w: 3.0, h: 2.0, group: "Print" },
    CanvasRatio { id: "5:7", label: "5:7 · Print", w: 5.0, h: 7.0, group: "Print" },
    CanvasRatio { id: "8:10", label: "8:10 · Print", w: 8.0, h: 10.0, group: "Print" },
    CanvasRatio { id: "A4", label: "A4 · Paper", w: 210.0, h: 297.0, group: "Print" },
    CanvasRatio { id: "custom", label: "Custom…", w: 0.0, h: 0.0, group: "Custom" },
];

impl CanvasSpec {
    pub fn aspect(&self) -> f64 {
        if self.ratio == "custom" { return self.custom_w.max(0.05) / self.custom_h.max(0.05); }
        match CANVAS_RATIOS.iter().find(|r| r.id == self.ratio) {
            Some(r) if r.h > 0.0 => r.w / r.h,
            _ => 0.8,
        }
    }

    /// Canvas size and the image rect (x, y, w, h) inside it, for an image of w×h pixels.
    pub fn layout(&self, w: f64, h: f64) -> ((f64, f64), (f64, f64, f64, f64)) {
        let a = self.aspect();
        let m = self.padding / 100.0 * w.max(h);
        let mut ww = (w + 2.0 * m).max((h + 2.0 * m) * a);
        let mut hh = ww / a;
        if hh < h + 2.0 * m {
            hh = h + 2.0 * m;
            ww = hh * a;
        }
        ww = ww.round();
        hh = hh.round();
        let (fx, fy) = ((ww - w) / 2.0, (hh - h) / 2.0);
        let (x, y) = (fx + fx * self.offset_x / 100.0, fy + fy * self.offset_y / 100.0);
        ((ww, hh), (x.round(), y.round(), w, h))
    }

    pub fn hash_into<H: Hasher>(&self, h: &mut H) {
        self.enabled.hash(h);
        self.ratio.hash(h);
        self.fill.hash(h);
        for v in [self.custom_w, self.custom_h, self.padding, self.offset_x, self.offset_y, self.radius, self.shadow, self.blur] {
            v.to_bits().hash(h);
        }
        self.color.hash_into(h);
        self.color2.hash_into(h);
    }
}

/// The editable state of a document. Value type so undo is a snapshot.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DocumentState {
    #[serde(default)]
    pub layers: Vec<LayerNode>,
    #[serde(default)]
    pub crop: CropRect,
    /// Aspect-ratio canvas with borders (None = off).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canvas: Option<CanvasSpec>,
}

impl DocumentState {
    pub fn active_canvas(&self) -> Option<&CanvasSpec> { self.canvas.as_ref().filter(|c| c.enabled) }
    pub fn hash_value(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.layers.len().hash(&mut h);
        for l in &self.layers { l.render_hash(&mut h); }
        self.crop.hash_into(&mut h);
        if let Some(c) = &self.canvas { c.hash_into(&mut h); }
        h.finish()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    #[serde(default = "one_i")]
    pub version: i64,
    #[serde(default = "app_name")]
    pub app: String,
    pub original_file_name: String,
    pub state: DocumentState,
}

fn one_i() -> i64 { 1 }
fn app_name() -> String { "EFFECTOR".into() }
