//! Image Overlay (0.9.1): the built-in texture library and the user's own overlay images.
//!
//! The library (dust, grunge, glass, paper, plastic…) is not shipped with the app: it is downloaded
//! once, the first time the effect is used, from the `textures-v<N>` release of this repo, and
//! unpacked into `<support>/Textures` (`manifest.json`, `<category>/<name>.jpg`,
//! `thumbs/<category>/<name>.jpg`). The user's images travel inside the layer value
//! (`ImageRef.data`), shrunk to `USER_MAX_SIDE` and re-encoded so presets stay small.
//! Decoded images are kept on the GPU in a small cache shared by the preview and the export.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Deserialize;

use crate::gpu::T;
use crate::model::ImageRef;

pub const PACK_VERSION: u32 = 1;
const PACK_URL: &str = "https://github.com/davipmelo/effector/releases/download/textures-v1/effector-textures-v1.zip";
/// Shown before the download starts (the real size comes from the server).
pub const PACK_APPROX_MB: u64 = 235;
const AGENT: &str = concat!("EFFECTOR/", env!("CARGO_PKG_VERSION"));
/// The user's images are shrunk to this long side when added (keeps presets and projects light).
pub const USER_MAX_SIDE: u32 = 3072;
/// Built-in texture of a new Image Overlay layer.
pub const DEFAULT_TEXTURE: (&str, &str) = ("dust/dust-1", "Dust 1");

// MARK: - Library

#[derive(Clone, Debug, Deserialize)]
pub struct Item {
    /// `category/name`
    pub id: String,
    pub name: String,
    pub file: String,
    pub thumb: String,
    #[serde(default)]
    pub w: u32,
    #[serde(default)]
    pub h: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Deserialize)]
struct Manifest {
    version: u32,
    categories: Vec<Category>,
}

pub fn library_dir() -> PathBuf { crate::settings::support_dir().join("Textures") }

fn library_cell() -> &'static Mutex<Option<Arc<Vec<Category>>>> {
    static L: OnceLock<Mutex<Option<Arc<Vec<Category>>>>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(read_manifest()))
}

fn read_manifest() -> Option<Arc<Vec<Category>>> {
    let d = std::fs::read(library_dir().join("manifest.json")).ok()?;
    let m: Manifest = serde_json::from_slice(&d).ok()?;
    (m.version >= PACK_VERSION && !m.categories.is_empty()).then(|| Arc::new(m.categories))
}

/// The installed library (None until it has been downloaded).
pub fn library() -> Option<Arc<Vec<Category>>> { library_cell().lock().unwrap_or_else(|e| e.into_inner()).clone() }

pub fn is_ready() -> bool { library().is_some() }

pub fn find(id: &str) -> Option<Item> { library()?.iter().flat_map(|c| c.items.iter()).find(|i| i.id == id).cloned() }

pub fn file_of(item: &Item) -> PathBuf { library_dir().join(&item.file) }
pub fn thumb_of(item: &Item) -> PathBuf { library_dir().join(&item.thumb) }

/// Changes whenever the library appears (renders keyed on it redo the overlay).
pub fn generation() -> u64 { GENERATION.load(Ordering::Relaxed) }
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Can this image be drawn right now?
pub fn is_available(r: &ImageRef) -> bool {
    if r.data.is_some() {
        return true;
    }
    r.builtin_id().map(|id| find(id).is_some()).unwrap_or(false)
}

// MARK: - Download

#[derive(Clone, Debug, PartialEq)]
pub enum Download {
    Idle,
    Running { done: u64, total: u64 },
    Unpacking,
    Failed(String),
}

fn dl_cell() -> &'static Mutex<Download> {
    static S: OnceLock<Mutex<Download>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Download::Idle))
}

pub fn download_state() -> Download { dl_cell().lock().unwrap_or_else(|e| e.into_inner()).clone() }
fn set_dl(s: Download) { *dl_cell().lock().unwrap_or_else(|e| e.into_inner()) = s; }

fn notifier() -> &'static Mutex<Option<egui::Context>> {
    static N: OnceLock<Mutex<Option<egui::Context>>> = OnceLock::new();
    N.get_or_init(|| Mutex::new(None))
}

/// The UI to repaint while downloading.
pub fn set_notify(ctx: egui::Context) { *notifier().lock().unwrap_or_else(|e| e.into_inner()) = Some(ctx); }

