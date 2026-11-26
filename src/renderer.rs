//! Renders a layer tree on top of a source texture. Layers are ordered top-first; the bottom-most
//! layer is applied first. Per-layer cache keyed by the hash of everything below, so editing a layer
//! only re-renders from it upwards.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::effects;
use crate::gpu::{Ctx, T};
use crate::model::*;

/// Size parameters are relative: expressed in pixels of a 1000 px (long side) reference image,
/// so a stack looks identical at any resolution (preview, export 1x/2x, thumbnails).
pub const REFERENCE_SIZE: f64 = 1000.0;

/// Per-image data effects may use besides the pixels.
#[derive(Clone, Default)]
pub struct Extras {
    pub subject: Option<crate::subject::SubjectSrc>,
    /// Crop of the render (masks are drawn in image coordinates).
    pub crop: CropRect,
    /// Image width and height divided by its long side.
    pub aspect: (f64, f64),
    /// Capture date (EXIF) for date stamps.
    pub taken: Option<crate::exif::DateTaken>,
}

impl Extras {
    /// Everything `layers` needs for `src`. `blocking` waits for the depth map (exports).
    pub fn for_render(src: &crate::imageio::SourceImage, crop: &CropRect, layers: &[LayerNode], blocking: bool) -> Extras {
        let long = src.width.max(src.height).max(1) as f64;
        Extras {
            subject: crate::subject::for_render(src, crop, layers, blocking),
            crop: *crop,
            aspect: (src.width as f64 / long, src.height as f64 / long),
            taken: crate::exif::date_taken_cached(src.id, &src.data),
        }
    }
}

#[derive(Default)]
pub struct Renderer {
    cache: HashMap<u64, T>,
    use_cache: bool,
}

impl Renderer {
    pub fn new(use_cache: bool) -> Renderer { Renderer { cache: HashMap::new(), use_cache } }

    pub fn clear_cache(&mut self) { self.cache.clear(); }

    /// `source_key` must change whenever the source texture content changes.
    pub fn render(&mut self, layers: &[LayerNode], source: &T, source_key: u64, is_export: bool, canvas: Option<&CanvasSpec>, extras: Extras) -> T {
        let scale = source.width().max(source.height()) as f64 / REFERENCE_SIZE;
        let mut ctx = Ctx::new(scale, source.clone(), is_export);
        ctx.content_key = source_key;
        let depth_key = extras.subject.as_ref().map(|d| Arc::as_ptr(&d.map) as usize).unwrap_or(0);
        ctx.subject_src = extras.subject;
        ctx.crop = extras.crop;
        ctx.aspect = if extras.aspect.0 > 0.0 { extras.aspect } else { (1.0, 1.0) };
        ctx.taken = extras.taken;
        let mut new_cache = HashMap::new();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        source_key.hash(&mut h);
        scale.to_bits().hash(&mut h);
        source.w.hash(&mut h);
        source.h.hash(&mut h);
        depth_key.hash(&mut h);
        extras.taken.hash(&mut h);
        for v in [extras.crop.x, extras.crop.y, extras.crop.w, extras.crop.h, extras.aspect.0, extras.aspect.1] {
            v.to_bits().hash(&mut h);
        }
        let (mut out, _, _) = self.render_stack(layers, source.clone(), h.finish(), source_key, &mut ctx, &mut new_cache);
        if let Some(c) = canvas {
            out = crate::canvas::apply(&out, c, &mut ctx);
        }
        ctx.flush();
        if self.use_cache {
            self.cache = new_cache;
        }
        out
    }

