//! Presets: saved layer stacks (user + factory), import/export as `.effectorpreset`.

use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::effects;
use crate::model::*;
use crate::swatches;
use crate::vals;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub name: String,
    pub layers: Vec<LayerNode>,
    #[serde(default)]
    pub is_factory: bool,
    /// JPEG thumbnail, base64 (Swift `Data`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    /// Section in the presets panel (factory presets only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

impl Preset {
    /// Factory preset names are translated; user names are shown as typed.
    pub fn display_name(&self) -> String { if self.is_factory { crate::l10n::tr(&self.name) } else { self.name.clone() } }
}

#[derive(Serialize, Deserialize)]
struct PresetFile {
    #[serde(default = "one")]
    version: i64,
    #[serde(default = "app")]
    app: String,
    presets: Vec<Preset>,
}
fn one() -> i64 { 1 }
fn app() -> String { "EFFECTOR".into() }

pub const CATEGORIES: [&str; 9] = [
    "Cinema", "Film", "Portrait & Light", "Y2K & Digital", "Retro Video", "Print & Graphic", "Pixel & Code", "Dream & Surreal", "Tracking & HUD",
];

// MARK: - Factory DSL (mirrors FactoryPresets in the macOS app)

fn make(name: &str, category: &str, layers_bottom_first: Vec<LayerNode>) -> Preset {
    let mut layers = layers_bottom_first;
    layers.reverse();
    Preset { id: Uuid::new_v4(), name: name.into(), layers, is_factory: true, thumbnail: None, category: Some(category.into()) }
}

/// Builds an effect layer with overrides.
pub fn fx(id: &str, name: Option<&str>, opacity: f64, blend: BlendMode, values: Vec<(String, ParamValue)>) -> LayerNode {
    let mut p = Params::default();
    for (k, v) in values {
        p.set(&k, v);
    }
    let mut n = LayerNode::effect(id, None, Some(p));
    if let Some(name) = name {
        n.name = name.into();
    }
    n.opacity = opacity;
    n.blend = blend;
    n
}

/// An effect layer starting from one of the effect's Looks, with optional overrides.
fn look(id: &str, look_id: &str, name: Option<&str>, opacity: f64, blend: BlendMode, extra: Vec<(String, ParamValue)>) -> LayerNode {
    let mut vals: Vec<(String, ParamValue)> =
        effects::effect(id).and_then(|e| e.looks.iter().find(|l| l.id == look_id)).map(|l| l.values.clone()).unwrap_or_default();
    vals.extend(extra);
    vals.push(("_look".into(), ParamValue::Text(look_id.into())));
    fx(id, name, opacity, blend, vals)
}

/// A folder of layers inside a preset (children bottom-first, like `make`).
fn grp(name: &str, opacity: f64, blend: BlendMode, bottom_first: Vec<LayerNode>) -> LayerNode {
    let mut c = bottom_first;
    c.reverse();
    let mut g = LayerNode::group(name, c);
    g.opacity = opacity;
    g.blend = blend;
    g.expanded = false;
    g
}

/// Curves value from (x, y) points in 0…1 per channel (empty = identity).
fn curves(m: &[(f64, f64)], r: &[(f64, f64)], g: &[(f64, f64)], b: &[(f64, f64)]) -> ParamValue {
    let pts = |l: &[(f64, f64)]| -> Vec<CurvePoint> {
        if l.is_empty() {
            return CurveSet::identity();
        }
        let mut v = vec![CurvePoint { x: 0.0, y: 0.0 }];
        v.extend(l.iter().map(|p| CurvePoint { x: p.0, y: p.1 }));
        v.push(CurvePoint { x: 1.0, y: 1.0 });
        v
    };
    ParamValue::Curves(CurveSet { master: pts(m), red: pts(r), green: pts(g), blue: pts(b) })
}