fn notify() {
    if let Some(c) = notifier().lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        c.request_repaint();
    }
}

/// Starts downloading the library unless it is installed, running or failed (`retry` restarts a
/// failed one).
pub fn ensure_library(retry: bool) {
    if is_ready() {
        return;
    }
    match download_state() {
        Download::Running { .. } | Download::Unpacking => return,
        Download::Failed(_) if !retry => return,
        _ => {}
    }
    set_dl(Download::Running { done: 0, total: 0 });
    notify();
    std::thread::spawn(|| {
        let r = fetch_and_unpack();
        match r {
            Ok(()) => {
                *library_cell().lock().unwrap_or_else(|e| e.into_inner()) = read_manifest();
                GENERATION.fetch_add(1, Ordering::Relaxed);
                set_dl(Download::Idle);
            }
            Err(e) => {
                crate::settings::log_error(&format!("texture library: {e}"));
                set_dl(Download::Failed(e));
            }
        }
        notify();
    });
}

fn fetch_and_unpack() -> Result<(), String> {
    let support = crate::settings::support_dir();
    let zip_path = support.join("Textures.download");
    let local = std::env::var("EFFECTOR_TEXTURES_ZIP").ok().map(PathBuf::from);
    if let Some(p) = &local {
        // dev / offline: install from a local pack
        std::fs::copy(p, &zip_path).map_err(|e| e.to_string())?;
    } else {
        let mut resp = ureq::get(PACK_URL).header("User-Agent", AGENT).call().map_err(|e| format!("download failed ({e})"))?;
        let total = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        let mut reader = resp.body_mut().as_reader();
        let mut out = std::fs::File::create(&zip_path).map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; 1 << 16];
        let (mut done, mut last) = (0u64, std::time::Instant::now());
        loop {
            let n = reader.read(&mut buf).map_err(|e| format!("download interrupted ({e})"))?;
            if n == 0 {
                break;
            }
            std::io::Write::write_all(&mut out, &buf[..n]).map_err(|e| e.to_string())?;
            done += n as u64;
            if last.elapsed().as_millis() > 100 {
                set_dl(Download::Running { done, total });
                notify();
                last = std::time::Instant::now();
            }
        }
        if total > 0 && done != total {
            let _ = std::fs::remove_file(&zip_path);
            return Err("download incomplete".into());
        }
    }
    set_dl(Download::Unpacking);
    notify();
    let tmp = support.join("Textures.unpack");
    let _ = std::fs::remove_dir_all(&tmp);
    let r = unpack(&zip_path, &tmp);
    let _ = std::fs::remove_file(&zip_path);
    r?;
    let dest = library_dir();
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
    Ok(())
}

fn unpack(zip_path: &Path, dir: &Path) -> Result<(), String> {
    let f = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|_| "the texture pack is damaged".to_string())?;
    for i in 0..z.len() {
        let mut e = z.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = e.enclosed_name() else { continue };
        let out = dir.join(rel);
        if e.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut data = Vec::with_capacity(e.size() as usize);
        // reading checks the CRC of every file
        e.read_to_end(&mut data).map_err(|_| "the texture pack is damaged".to_string())?;
        std::fs::write(&out, data).map_err(|e| e.to_string())?;
    }
    if !dir.join("manifest.json").exists() {
        return Err("the texture pack has no manifest".into());
    }
    Ok(())
}

// MARK: - The user's images

