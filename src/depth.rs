//! Depth maps: a monocular depth model (Depth Anything V2 Small, Apache-2.0) estimates how near
//! every pixel is, entirely on this computer (CPU, ~1 s per photo). The map is computed once per
//! image, on demand (only when a layer uses depth), and kept in memory.
//!
//! Values are normalised 0..1 with 1 = nearest. The map covers the whole (uncropped) image; the
//! renderer crops and upsamples it to the render size with an edge-aware filter (`k_depthup`).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use candle_core::{DType, Device, Module, Shape, Tensor};
use candle_nn::var_builder::SimpleBackend;
use candle_nn::VarBuilder;
use candle_transformers::models::depth_anything_v2::{DepthAnythingV2, DepthAnythingV2Config};
use candle_transformers::models::dinov2;
use uuid::Uuid;

use crate::imageio::SourceImage;
use crate::model::{CropRect, LayerNode};

pub const MODEL_FILE: &str = "depth_anything_v2_vits_f16.safetensors";
const INPUT: usize = 518;

/// A computed depth map (whole image, model resolution).
pub struct DepthMap {
    pub w: usize,
    pub h: usize,
    /// row-major, 0..1, 1 = near
    pub data: Vec<f32>,
}

impl DepthMap {
    /// Depth at normalised image coordinates (bilinear).
    pub fn sample(&self, u: f64, v: f64) -> f32 {
        let x = (u.clamp(0.0, 1.0) * (self.w - 1) as f64) as f32;
        let y = (v.clamp(0.0, 1.0) * (self.h - 1) as f64) as f32;
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let g = |x: usize, y: usize| self.data[y * self.w + x];
        let a = g(x0, y0) * (1.0 - fx) + g(x1, y0) * fx;
        let b = g(x0, y1) * (1.0 - fx) + g(x1, y1) * fx;
        a * (1.0 - fy) + b * fy
    }
}

/// What the renderer needs to use depth for one render.
#[derive(Clone)]
pub struct DepthSrc {
    pub map: Arc<DepthMap>,
    pub crop: CropRect,
}

#[derive(Default)]
struct Store {
    maps: HashMap<Uuid, Arc<DepthMap>>,
    running: HashSet<Uuid>,
    failed: HashMap<Uuid, String>,
    /// bumps whenever a map finishes (the UI re-renders when it changes)
    generation: u64,
}

fn store() -> &'static Mutex<Store> {
    static S: OnceLock<Mutex<Store>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Store::default()))
}

fn lock() -> std::sync::MutexGuard<'static, Store> { store().lock().unwrap_or_else(|e| e.into_inner()) }

pub fn generation() -> u64 { lock().generation }
pub fn get(id: Uuid) -> Option<Arc<DepthMap>> { lock().maps.get(&id).cloned() }
pub fn is_running(id: Uuid) -> bool { lock().running.contains(&id) }
pub fn error(id: Uuid) -> Option<String> { lock().failed.get(&id).cloned() }
pub fn forget(id: Uuid) {
    let mut s = lock();
    s.maps.remove(&id);
    s.failed.remove(&id);
}

/// Starts computing the depth map of `src` in the background (no-op if done or running).
/// `done` is called (from the worker thread) when it finishes.
pub fn request(src: &Arc<SourceImage>, done: impl FnOnce() + Send + 'static) {
    {
        let mut s = lock();
        if s.maps.contains_key(&src.id) || s.running.contains(&src.id) || s.failed.contains_key(&src.id) {
            return;
        }
        s.running.insert(src.id);
    }
    let src = src.clone();
    std::thread::Builder::new()
        .name("effector-depth".into())
        .spawn(move || {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| compute(&src)));
            let mut s = lock();
            s.running.remove(&src.id);
            match r {
                Ok(Ok(m)) => {
                    s.maps.insert(src.id, Arc::new(m));
                }
                Ok(Err(e)) => {
                    crate::settings::log_error(&format!("depth: {e}"));
                    s.failed.insert(src.id, e);
                }
                Err(_) => {
                    crate::settings::log_error("depth: panic");
                    s.failed.insert(src.id, "internal error".into());
                }
            }
            s.generation += 1;
            drop(s);
            done();
        })
        .ok();
}

