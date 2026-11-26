//! Subject detection: a salient-object segmentation model (ONNX, run with tract on the CPU, the
//! same on macOS and Windows) finds the main subject of a photo, entirely on this computer.
//! The matte is computed once per image, on demand (only when a layer or mask uses it), in the
//! background, and kept in memory.
//!
//! Values are 0..1 with 1 = subject. The matte covers the whole (uncropped) image; the renderer
//! crops and upsamples it to the render size with an edge-aware filter (`k_matteup`), so its
//! edges snap to the real edges of the photo.
//!
//! Replaces the depth map of 0.8.0–0.8.2 (same plumbing: effects that used "near" now use the
//! subject, "far" the background).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use tract_onnx::prelude::*;
use uuid::Uuid;

use crate::imageio::SourceImage;
use crate::model::{CropRect, LayerNode};

/// Model files, best first: (file, input size, normalisation (mean, std)). IS-Net general use
/// (DIS, Apache-2.0, ONNX export of the rembg project): ~2 s per photo on a laptop CPU.
pub const MODELS: [(&str, usize, [f32; 3], [f32; 3]); 1] = [("subject_isnet.onnx", 1024, [0.5, 0.5, 0.5], [1.0, 1.0, 1.0])];

/// A computed subject matte (whole image, model resolution).
pub struct Matte {
    pub w: usize,
    pub h: usize,
    /// row-major, 0..1, 1 = subject
    pub data: Vec<f32>,
}


/// What the renderer needs to use the subject matte for one render.
#[derive(Clone)]
pub struct SubjectSrc {
    pub map: Arc<Matte>,
    pub crop: CropRect,
}

#[derive(Default)]
struct Store {
    maps: HashMap<Uuid, Arc<Matte>>,
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
pub fn get(id: Uuid) -> Option<Arc<Matte>> { lock().maps.get(&id).cloned() }
pub fn is_running(id: Uuid) -> bool { lock().running.contains(&id) }
pub fn error(id: Uuid) -> Option<String> { lock().failed.get(&id).cloned() }
pub fn forget(id: Uuid) {
    let mut s = lock();
    s.maps.remove(&id);
    s.failed.remove(&id);
}

/// Starts computing the subject matte of `src` in the background (no-op if done or running).
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
        .name("effector-subject".into())
        .spawn(move || {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| compute(&src)));
            let mut s = lock();
            s.running.remove(&src.id);
            match r {
                Ok(Ok(m)) => {
                    s.maps.insert(src.id, Arc::new(m));
                }
                Ok(Err(e)) => {
                    crate::settings::log_error(&format!("subject: {e}"));
                    s.failed.insert(src.id, e);
                }
                Err(_) => {
                    crate::settings::log_error("subject: panic");
                    s.failed.insert(src.id, "internal error".into());
                }
            }
            s.generation += 1;
            drop(s);
            done();
        })
        .ok();
}

/// Computes (or returns the cached) subject matte now, on this thread (exports, tests).
pub fn get_blocking(src: &SourceImage) -> Option<Arc<Matte>> {
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
            crate::settings::log_error(&format!("subject: {e}"));
            None
        }
    }
}

// MARK: - Which layers use the subject

/// Does this stack need the subject matte? (Pass resolved layers: `DocumentState::render_layers`.)
pub fn layers_need_subject(layers: &[LayerNode]) -> bool {
    layers.iter().any(|n| {
        n.enabled
            && (n.depth_mask.as_ref().map(|m| m.enabled).unwrap_or(false)
                || n.resolved_mask.as_ref().map(|r| r.mask.needs_subject()).unwrap_or(false)
                || n.embedded_mask.as_ref().map(|m| m.needs_subject()).unwrap_or(false)
                || (n.is_group && layers_need_subject(&n.children))
                || (!n.is_group && crate::effects::effect(&n.effect_id).map(|e| (e.uses_subject)(&n.params)).unwrap_or(false)))
    })
}

// MARK: - Model

/// Where model files are looked for (first existing wins), with the model's settings.
pub fn model_candidates() -> Vec<(PathBuf, usize, [f32; 3], [f32; 3])> {
    let mut v = vec![];
    if let Ok(p) = std::env::var("EFFECTOR_SUBJECT_MODEL") {
        let size = std::env::var("EFFECTOR_SUBJECT_SIZE").ok().and_then(|s| s.parse().ok()).unwrap_or(MODELS[0].1);
        v.push((PathBuf::from(p), size, MODELS[0].2, MODELS[0].3));
    }
    let mut dirs = vec![];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.join("../Resources/models")); // macOS .app
            dirs.push(dir.join("models")); // Windows install / next to the binary
        }
    }
    dirs.push(crate::settings::support_dir().join("models"));
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/models")); // dev
    for (file, size, mean, std) in MODELS {
        for d in &dirs {
            v.push((d.join(file), size, mean, std));
        }
    }
    v
}