/// Stable 64-bit FNV-1a (names the content of an image across runs and machines).
fn fnv64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in data {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Shrinks and re-encodes pixels into an overlay image (JPEG, or PNG when it has transparency).
pub fn user_image_from_rgba(rgba: Vec<u8>, w: usize, h: usize, name: &str) -> Result<ImageRef, String> {
    let mut img = image::RgbaImage::from_raw(w as u32, h as u32, rgba).ok_or("bad image size")?;
    let long = img.width().max(img.height());
    if long > USER_MAX_SIDE {
        let s = USER_MAX_SIDE as f64 / long as f64;
        let (nw, nh) = (((img.width() as f64 * s).round() as u32).max(1), ((img.height() as f64 * s).round() as u32).max(1));
        img = image::imageops::thumbnail(&img, nw, nh);
    }
    let (w, h) = (img.width() as usize, img.height() as usize);
    let alpha = img.pixels().any(|p| p[3] < 250);
    let fmt = if alpha { crate::imageio::Format::Png } else { crate::imageio::Format::Jpg };
    let data = crate::imageio::encode(img.as_raw(), w, h, fmt, 0.9, alpha)?;
    let stem = Path::new(name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| name.to_string());
    Ok(ImageRef { id: format!("user:{:016x}", fnv64(&data)), name: stem, data: Some(Arc::new(data)) })
}

/// Any image file the app can open (JPEG, PNG, TIFF, HEIC, RAW…).
pub fn user_image_from_file(path: &Path) -> Result<ImageRef, String> {
    let src = crate::imageio::SourceImage::load_path(path)?;
    let (px, w, h) = src.rgba8_thumbnail(src.width.max(src.height));
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Image".into());
    user_image_from_rgba(px, w, h, &name)
}

// MARK: - Decoding and the GPU cache

pub struct Decoded {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
}

fn decode_bytes(data: &[u8]) -> Option<Decoded> {
    let img = image::load_from_memory(data).ok()?.to_rgba8();
    Some(Decoded { w: img.width() as usize, h: img.height() as usize, rgba: img.into_raw() })
}

/// Pixels of an overlay image (None while a built-in texture is not downloaded).
pub fn decode(r: &ImageRef) -> Option<Decoded> {
    if let Some(d) = &r.data {
        return decode_bytes(d);
    }
    let item = find(r.builtin_id()?)?;
    decode_bytes(&std::fs::read(file_of(&item)).ok()?)
}

/// Small RGBA copy for the picker (built-ins read their thumbnail file).
pub fn thumbnail(r: &ImageRef, max_side: u32) -> Option<Decoded> {
    let bytes = match (&r.data, r.builtin_id().and_then(find)) {
        (Some(d), _) => d.as_ref().clone(),
        (None, Some(item)) => std::fs::read(thumb_of(&item)).or_else(|_| std::fs::read(file_of(&item))).ok()?,
        _ => return None,
    };
    let img = image::load_from_memory(&bytes).ok()?;
    let img = if img.width().max(img.height()) > max_side { img.thumbnail(max_side, max_side) } else { img };
    let img = img.to_rgba8();
    Some(Decoded { w: img.width() as usize, h: img.height() as usize, rgba: img.into_raw() })
}

fn gpu_cache() -> &'static Mutex<Vec<(String, T)>> {
    static C: OnceLock<Mutex<Vec<(String, T)>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// The image as a GPU texture (rgba8, sRGB values), kept for the next renders.
pub fn texture(r: &ImageRef) -> Option<T> {
    {
        let mut c = gpu_cache().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(i) = c.iter().position(|(id, _)| *id == r.id) {
            let e = c.remove(i);
            let t = e.1.clone();
            c.push(e);
            return Some(t);
        }
    }
    let d = decode(r)?;
    let g = crate::gpu::gpu();
    let (mut w, mut h, mut px) = (d.w, d.h, d.rgba);
    let max = g.max_dim as usize;
    if w > max || h > max {
        let img = image::RgbaImage::from_raw(w as u32, h as u32, px)?;
        let s = max as f64 / w.max(h) as f64;
        let small = image::imageops::thumbnail(&img, ((w as f64 * s) as u32).max(1), ((h as f64 * s) as u32).max(1));
        (w, h, px) = (small.width() as usize, small.height() as usize, small.into_raw());
    }
    let t = g.texture_from_rgba8(&px, w, h);
    let mut c = gpu_cache().lock().unwrap_or_else(|e| e.into_inner());
    c.push((r.id.clone(), t.clone()));
    // a few full-size images at most (~20 MB each)
    while c.len() > 4 {
        c.remove(0);
    }
    Some(t)
}

/// Does any of these layers draw a built-in texture? (starts the library download)
pub fn layers_use_library(nodes: &[crate::model::LayerNode]) -> bool {
    nodes.iter().any(|n| {
        if n.is_group {
            return layers_use_library(&n.children);
        }
        // only image parameters in use (e.g. Frames & Borders' paper only with Torn Paper + Paper)
        let Some(e) = crate::effects::effect(&n.effect_id) else { return false };
        e.params.iter().any(|d| {
            matches!(d.kind, crate::model::ParamKind::Image(_))
                && d.is_visible(&n.params)
                && matches!(n.params.get(&d.key), Some(crate::model::ParamValue::Image(r)) if r.builtin_id().is_some())
        })
    })
}
