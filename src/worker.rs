//! Background workers: the preview renderer (latest request wins) and a thumbnail queue.
//! Both use the shared GPU; results come back over channels and wake the UI.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use uuid::Uuid;

use crate::gpu::{Ctx, T};
use crate::imageio::{self, SourceImage};
use crate::model::*;
use crate::renderer::Renderer;

pub struct RenderRequest {
    pub generation: u64,
    pub layers: Vec<LayerNode>,
    pub source: Arc<SourceImage>,
    pub crop: CropRect,
    pub canvas: Option<CanvasSpec>,
    pub scale: f64,
    /// Also produce the cropped full-resolution-ish texture for the crop tool.
    pub want_crop_view: bool,
}

pub struct RenderResult {
    pub generation: u64,
    pub output: T,
    pub original: T,
    /// UI copies (premultiplied linear) of output / original.
    pub display: T,
    pub display_original: T,
    pub crop_view: Option<T>,
}

pub struct RenderWorker {
    pending: Arc<Mutex<Option<RenderRequest>>>,
    wake: Sender<()>,
    pub results: Receiver<RenderResult>,
}

fn to_display(ctx: &mut Ctx, t: &T) -> T {
    ctx.apply("k_egui", &[t], &[])
}

impl RenderWorker {
    pub fn start(repaint: egui::Context) -> RenderWorker {
        let pending: Arc<Mutex<Option<RenderRequest>>> = Arc::new(Mutex::new(None));
        let (wake, wake_rx) = channel::<()>();
        let (res_tx, results) = channel::<RenderResult>();
        let p2 = pending.clone();
        thread::Builder::new()
            .name("effector-render".into())
            .spawn(move || {
                let mut renderer = Renderer::new(true);
                let mut src_cache: Option<((Uuid, u64, u64), T)> = None;
                let mut crop_cache: Option<(Uuid, T)> = None;
                while wake_rx.recv().is_ok() {
                    let Some(req) = p2.lock().unwrap().take() else { continue };
                    let key = (req.source.id, crop_key(&req.crop), req.scale.to_bits());
                    let src_tex = match &src_cache {
                        Some((k, t)) if *k == key => t.clone(),
                        _ => {
                            let t = req.source.make_texture(&req.crop, req.scale);
                            src_cache = Some((key, t.clone()));
                            t
                        }
                    };
                    let skey = {
                        use std::hash::{Hash, Hasher};
                        let mut h = std::collections::hash_map::DefaultHasher::new();
                        req.source.id.hash(&mut h);
                        crop_key(&req.crop).hash(&mut h);
                        h.finish()
                    };
                    let out = renderer.render(&req.layers, &src_tex, skey, false, req.canvas.as_ref());
                    let mut ctx = Ctx::new(1.0, out.clone(), false);
                    let display = to_display(&mut ctx, &out);
                    let display_original = to_display(&mut ctx, &src_tex);
                    let crop_view = if req.want_crop_view {
                        match &crop_cache {
                            Some((id, t)) if *id == req.source.id => Some(t.clone()),
                            _ => {
                                let s = (2400.0 / req.source.width.max(req.source.height) as f64).min(1.0);
                                let full = req.source.make_texture(&CropRect::default(), s);
                                let d = to_display(&mut ctx, &full);
                                crop_cache = Some((req.source.id, d.clone()));
                                Some(d)
                            }
                        }
                    } else {
                        None
                    };
                    ctx.flush();
                    crate::gpu::gpu().trim_pool(512 << 20);
                    let _ = res_tx.send(RenderResult { generation: req.generation, output: out, original: src_tex, display, display_original, crop_view });
                    repaint.request_repaint();
                }
            })
            .expect("render thread");
        RenderWorker { pending, wake, results }
    }

    /// Queues a render; an older request that has not started yet is replaced.
    pub fn request(&self, r: RenderRequest) {
        *self.pending.lock().unwrap() = Some(r);
        let _ = self.wake.send(());
    }
}

fn crop_key(c: &CropRect) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for v in [c.x, c.y, c.w, c.h] {
        v.to_bits().hash(&mut h);
    }
    h.finish()
}

// MARK: - Thumbnails