fn model_choice() -> Option<(PathBuf, usize, [f32; 3], [f32; 3])> { model_candidates().into_iter().find(|c| c.0.is_file()) }
pub fn model_available() -> bool { model_choice().is_some() }

type Plan = Arc<TypedRunnableModel>;

fn load_model(path: &PathBuf, size: usize) -> Result<Plan, String> {
    let e = |x: TractError| x.to_string();
    tract_onnx::onnx()
        .model_for_path(path)
        .map_err(e)?
        .with_input_fact(0, f32::fact([1, 3, size, size]).into())
        .map_err(e)?
        .into_optimized()
        .map_err(e)?
        .into_runnable()
        .map_err(e)
}

type Job = (Vec<f32>, std::sync::mpsc::Sender<Result<Vec<f32>, String>>);

/// One dedicated thread owns the model and runs every inference.
fn infer(input: Vec<f32>, path: PathBuf, size: usize) -> Result<Vec<f32>, String> {
    static TX: OnceLock<Mutex<std::sync::mpsc::Sender<Job>>> = OnceLock::new();
    let tx = TX.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("effector-subject-model".into())
            .spawn(move || {
                let mut model: Option<Result<Plan, String>> = None;
                while let Ok((input, reply)) = rx.recv() {
                    let m = model.get_or_insert_with(|| load_model(&path, size));
                    let r = match m {
                        Err(e) => Err(e.clone()),
                        Ok(m) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<Vec<f32>, String> {
                            let t = Tensor::from_shape(&[1, 3, size, size], &input).map_err(|e| e.to_string())?;
                            let plan: &Plan = m;
                            let out = plan.run(tvec!(t.into())).map_err(|e| e.to_string())?;
                            let v = out[0].to_plain_array_view::<f32>().map_err(|e| e.to_string())?.iter().copied().collect::<Vec<f32>>();
                            Ok(v)
                        }))
                        .unwrap_or_else(|_| Err("subject model crashed".into())),
                    };
                    let _ = reply.send(r);
                }
            })
            .expect("subject thread");
        Mutex::new(tx)
    });
    let (rtx, rrx) = std::sync::mpsc::channel();
    tx.lock().unwrap_or_else(|e| e.into_inner()).send((input, rtx)).map_err(|e| e.to_string())?;
    rrx.recv().map_err(|e| e.to_string())?
}

fn compute(src: &SourceImage) -> Result<Matte, String> {
    let t0 = std::time::Instant::now();
    let (path, size, mean, std) = model_choice().ok_or_else(|| "subject model not found".to_string())?;
    let (px, w, h) = src.rgba8_thumbnail(size * 2);
    let img = image::RgbaImage::from_raw(w as u32, h as u32, px).ok_or("bad image")?;
    // the model wants a square; a uniform squash is undone when the matte is sampled in image space
    let r = image::imageops::resize(&img, size as u32, size as u32, image::imageops::FilterType::Triangle);
    let n = size * size;
    let mut d = vec![0f32; 3 * n];
    for (i, p) in r.pixels().enumerate() {
        let a = p[3] as f32 / 255.0;
        for c in 0..3 {
            let v = p[c] as f32 / 255.0 * a + 0.5 * (1.0 - a);
            d[c * n + i] = (v - mean[c]) / std[c];
        }
    }
    let v = infer(d, path, size)?;
    if v.len() < n {
        return Err("unexpected model output".into());
    }
    // the models output a sigmoid map; stretch it to 0..1 like rembg does
    let (lo, hi) = v[..n].iter().fold((f32::MAX, f32::MIN), |(a, b), x| (a.min(*x), b.max(*x)));
    let span = (hi - lo).max(1e-6);
    let data = v[..n].iter().map(|x| ((x - lo) / span).clamp(0.0, 1.0)).collect();
    if std::env::var("EFFECTOR_DEBUG").is_ok() {
        eprintln!("subject {size}×{size} in {:?}", t0.elapsed());
    }
    Ok(Matte { w: size, h: size, data })
}

/// Matte for a render of `layers`: only when the stack uses it. `blocking` computes it now
/// (exports); otherwise it is used once ready.
pub fn for_render(src: &SourceImage, crop: &CropRect, layers: &[LayerNode], blocking: bool) -> Option<SubjectSrc> {
    if !layers_need_subject(layers) {
        return None;
    }
    let map = if blocking { get_blocking(src) } else { get(src.id) }?;
    Some(SubjectSrc { map, crop: *crop })
}