    /// `stable_hash` follows `input_hash` but leaves out the render size (see `Ctx::layer_key`).
    fn render_stack(&self, nodes: &[LayerNode], input: T, input_hash: u64, stable_hash: u64, ctx: &mut Ctx, new_cache: &mut HashMap<u64, T>) -> (T, u64, u64) {
        let mut current = input;
        let mut current_hash = input_hash;
        let mut current_stable = stable_hash;
        for node in nodes.iter().rev() {
            if !node.enabled {
                continue;
            }
            let mut h = std::collections::hash_map::DefaultHasher::new();
            current_hash.hash(&mut h);
            node.render_hash(&mut h);
            let key = h.finish();
            let stable = if stable_hash == 0 {
                0
            } else {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                current_stable.hash(&mut h);
                node.render_hash(&mut h);
                h.finish()
            };
            if self.use_cache {
                if let Some(cached) = self.cache.get(&key).or_else(|| new_cache.get(&key)) {
                    current = cached.clone();
                    current_hash = key;
                    current_stable = stable;
                    new_cache.insert(key, current.clone());
                    continue;
                }
            }
            let mut out: T;
            if node.is_group {
                let (sub, _, _) = self.render_stack(&node.children, current.clone(), current_hash, current_stable, ctx, new_cache);
                out = sub;
            } else if let Some(eff) = effects::effect(&node.effect_id) {
                // the effect's output before mask / opacity / blend, cached on its own
                let fx_key = {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    0xFEEDu16.hash(&mut h);
                    current_hash.hash(&mut h);
                    node.effect_hash(&mut h);
                    h.finish()
                };
                let cached = if self.use_cache { self.cache.get(&fx_key).or_else(|| new_cache.get(&fx_key)).cloned() } else { None };
                out = match cached {
                    Some(c) => c,
                    None => {
                        ctx.layer_key = stable;
                        let mut o = (eff.apply)(&current, &node.params, ctx);
                        if o.w != current.w || o.h != current.h {
                            o = ctx.resize(&o, current.width(), current.height(), false);
                        }
                        // Effects never change transparency: restore the exact input alpha.
                        if !Arc::ptr_eq(&o, &current) && !eff.produces_alpha {
                            o = ctx.apply("k_withalpha", &[&o, &current], &[]);
                        }
                        o
                    }
                };
                if self.use_cache && (node.resolved_mask.is_some() || node.params.choice_or("depthBias", "off") != "off") {
                    new_cache.insert(fx_key, out.clone());
                }
            } else {
                continue;
            }
            let mask = layer_mask(node, ctx);
            if let Some(m) = mask {
                out = ctx.apply("k_blendm", &[&current, &out, &m], &[node.opacity as f32, node.blend.shader_index()]);
            } else if node.opacity < 0.999 || node.blend != BlendMode::Normal || !Arc::ptr_eq(&out, &current) {
                out = ctx.apply("k_blend", &[&current, &out], &[node.opacity as f32, node.blend.shader_index()]);
            }
            if self.use_cache {
                new_cache.insert(key, out.clone());
            }
            current = out;
            current_hash = key;
            current_stable = stable;
        }
        (current, current_hash, current_stable)
    }
}

/// Mask of a layer: its document mask (`resolved_mask`) × the effect's "Where" weighting
/// (subject / background), if any.
fn layer_mask(node: &LayerNode, ctx: &mut Ctx) -> Option<T> {
    let rm = node.resolved_mask.as_ref().filter(|r| r.mask.is_active());
    let bias = if node.is_group { "off".to_string() } else { node.params.choice_or("depthBias", "off") };
    let dir = match bias.as_str() {
        "near" => 1.0,
        "far" => -1.0,
        _ => 0.0,
    };
    if rm.is_none() && dir == 0.0 {
        return None;
    }
    let mut m = rm.and_then(|r| mask_texture(&r.mask, r.invert, ctx));
    if dir != 0.0 {
        if let Some(d) = ctx.subject() {
            let amount = (node.params.num("depthBiasAmount", 80.0) / 100.0) as f32;
            let b = ctx.apply("k_depthmask", &[&d], &[0.0, 0.0, 0.0, 0.0, 0.0, dir, amount]);
            m = Some(match m {
                Some(x) => ctx.apply("k_maskop", &[&x, &b], &[MaskMode::Intersect.shader_index()]),
                None => b,
            });
        }
    }
    m
}