/// Computes (or returns the cached) depth map now, on this thread (exports, tests).
pub fn get_blocking(src: &SourceImage) -> Option<Arc<DepthMap>> {
    if let Some(m) = get(src.id) {
        return Some(m);
    }
    // another thread is already on it: wait for it
    while is_running(src.id) {
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    if let Some(m) = get(src.id) {
        return Some(m);
    }
    match compute(src) {
        Ok(m) => {
            let m = Arc::new(m);
            let mut s = lock();
            s.maps.insert(src.id, m.clone());
            s.generation += 1;
            Some(m)
        }
        Err(e) => {
            crate::settings::log_error(&format!("depth: {e}"));
            None
        }
    }
}

// MARK: - Which layers use depth

/// Does this stack need a depth map to render?
pub fn layers_need_depth(layers: &[LayerNode]) -> bool {
    layers.iter().any(|n| {
        n.enabled
            && (n.depth_mask.as_ref().map(|m| m.enabled).unwrap_or(false)
                || (n.is_group && layers_need_depth(&n.children))
                || (!n.is_group && crate::effects::effect(&n.effect_id).map(|e| (e.uses_depth)(&n.params)).unwrap_or(false)))
    })
}

// MARK: - Model

/// Where the model file is looked for (first existing wins).
pub fn model_candidates() -> Vec<PathBuf> {
    let mut v = vec![];
    if let Ok(p) = std::env::var("EFFECTOR_DEPTH_MODEL") {
        v.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            v.push(dir.join("../Resources/models").join(MODEL_FILE)); // macOS .app
            v.push(dir.join("models").join(MODEL_FILE)); // Windows install / next to the binary
        }
    }
    v.push(crate::settings::support_dir().join("models").join(MODEL_FILE));
    v.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/models").join(MODEL_FILE)); // dev
    v
}

pub fn model_path() -> Option<PathBuf> { model_candidates().into_iter().find(|p| p.is_file()) }
pub fn model_available() -> bool { model_path().is_some() }

/// The DINOv2 classification head is not part of Depth Anything: serve zeros for it.
struct Weights(candle_core::safetensors::MmapedSafetensors);

impl SimpleBackend for Weights {
    fn get(&self, s: Shape, name: &str, h: candle_nn::Init, dtype: DType, dev: &Device) -> candle_core::Result<Tensor> {
        if name.starts_with("pretrained.head.") {
            return Tensor::zeros(s, dtype, dev);
        }
        SimpleBackend::get(&self.0, s, name, h, dtype, dev)
    }
    fn get_unchecked(&self, name: &str, dtype: DType, dev: &Device) -> candle_core::Result<Tensor> {
        SimpleBackend::get_unchecked(&self.0, name, dtype, dev)
    }
    fn contains_tensor(&self, name: &str) -> bool { name.starts_with("pretrained.head.") || SimpleBackend::contains_tensor(&self.0, name) }
}

fn load_model() -> Result<DepthAnythingV2, String> {
    let path = model_path().ok_or_else(|| "depth model not found".to_string())?;
    let st = unsafe { candle_core::safetensors::MmapedSafetensors::new(&path) }.map_err(|e| e.to_string())?;
    let vb = VarBuilder::from_backend(Box::new(Weights(st)), DType::F32, Device::Cpu);
    let dino = dinov2::vit_small(vb.pp("pretrained")).map_err(|e| e.to_string())?;
    DepthAnythingV2::new(Arc::new(dino), DepthAnythingV2Config::vit_small(), vb).map_err(|e| e.to_string())
}

type Job = (Vec<f32>, std::sync::mpsc::Sender<Result<(Vec<f32>, usize, usize), String>>);