#[allow(dead_code)]
fn s_curve() -> ParamValue { curves(&[(0.25, 0.20), (0.75, 0.80)], &[], &[], &[]) }
#[allow(dead_code)]
fn soft_s() -> ParamValue { curves(&[(0.25, 0.22), (0.75, 0.78)], &[], &[], &[]) }

/// Film layer with grain kept fine and subtle.
fn film(stock: &str, grain: f64, halation: f64, extra: Vec<(String, ParamValue)>) -> LayerNode {
    let mut v = vals![
        "stock" => stock, "grainOn" => grain > 0.0, "grain" => grain, "grainSize" => 0.8,
        "halationOn" => halation > 0.0, "halation" => halation
    ];
    v.extend(extra);
    fx("film", None, 1.0, BlendMode::Normal, v)
}

fn hex(s: &str) -> Rgba { Rgba::hex(s) }

mod factory_data {
    use super::*;
    include!("presets_factory.rs");
}

pub fn factory() -> &'static Vec<Preset> {
    static F: OnceLock<Vec<Preset>> = OnceLock::new();
    F.get_or_init(factory_data::all)
}

// MARK: - User store

fn file() -> std::path::PathBuf { crate::settings::support_dir().join("presets.json") }

pub fn user() -> &'static Mutex<Vec<Preset>> {
    static U: OnceLock<Mutex<Vec<Preset>>> = OnceLock::new();
    U.get_or_init(|| {
        let v = std::fs::read(file()).ok().and_then(|d| serde_json::from_slice::<PresetFile>(&d).ok()).map(|f| f.presets).unwrap_or_default();
        Mutex::new(v)
    })
}

fn persist(list: &[Preset]) {
    if let Ok(d) = serde_json::to_vec(&PresetFile { version: 1, app: "EFFECTOR".into(), presets: list.to_vec() }) {
        crate::settings::write_atomic(&file(), &d);
    }
}

/// Factory (when visible) + user presets.
pub fn all() -> Vec<Preset> {
    let mut v: Vec<Preset> = if crate::settings::get().factory_presets_visible { factory().clone() } else { vec![] };
    v.extend(user().lock().unwrap().iter().cloned());
    v
}

pub fn add(name: &str, layers: &[LayerNode], thumbnail_jpeg: Option<Vec<u8>>) {
    use base64::Engine;
    let p = Preset {
        id: Uuid::new_v4(),
        name: name.into(),
        layers: layers.iter().map(|l| l.reidentified()).collect(),
        is_factory: false,
        thumbnail: thumbnail_jpeg.map(|d| base64::engine::general_purpose::STANDARD.encode(d)),
        category: None,
    };
    let mut u = user().lock().unwrap();
    u.insert(0, p);
    persist(&u);
}

pub fn rename(id: Uuid, name: &str) {
    let mut u = user().lock().unwrap();
    if let Some(p) = u.iter_mut().find(|p| p.id == id) {
        p.name = name.into();
    }
    persist(&u);
}

pub fn delete(id: Uuid) {
    let mut u = user().lock().unwrap();
    u.retain(|p| p.id != id);
    persist(&u);
}

pub fn export_bytes(presets: &[Preset]) -> Vec<u8> {
    let ps: Vec<Preset> = presets.iter().cloned().map(|mut p| {
        p.is_factory = false;
        p
    }).collect();
    serde_json::to_vec_pretty(&PresetFile { version: 1, app: "EFFECTOR".into(), presets: ps }).unwrap_or_default()
}

pub fn import_bytes(d: &[u8]) -> Result<usize, String> {
    let f: PresetFile = serde_json::from_slice(d).map_err(|e| e.to_string())?;
    let imported: Vec<Preset> = f
        .presets
        .into_iter()
        .map(|mut p| {
            p.id = Uuid::new_v4();
            p.is_factory = false;
            p.layers = p.layers.iter().map(|l| l.reidentified()).collect();
            p
        })
        .collect();
    let n = imported.len();
    let mut u = user().lock().unwrap();
    for (i, p) in imported.into_iter().enumerate() {
        u.insert(i, p);
    }
    persist(&u);
    Ok(n)
}
