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
        let (mut out, _) = self.render_stack(layers, source.clone(), h.finish(), &mut ctx, &mut new_cache);
        if let Some(c) = canvas {
            out = crate::canvas::apply(&out, c, &mut ctx);
        }
        ctx.flush();
        if self.use_cache {
            self.cache = new_cache;
        }
        out
    }

    fn render_stack(&self, nodes: &[LayerNode], input: T, input_hash: u64, ctx: &mut Ctx, new_cache: &mut HashMap<u64, T>) -> (T, u64) {
        let mut current = input;
        let mut current_hash = input_hash;
        for node in nodes.iter().rev() {
            if !node.enabled {
                continue;
            }
            let mut h = std::collections::hash_map::DefaultHasher::new();
            current_hash.hash(&mut h);
            node.render_hash(&mut h);
            let key = h.finish();
            if self.use_cache {
                if let Some(cached) = self.cache.get(&key).or_else(|| new_cache.get(&key)) {
                    current = cached.clone();
                    current_hash = key;
                    new_cache.insert(key, current.clone());
                    continue;
                }
            }
            let mut out: T;
            if node.is_group {
                let (sub, _) = self.render_stack(&node.children, current.clone(), current_hash, ctx, new_cache);
                out = sub;
            } else if let Some(eff) = effects::effect(&node.effect_id) {
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
        }
        (current, current_hash)
    }
}

/// Depth mask of a layer (its own range mask × the effect's "Depth" weighting), if any.
fn layer_mask(node: &LayerNode, ctx: &mut Ctx) -> Option<T> {
    let range = node.depth_mask.as_ref().filter(|m| m.enabled);
    let bias = if node.is_group { "off".to_string() } else { node.params.choice_or("depthBias", "off") };
    let dir = match bias.as_str() {
        "near" => 1.0,
        "far" => -1.0,
        _ => 0.0,
    };
    if range.is_none() && dir == 0.0 {
        return None;
    }
    let d = ctx.depth()?;
    let r = range.map(|m| m.uniforms()).unwrap_or([0.0; 5]);
    let amount = (node.params.num("depthBiasAmount", 80.0) / 100.0) as f32;
    Some(ctx.apply("k_depthmask", &[&d], &[r[0], r[1], r[2], r[3], r[4], dir, amount]))
}