/// Renders one mask part (1 = selected) at the render size. None when it needs the subject
/// matte and that is not ready yet.
pub fn part_texture(shape: &MaskShape, ctx: &mut Ctx) -> Option<T> {
    let (w, h) = (ctx.width, ctx.height);
    let c = ctx.crop;
    let (ax, ay) = ctx.aspect;
    let g = [c.x as f32, c.y as f32, c.w as f32, c.h as f32, ax as f32, ay as f32];
    let src = ctx.source.clone();
    Some(match shape {
        MaskShape::Linear { x, y, angle, width } => {
            let mut u = vec![0.0];
            u.extend(g);
            u.extend([*x as f32, *y as f32, *angle as f32, *width as f32]);
            ctx.apply_sized("k_maskgeom", &[&src], &u, None, w, h)
        }
        MaskShape::Radial { x, y, rx, ry, angle, feather } => {
            let mut u = vec![1.0];
            u.extend(g);
            u.extend([*x as f32, *y as f32, *rx as f32, *ry as f32, *angle as f32, *feather as f32]);
            ctx.apply_sized("k_maskgeom", &[&src], &u, None, w, h)
        }
        MaskShape::Brush { strokes } => {
            let (rw, rh) = crate::brush::raster_size(w, h);
            let cov = crate::brush::raster(strokes, rw, rh, &c, ctx.aspect);
            let px: Vec<f32> = cov.iter().flat_map(|v| [*v, *v, *v, 1.0]).collect();
            let low = ctx.gpu().texture_from_rgba_f32(&px, rw, rh, 1);
            if (rw, rh) == (w, h) { low } else { ctx.resize(&low, w, h, true) }
        }
        MaskShape::Subject { softness, grow } | MaskShape::Background { softness, grow } => {
            let d = ctx.subject()?;
            let (sf, gr) = (softness.clamp(0.0, 1.0), grow.clamp(-1.0, 1.0));
            // blur so the edge can move (grow / shrink) and soften
            let r = ctx.px(2.0 + sf * 30.0 + gr.abs() * 30.0);
            let b = if r > 0.6 { ctx.blur(&d, r) } else { d };
            let thr = (0.5 - gr * 0.42) as f32;
            let hw = (0.04 + sf * 0.45) as f32;
            let inv = if matches!(shape, MaskShape::Background { .. }) { 1.0 } else { 0.0 };
            ctx.apply("k_subjectmask", &[&b], &[thr, hw, inv])
        }
        MaskShape::Depth { from, to, feather } => {
            let d = ctx.subject()?;
            ctx.apply("k_depthmask", &[&d], &[1.0, *from as f32, *to as f32, *feather as f32, 0.0, 0.0, 0.0])
        }
    })
}

/// Renders a document mask (1 = selected) at the render size. The first enabled part is the
/// base; the others add, subtract or intersect. None when no part is enabled.
pub fn mask_texture(mask: &Mask, invert: bool, ctx: &mut Ctx) -> Option<T> {
    let mut acc: Option<T> = None;
    for part in mask.parts.iter().filter(|p| p.enabled) {
        let Some(mut t) = part_texture(&part.shape, ctx) else { continue };
        if part.invert {
            t = ctx.apply("k_maskop", &[&t, &t], &[3.0]);
        }
        acc = Some(match acc {
            None if part.mode == MaskMode::Subtract => {
                // a lone subtract part: everything minus it
                ctx.apply("k_maskop", &[&t, &t], &[3.0])
            }
            None => t,
            Some(a) => ctx.apply("k_maskop", &[&a, &t], &[part.mode.shader_index()]),
        });
    }
    if invert {
        acc = acc.map(|a| ctx.apply("k_maskop", &[&a, &a], &[3.0]));
    }
    acc
}
