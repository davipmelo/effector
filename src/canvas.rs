//! Canvas & Borders: places the processed image on a canvas of another aspect ratio.

use crate::gpu::{Ctx, T};
use crate::model::CanvasSpec;

pub fn apply(image: &T, spec: &CanvasSpec, ctx: &mut Ctx) -> T {
    let ((cw, ch), rect) = spec.layout(image.width() as f64, image.height() as f64);
    let (w, h) = ((cw as usize).max(1), (ch as usize).max(1));
    let mut bg = image.clone();
    if spec.fill == "blur" {
        let f = 6;
        let small = ctx.apply_sized("k_cover", &[image], &[], None, (w / f).max(1), (h / f).max(1));
        bg = ctx.blur(&small, (spec.blur / 100.0 * w.max(h) as f64 / f as f64 * 0.04).max(0.5));
    }
    let fill: f32 = match spec.fill.as_str() { "blur" => 1.0, "gradient" => 2.0, _ => 0.0 };
    let radius = (spec.radius / 100.0 * rect.2.min(rect.3) * 0.5) as f32;
    let shadow_size = (w.max(h) as f64 * 0.025) as f32;
    let mut u = vec![rect.0 as f32, rect.1 as f32, rect.2 as f32, rect.3 as f32, radius, fill];
    u.extend(spec.color.floats());
    u.extend(spec.color2.floats());
    u.extend([(spec.shadow / 100.0) as f32, shadow_size, if spec.fill == "blur" { 0.12 } else { 0.0 }]);
    ctx.apply_sized("k_canvas", &[image, &bg], &u, None, w, h)
}
