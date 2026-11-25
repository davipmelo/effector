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
    /// Show the depth map instead of the result.
    pub show_depth: bool,
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

/// What the render thread answers: a finished render, or the generation that failed (the
/// previous image stays on screen and the next edit renders again from scratch).
pub enum RenderMsg {
    Done(RenderResult),
    Failed(u64),
}

pub struct RenderWorker {
    pending: Arc<Mutex<Option<RenderRequest>>>,
    wake: Sender<()>,
    pub results: Receiver<RenderMsg>,
    thread: thread::JoinHandle<()>,
}

fn lock<V>(m: &Mutex<V>) -> std::sync::MutexGuard<'_, V> { m.lock().unwrap_or_else(|e| e.into_inner()) }

fn to_display(ctx: &mut Ctx, t: &T) -> T {
    ctx.apply("k_egui", &[t], &[])
}

impl RenderWorker {
    pub fn start(repaint: egui::Context) -> RenderWorker {
        let pending: Arc<Mutex<Option<RenderRequest>>> = Arc::new(Mutex::new(None));
        let (wake, wake_rx) = channel::<()>();
        let (res_tx, results) = channel::<RenderMsg>();
        let p2 = pending.clone();
        let thread = thread::Builder::new()
            .name("effector-render".into())
            .spawn(move || {
                let mut renderer = Renderer::new(true);
                let mut src_cache: Option<((Uuid, u64, u64), T)> = None;
                let mut crop_cache: Option<(Uuid, T)> = None;
                while wake_rx.recv().is_ok() {
                    let Some(req) = lock(&p2).take() else { continue };
                    let generation = req.generation;
                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
                        let extras = crate::renderer::Extras::for_render(&req.source, &req.crop, &req.layers, false);
                        let out = renderer.render(&req.layers, &src_tex, skey, false, req.canvas.as_ref(), extras);
                        let mut ctx = Ctx::new(1.0, out.clone(), false);
                        let mut display = to_display(&mut ctx, &out);
                        if req.show_depth {
                            if let Some(map) = crate::depth::get(req.source.id) {
                                let mut dc = Ctx::new(1.0, src_tex.clone(), false);
                                dc.depth_src = Some(crate::depth::DepthSrc { map, crop: req.crop });
                                if let Some(d) = dc.depth() {
                                    let v = dc.apply("k_depthview", &[&d], &[]);
                                    let v = if v.width() != out.width() || v.height() != out.height() { dc.resize(&v, out.width(), out.height(), true) } else { v };
                                    display = to_display(&mut dc, &v);
                                }
                                dc.flush();
                            }
                        }
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
                        RenderResult { generation: req.generation, output: out, original: src_tex, display, display_original, crop_view }
                    }));
                    match res {
                        Ok(r) => {
                            let _ = res_tx.send(RenderMsg::Done(r));
                        }
                        Err(_) => {
                            // start clean: a half-finished render may have left bad cache entries
                            renderer = Renderer::new(true);
                            src_cache = None;
                            crop_cache = None;
                            let _ = res_tx.send(RenderMsg::Failed(generation));
                        }
                    }
                    repaint.request_repaint();
                }
            })
            .expect("render thread");
        RenderWorker { pending, wake, results, thread }
    }

    /// Queues a render; an older request that has not started yet is replaced.
    pub fn request(&self, r: RenderRequest) {
        *lock(&self.pending) = Some(r);
        let _ = self.wake.send(());
    }

    /// The render thread is gone (it should never happen; the app then starts a new one).
    pub fn is_dead(&self) -> bool { self.thread.is_finished() }
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
    let extras = crate::renderer::Extras::for_render(source, crop, layers, false);
    let out = Renderer::new(false).render(layers, &tex, 0, false, canvas, extras);
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
                    // a failing thumbnail must never take the thread down
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match job {
                        ThumbJob::Effects { source, crop, generation } => {
                            let (cw, ch) = source.cropped_size(&crop);
                            let s = 220.0 / cw.max(ch).max(1) as f64;
                            let tex = source.make_texture(&crop, s);
                            let mut r = Renderer::new(false);
                            for eff in crate::effects::all().iter().filter(|e| !e.hidden) {
                                let node = LayerNode::effect(eff.id, None, None);
                                let out = r.render(&[node], &tex, 0, false, None, Default::default());
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
                                let extras = crate::renderer::Extras::for_render(&source, &crop, &layers, false);
                                let out = r.render(&layers, &tex, 0, false, None, extras);
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
                                    Err(_) => return,
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
                    }));
                    crate::gpu::gpu().trim_pool(64 << 20);
                }
            })
            .expect("thumb thread");
        ThumbWorker { tx, results }
    }

    pub fn send(&self, j: ThumbJob) { let _ = self.tx.send(j); }
}