pub enum ThumbJob {
    /// Effect browser thumbnails (one per effect) from the active image.
    Effects { source: Arc<SourceImage>, crop: CropRect, generation: u64 },
    /// Preset thumbnails.
    Presets { source: Arc<SourceImage>, crop: CropRect, presets: Vec<(Uuid, Vec<LayerNode>)>, generation: u64 },
    /// Filmstrip thumbnail of one document.
    Strip { doc: Uuid, source: Option<Arc<SourceImage>>, data: Arc<Vec<u8>>, file_name: String, state: DocumentState },
    /// Arbitrary small render (preset thumbnails when saving), answered on `reply`.
    Render { source: Arc<SourceImage>, crop: CropRect, layers: Vec<LayerNode>, size: f64, reply: Sender<Option<(Vec<u8>, usize, usize)>> },
}

pub enum ThumbResult {
    Effect { generation: u64, id: String, rgba: Vec<u8>, w: usize, h: usize },
    Preset { generation: u64, id: Uuid, rgba: Vec<u8>, w: usize, h: usize },
    Strip { doc: Uuid, rgba: Vec<u8>, w: usize, h: usize },
}

pub struct ThumbWorker {
    tx: Sender<ThumbJob>,
    pub results: Receiver<ThumbResult>,
}

pub fn render_small(source: &SourceImage, crop: &CropRect, layers: &[LayerNode], size: f64, canvas: Option<&CanvasSpec>) -> (Vec<u8>, usize, usize) {
    let (cw, ch) = source.cropped_size(crop);
    let s = size / cw.max(ch).max(1) as f64;
    let tex = source.make_texture(crop, s);
    let out = Renderer::new(false).render(layers, &tex, 0, false, canvas);
    let px = imageio::read_rgba8(&out);
    (px, out.width(), out.height())
}

impl ThumbWorker {
    pub fn start(repaint: egui::Context) -> ThumbWorker {
        let (tx, rx) = channel::<ThumbJob>();
        let (res_tx, results) = channel::<ThumbResult>();
        thread::Builder::new()
            .name("effector-thumbs".into())
            .spawn(move || {
                while let Ok(job) = rx.recv() {
                    match job {
                        ThumbJob::Effects { source, crop, generation } => {
                            let (cw, ch) = source.cropped_size(&crop);
                            let s = 220.0 / cw.max(ch).max(1) as f64;
                            let tex = source.make_texture(&crop, s);
                            let mut r = Renderer::new(false);
                            for eff in crate::effects::all() {
                                let node = LayerNode::effect(eff.id, None, None);
                                let out = r.render(&[node], &tex, 0, false, None);
                                let px = imageio::read_rgba8(&out);
                                let _ = res_tx.send(ThumbResult::Effect { generation, id: eff.id.into(), rgba: px, w: out.width(), h: out.height() });
                                repaint.request_repaint();
                            }
                        }
                        ThumbJob::Presets { source, crop, presets, generation } => {
                            let (cw, ch) = source.cropped_size(&crop);
                            let s = 180.0 / cw.max(ch).max(1) as f64;
                            let tex = source.make_texture(&crop, s);
                            let mut r = Renderer::new(false);
                            for (id, layers) in presets {
                                let out = r.render(&layers, &tex, 0, false, None);
                                let px = imageio::read_rgba8(&out);
                                let _ = res_tx.send(ThumbResult::Preset { generation, id, rgba: px, w: out.width(), h: out.height() });
                                repaint.request_repaint();
                            }
                        }
                        ThumbJob::Strip { doc, source, data, file_name, state } => {
                            let src = match source {
                                Some(s) => s,
                                None => match SourceImage::load_data(data, &file_name) {
                                    Ok(s) => Arc::new(s),
                                    Err(_) => continue,
                                },
                            };
                            let (px, w, h) = render_small(&src, &state.crop, &state.layers, 180.0, state.active_canvas());
                            let _ = res_tx.send(ThumbResult::Strip { doc, rgba: px, w, h });
                            repaint.request_repaint();
                        }
                        ThumbJob::Render { source, crop, layers, size, reply } => {
                            let r = render_small(&source, &crop, &layers, size, None);
                            let _ = reply.send(Some(r));
                        }
                    }
                    crate::gpu::gpu().trim_pool(64 << 20);
                }
            })
            .expect("thumb thread");
        ThumbWorker { tx, results }
    }

    pub fn send(&self, j: ThumbJob) { let _ = self.tx.send(j); }
}
