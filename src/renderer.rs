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
    pub depth: Option<crate::depth::DepthSrc>,
    /// Capture date (EXIF) for date stamps.
    pub taken: Option<crate::exif::DateTaken>,
}

impl Extras {
    /// Everything `layers` needs for `src`. `blocking` waits for the depth map (exports).
    pub fn for_render(src: &crate::imageio::SourceImage, crop: &CropRect, layers: &[LayerNode], blocking: bool) -> Extras {
        Extras { depth: crate::depth::for_render(src, crop, layers, blocking), taken: crate::exif::date_taken_cached(src.id, &src.data) }
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
        let depth_key = extras.depth.as_ref().map(|d| Arc::as_ptr(&d.map) as usize).unwrap_or(0);
        ctx.depth_src = extras.depth;
        ctx.taken = extras.taken;
        let mut new_cache = HashMap::new();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        source_key.hash(&mut h);
        scale.to_bits().hash(&mut h);
        source.w.hash(&mut h);
        source.h.hash(&mut h);
        depth_key.hash(&mut h);
        extras.taken.hash(&mut h);
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
                ctx.layer_key = stable;
                out = (eff.apply)(&current, &node.params, ctx);
                if out.w != current.w || out.h != current.h {
                    out = ctx.resize(&out, current.width(), current.height(), false);
                }
                // Effects never change transparency: restore the exact input alpha.
                if !Arc::ptr_eq(&out, &current) && !eff.produces_alpha {
                    out = ctx.apply("k_withalpha", &[&out, &current], &[]);
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

/// Mask of a layer: its document mask (`resolved_mask`) × the effect's "Depth" weighting, if any.
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
    let d = ctx.depth()?;
    let mut m = rm.and_then(|r| mask_texture(&r.mask, r.invert, &d, ctx));
    if dir != 0.0 {
        let amount = (node.params.num("depthBiasAmount", 80.0) / 100.0) as f32;
        let b = ctx.apply("k_depthmask", &[&d], &[0.0, 0.0, 0.0, 0.0, 0.0, dir, amount]);
        m = Some(match m {
            Some(x) => ctx.apply("k_maskop", &[&x, &b], &[MaskMode::Intersect.shader_index()]),
            None => b,
        });
    }
    m
}

/// Renders a document mask (1 = selected) at the size of `depth`. The first enabled part is the
/// base; the others add, subtract or intersect. None when no part is enabled.
pub fn mask_texture(mask: &Mask, invert: bool, depth: &T, ctx: &mut Ctx) -> Option<T> {
    let mut acc: Option<T> = None;
    for part in mask.parts.iter().filter(|p| p.enabled) {
        let t = match &part.shape {
            MaskShape::Depth { from, to, feather } => {
                ctx.apply("k_depthmask", &[depth], &[1.0, *from as f32, *to as f32, *feather as f32, if part.invert { 1.0 } else { 0.0 }, 0.0, 0.0])
            }
        };
        acc = Some(match acc {
            None => t,
            Some(a) => ctx.apply("k_maskop", &[&a, &t], &[part.mode.shader_index()]),
        });
    }
    if invert {
        acc = acc.map(|a| ctx.apply("k_maskop", &[&a, &a], &[3.0]));
    }
    acc
}