/// The model is not `Send`: it lives on one dedicated thread that runs every inference.
fn infer(input: Vec<f32>) -> Result<(Vec<f32>, usize, usize), String> {
    static TX: OnceLock<Mutex<std::sync::mpsc::Sender<Job>>> = OnceLock::new();
    let tx = TX.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("effector-depth-model".into())
            .spawn(move || {
                let mut model: Option<Result<DepthAnythingV2, String>> = None;
                while let Ok((input, reply)) = rx.recv() {
                    let m = model.get_or_insert_with(load_model);
                    let r = match m {
                        Err(e) => Err(e.clone()),
                        Ok(m) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<_, String> {
                            let x = Tensor::from_vec(input, (1, 3, INPUT, INPUT), &Device::Cpu).map_err(|e| e.to_string())?;
                            let y = m.forward(&x).map_err(|e| e.to_string())?;
                            let (_, _, oh, ow) = y.dims4().map_err(|e| e.to_string())?;
                            let v: Vec<f32> = y.flatten_all().and_then(|t| t.to_vec1()).map_err(|e| e.to_string())?;
                            Ok((v, ow, oh))
                        }))
                        .unwrap_or_else(|_| Err("depth model crashed".into())),
                    };
                    let _ = reply.send(r);
                }
            })
            .expect("depth thread");
        Mutex::new(tx)
    });
    let (rtx, rrx) = std::sync::mpsc::channel();
    tx.lock().unwrap_or_else(|e| e.into_inner()).send((input, rtx)).map_err(|e| e.to_string())?;
    rrx.recv().map_err(|e| e.to_string())?
}

fn compute(src: &SourceImage) -> Result<DepthMap, String> {
    let t0 = std::time::Instant::now();
    let (px, w, h) = src.rgba8_thumbnail(INPUT * 2);
    let img = image::RgbaImage::from_raw(w as u32, h as u32, px).ok_or("bad image")?;
    // the model wants 518×518; a uniform squash is undone when the map is sampled in image space
    let r = image::imageops::resize(&img, INPUT as u32, INPUT as u32, image::imageops::FilterType::Triangle);
    let mean = [0.485f32, 0.456, 0.406];
    let std = [0.229f32, 0.224, 0.225];
    let n = INPUT * INPUT;
    let mut d = vec![0f32; 3 * n];
    for (i, p) in r.pixels().enumerate() {
        // transparent pixels read as mid grey so they don't look like a black wall
        let a = p[3] as f32 / 255.0;
        for c in 0..3 {
            let v = p[c] as f32 / 255.0 * a + 0.5 * (1.0 - a);
            d[c * n + i] = (v - mean[c]) / std[c];
        }
    }
    let (v, ow, oh) = infer(d)?;
    // robust normalisation (ignore the extreme 0.5 % at both ends)
    let mut sorted = v.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = sorted[(sorted.len() as f64 * 0.005) as usize];
    let hi = sorted[((sorted.len() as f64 * 0.995) as usize).min(sorted.len() - 1)];
    let span = (hi - lo).max(1e-6);
    let data = v.iter().map(|x| ((x - lo) / span).clamp(0.0, 1.0)).collect();
    if std::env::var("EFFECTOR_DEBUG").is_ok() {
        eprintln!("depth {}×{} in {:?}", ow, oh, t0.elapsed());
    }
    Ok(DepthMap { w: ow, h: oh, data })
}

/// Depth for a render of `layers`: only when the stack uses depth. `blocking` computes it now
/// (exports); otherwise it is used once ready.
pub fn for_render(src: &SourceImage, crop: &CropRect, layers: &[LayerNode], blocking: bool) -> Option<DepthSrc> {
    if !layers_need_depth(layers) {
        return None;
    }
    let map = if blocking { get_blocking(src) } else { get(src.id) }?;
    Some(DepthSrc { map, crop: *crop })
}
