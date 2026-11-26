//! Masks on the canvas (Lightroom Classic style): drag to draw a linear or radial gradient, paint
//! with the brush, and move / reshape the selected gradient with its pins and handles.
//!
//! Mask geometry lives in image coordinates (0..1 of the whole, uncropped photo); lengths are in
//! units of the photo's long side ("long units"), so masks follow crops and any resolution.

use egui::{pos2, vec2, Color32, Id, Pos2, Rect, Shape, Stroke, Ui};
use uuid::Uuid;

use crate::app::{AppState, MaskDraw, Tool};
use crate::model::*;

/// Screen ↔ image mapping of the photo on the canvas.
#[derive(Clone, Copy)]
pub struct Geo {
    pub photo: Rect,
    crop: CropRect,
    /// image width and height / long side
    ax: f64,
    ay: f64,
}

impl Geo {
    pub fn new(app: &AppState, img_rect: Rect) -> Option<Geo> {
        let src = app.source()?;
        let st = app.state();
        let long = src.width.max(src.height).max(1) as f64;
        Some(Geo { photo: photo_rect(app, img_rect), crop: st.crop, ax: src.width as f64 / long, ay: src.height as f64 / long })
    }
    /// Screen point → image coordinates.
    pub fn to_img(&self, p: Pos2) -> [f64; 2] {
        let u = ((p.x - self.photo.left()) / self.photo.width()) as f64;
        let v = ((p.y - self.photo.top()) / self.photo.height()) as f64;
        [self.crop.x + u * self.crop.w, self.crop.y + v * self.crop.h]
    }
    pub fn to_screen(&self, q: [f64; 2]) -> Pos2 {
        pos2(
            self.photo.left() + ((q[0] - self.crop.x) / self.crop.w) as f32 * self.photo.width(),
            self.photo.top() + ((q[1] - self.crop.y) / self.crop.h) as f32 * self.photo.height(),
        )
    }
    /// Screen pixels per long unit.
    pub fn ppu(&self) -> f32 { self.photo.width() / (self.crop.w * self.ax).max(1e-9) as f32 }
    /// Difference of two image points in long units.
    fn long(&self, a: [f64; 2], b: [f64; 2]) -> (f64, f64) { ((b[0] - a[0]) * self.ax, (b[1] - a[1]) * self.ay) }
    /// Image point + offset in long units.
    fn offset(&self, a: [f64; 2], dx: f64, dy: f64) -> [f64; 2] { [a[0] + dx / self.ax, a[1] + dy / self.ay] }
}

/// The photo inside the canvas borders, on screen.
pub fn photo_rect(app: &AppState, img_rect: Rect) -> Rect {
    let Some(src) = app.source() else { return img_rect };
    let st = app.state();
    match st.active_canvas() {
        Some(c) => {
            let (cw, ch) = src.cropped_size(&st.crop);
            let ((ow, oh), (ix, iy, iw, ih)) = c.layout(cw as f64, ch as f64);
            let (sx, sy) = (img_rect.width() as f64 / ow, img_rect.height() as f64 / oh);
            Rect::from_min_size(pos2(img_rect.left() + (ix * sx) as f32, img_rect.top() + (iy * sy) as f32), vec2((iw * sx) as f32, (ih * sy) as f32))
        }
        None => img_rect,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    /// drawing a new gradient from `start`
    Create { mask: Uuid, part: Uuid, start: [f64; 2] },
    Paint { mask: Uuid, part: Uuid },
    /// ⌘ (Mac) / Alt (Windows) + right-drag: brush size (sideways) and feather (up / down)
    BrushResize { anchor: Pos2, size: f32, feather: f32 },
    Move { mask: Uuid, part: Uuid, grab: [f64; 2], orig: [f64; 2] },
    /// one side line of a linear gradient (`far` = the fading side); the other side stays put
    LinearSide { mask: Uuid, part: Uuid, far: bool },
    /// radial handle: along its own x axis (true) or y axis
    RadialEdge { mask: Uuid, part: Uuid, x_axis: bool },
    /// the dashed inner ellipse of a radial gradient: its feather
    RadialFeather { mask: Uuid, part: Uuid },
    /// the ring around the pin: rotation
    Rotate { mask: Uuid, part: Uuid, grab: f64, orig: f64 },
}

impl Op {
    fn part(&self) -> Option<Uuid> {
        match *self {
            Op::Move { part, .. } | Op::LinearSide { part, .. } | Op::RadialEdge { part, .. } | Op::RadialFeather { part, .. } | Op::Rotate { part, .. } => Some(part),
            _ => None,
        }
    }
    /// Same handle (ignores the drag bookkeeping).
    fn same_target(&self, o: &Op) -> bool { std::mem::discriminant(self) == std::mem::discriminant(o) && self.part() == o.part() && self.part().is_some() && self.side() == o.side() }
    fn side(&self) -> i32 {
        match *self {
            Op::LinearSide { far, .. } => far as i32,
            Op::RadialEdge { x_axis, .. } => x_axis as i32,
            _ => 0,
        }
    }
}

const HIT: f32 = 9.0;
/// The rotation ring around the pin: from the pin's edge out to this distance (screen px).
const RING: f32 = 26.0;

fn op_id() -> Id { Id::new("mask-canvas-op") }

fn accent() -> Color32 { Color32::from_rgb(255, 196, 0) }

/// ⌘ on the Mac, Alt on Windows (the brush resize modifier, with the right button).
fn resize_modifier(m: &egui::Modifiers) -> bool { if cfg!(target_os = "macos") { m.mac_cmd } else { m.alt } }

fn snap(deg: f64, step: f64) -> f64 { (deg / step).round() * step }

/// Handles everything mask-related on the canvas. Returns true when it uses the pointer (the
/// canvas must not pan then).
pub fn interact(ui: &mut Ui, app: &mut AppState, resp: &egui::Response, img_rect: Rect) -> bool {
    if app.tool != Tool::Masks {
        return false;
    }
    let Some(g) = Geo::new(app, img_rect) else { return false };
    let painter = ui.painter().with_clip_rect(resp.rect);
    let mut op: Option<Op> = ui.data(|d| d.get_temp(op_id()));
    let pointer = ui.input(|i| i.pointer.hover_pos());
    let (shift, alt, mods) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt, i.modifiers));
    let st = app.state();
    let mask = app.selected_mask.and_then(|m| st.mask(m).cloned());

    // ---- what is under the pointer (or being dragged), then draw the handles
    let hover = if app.mask_draw.is_none() { op.filter(|o| o.part().is_some()).or_else(|| pointer.and_then(|p| hit_test(&g, mask.as_ref(), app.selected_part, p))) } else { None };
    if app.mask_draw.is_none() {
        if let Some(m) = &mask {
            for p in &m.parts {
                draw_part(&painter, &g, p, Some(p.id) == app.selected_part, hover.as_ref());
            }
        }
    }

    // ---- start of a drag
    if resp.drag_started_by(egui::PointerButton::Secondary) && op.is_none() && app.mask_draw == Some(MaskDraw::Brush) && resize_modifier(&mods) {
        if let Some(o) = ui.input(|i| i.pointer.press_origin()).or(pointer) {
            op = Some(Op::BrushResize { anchor: o, size: app.brush.size, feather: app.brush.feather });
        }
    } else if resp.drag_started_by(egui::PointerButton::Primary) && op.is_none() {
        let origin = ui.input(|i| i.pointer.press_origin()).or(pointer);
        if let Some(o) = origin {
            let q = g.to_img(o);
            match app.mask_draw {
                Some(MaskDraw::Linear) => {
                    let into = app.mask_draw_into.take();
                    let (m, p) = app.add_mask_part(MaskShape::Linear { x: q[0], y: q[1], angle: 90.0, width: 0.0005 }, into);
                    op = Some(Op::Create { mask: m, part: p, start: q });
                }
                Some(MaskDraw::Radial) => {
                    let into = app.mask_draw_into.take();
                    let (m, p) = app.add_mask_part(MaskShape::Radial { x: q[0], y: q[1], rx: 0.0005, ry: 0.0005, angle: 0.0, feather: 0.5 }, into);
                    op = Some(Op::Create { mask: m, part: p, start: q });
                }
                Some(MaskDraw::Brush) => {
                    op = Some(begin_stroke(app, q, alt));
                }
                None => {
                    if let Some(h) = hit_test(&g, mask.as_ref(), app.selected_part, o) {
                        op = Some(match h {
                            Op::Move { mask, part, .. } => {
                                let orig = part_center(app, mask, part).unwrap_or(q);
                                Op::Move { mask, part, grab: q, orig }
                            }
                            Op::Rotate { mask, part, .. } => {
                                let c = part_center(app, mask, part).unwrap_or(q);
                                let (dx, dy) = g.long(c, q);
                                Op::Rotate { mask, part, grab: dy.atan2(dx).to_degrees(), orig: part_angle(app, mask, part) }
                            }
                            other => other,
                        });
                        if let Some(p) = op.and_then(|o| o.part()) {
                            app.selected_part = Some(p);
                        }
                    }
                }
            }
        }
    } else if resp.clicked() && app.mask_draw == Some(MaskDraw::Brush) {
        // a click paints one dab
        if let Some(pp) = pointer {
            begin_stroke(app, g.to_img(pp), alt);
        }
    } else if resp.clicked() && app.mask_draw.is_none() {
        if let Some(p) = hover.and_then(|h| h.part()) {
            app.selected_part = Some(p);
        }
    }

    // ---- dragging (Shift constrains every drag)
    if let (Some(o), Some(pp)) = (op, pointer) {
        let q = g.to_img(pp);
        let key = |p: Uuid| Some(format!("mask-canvas-{p}"));
        match o {
            Op::Create { mask, part, start } => {
                let (dx, dy) = g.long(start, q);
                app.update_part(mask, part, key(part), |p| match &mut p.shape {
                    MaskShape::Linear { x, y, angle, width } => {
                        let mut a = dy.atan2(dx).to_degrees();
                        let mut len = (dx * dx + dy * dy).sqrt();
                        if shift {
                            // straight: the direction snaps to 45° steps
                            let sa = snap(a, 45.0);
                            len *= (a - sa).to_radians().cos().max(0.0);
                            a = sa;
                        }
                        let r = a.to_radians();
                        let c = g.offset(start, r.cos() * len / 2.0, r.sin() * len / 2.0);
                        (*x, *y, *angle, *width) = (c[0], c[1], a, len.max(0.0005));
                    }
                    MaskShape::Radial { rx, ry, .. } => {
                        let (a, b) = (dx.abs().max(0.0005), dy.abs().max(0.0005));
                        (*rx, *ry) = if shift { (a.max(b), a.max(b)) } else { (a, b) };
                    }
                    _ => {}
                });
            }
            Op::Paint { mask, part } => {
                let min_step = 1.5 / g.ppu() as f64;
                app.update_part(mask, part, key(part), |p| {
                    if let MaskShape::Brush { strokes } = &mut p.shape {
                        if let Some(s) = strokes.last_mut() {
                            let last = s.points.last().copied().unwrap_or([q[0] as f32, q[1] as f32]);
                            let (dx, dy) = g.long([last[0] as f64, last[1] as f64], q);
                            if (dx * dx + dy * dy).sqrt() >= min_step {
                                s.points.push([q[0] as f32, q[1] as f32]);
                            }
                        }
                    }
                });
            }
            Op::BrushResize { anchor, size, feather } => {
                let d = pp - anchor;
                app.brush.size = (size + d.x / g.ppu()).clamp(0.002, 0.5);
                app.brush.feather = (feather - d.y / 200.0).clamp(0.0, 1.0);
            }
            Op::Move { mask, part, grab, orig } => {
                let (mut mx, mut my) = (q[0] - grab[0], q[1] - grab[1]);
                if shift {
                    // only along the main axis of the move
                    let (lx, ly) = g.long(grab, q);
                    if lx.abs() >= ly.abs() { my = 0.0 } else { mx = 0.0 }
                }
                let (nx, ny) = (orig[0] + mx, orig[1] + my);
                app.update_part(mask, part, key(part), |p| match &mut p.shape {
                    MaskShape::Linear { x, y, .. } | MaskShape::Radial { x, y, .. } => {
                        (*x, *y) = (nx, ny);
                    }
                    _ => {}
                });
            }
            Op::LinearSide { mask, part, far } => {
                app.update_part(mask, part, key(part), |p| {
                    if let MaskShape::Linear { x, y, angle, width } = &mut p.shape {
                        let r = angle.to_radians();
                        let (ux, uy) = (r.cos(), r.sin());
                        let (dx, dy) = g.long([*x, *y], q);
                        let t = dx * ux + dy * uy;
                        // positions of both sides along the gradient; only the dragged one moves
                        let (mut near, mut fs) = (-*width / 2.0, *width / 2.0);
                        if far { fs = t.max(near + 0.002) } else { near = t.min(fs - 0.002) }
                        let mid = (near + fs) / 2.0;
                        let c = g.offset([*x, *y], ux * mid, uy * mid);
                        (*x, *y, *width) = (c[0], c[1], fs - near);
                    }
                });
            }
            Op::RadialEdge { mask, part, x_axis } => {
                app.update_part(mask, part, key(part), |p| {
                    if let MaskShape::Radial { x, y, rx, ry, angle, .. } = &mut p.shape {
                        let (dx, dy) = g.long([*x, *y], q);
                        let a = angle.to_radians();
                        let (ux, uy) = (a.cos(), a.sin());
                        let along = if x_axis { dx * ux + dy * uy } else { -dx * uy + dy * ux };
                        let r = along.abs().max(0.002);
                        if shift {
                            (*rx, *ry) = (r, r);
                        } else if x_axis {
                            *rx = r;
                        } else {
                            *ry = r;
                        }
                    }
                });
            }
            Op::RadialFeather { mask, part } => {
                app.update_part(mask, part, key(part), |p| {
                    if let MaskShape::Radial { x, y, rx, ry, angle, feather } = &mut p.shape {
                        let rn = radial_norm(&g, [*x, *y], *rx, *ry, *angle, q);
                        let mut f = (1.0 - rn).clamp(0.0, 1.0);
                        if shift {
                            f = (f * 10.0).round() / 10.0;
                        }
                        *feather = f;
                    }
                });
            }
            Op::Rotate { mask, part, grab, orig } => {
                let c = part_center(app, mask, part).unwrap_or(q);
                let (dx, dy) = g.long(c, q);
                let mut a = orig + dy.atan2(dx).to_degrees() - grab;
                if shift {
                    a = snap(a, 15.0);
                }
                let a = (a + 540.0).rem_euclid(360.0) - 180.0;
                app.update_part(mask, part, key(part), |p| match &mut p.shape {
                    MaskShape::Linear { angle, .. } | MaskShape::Radial { angle, .. } => *angle = a,
                    _ => {}
                });
            }
        }
    }

    // ---- end of a drag
    if resp.drag_stopped() {
        if let Some(Op::Create { mask, part, start }) = op {
            // a click without a drag: a sensible default size around the point
            let tiny = app.state().mask(mask).and_then(|m| m.parts.iter().find(|p| p.id == part)).map(|p| match p.shape {
                MaskShape::Linear { width, .. } => width < 0.01,
                MaskShape::Radial { rx, ry, .. } => rx.max(ry) < 0.01,
                _ => false,
            });
            if tiny == Some(true) {
                app.update_part(mask, part, None, |p| match &mut p.shape {
                    MaskShape::Linear { x, y, angle, width } => {
                        (*x, *y) = (start[0], start[1]);
                        *angle = 90.0;
                        *width = 0.3;
                    }
                    MaskShape::Radial { rx, ry, .. } => {
                        (*rx, *ry) = (0.2, 0.15);
                    }
                    _ => {}
                });
            }
            app.mask_draw = None;
        }
        op = None;
    }

    // ---- cursors
    let over = pointer.map(|p| resp.rect.contains(p)).unwrap_or(false);
    let mut busy = op.is_some();
    match app.mask_draw {
        Some(MaskDraw::Brush) if over || matches!(op, Some(Op::BrushResize { .. })) => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::None);
            // while resizing, the circle stays where the drag started
            let at = match op {
                Some(Op::BrushResize { anchor, .. }) => Some(anchor),
                _ => pointer,
            };
            if let Some(p) = at {
                let r = app.brush.size * g.ppu();
                let erase = app.brush_erase ^ alt;
                let c = if erase { Color32::from_rgb(255, 120, 120) } else { Color32::WHITE };
                painter.circle_stroke(p, r, Stroke::new(1.5, Color32::from_black_alpha(160)));
                painter.circle_stroke(p, r, Stroke::new(1.0, c));
                let inner = r * (1.0 - app.brush.feather).max(0.0);
                if inner > 2.0 {
                    painter.circle_stroke(p, inner, Stroke::new(1.0, with_alpha(c, 0.55)));
                }
                painter.line_segment([p - vec2(4.0, 0.0), p + vec2(4.0, 0.0)], Stroke::new(1.2, c));
                if !erase {
                    painter.line_segment([p - vec2(0.0, 4.0), p + vec2(0.0, 4.0)], Stroke::new(1.2, c));
                }
            }
            busy = true;
        }
        Some(_) if over => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            busy = true;
        }
        _ => {
            if let Some(h) = hover {
                busy = true;
                match h {
                    Op::Rotate { .. } => {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                        if let Some(p) = pointer {
                            rotate_cursor(&painter, p);
                        }
                    }
                    Op::LinearSide { .. } | Op::RadialEdge { .. } | Op::RadialFeather { .. } => {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
                    }
                    _ => ui.ctx().set_cursor_icon(if op.is_some() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab }),
                }
            }
        }
    }
    ui.data_mut(|d| match op {
        Some(o) => {
            d.insert_temp(op_id(), o);
        }
        None => d.remove::<Op>(op_id()),
    });
    if busy {
        ui.ctx().request_repaint();
    }
    busy
}

/// A small circular arrow (rotation cursor).
fn rotate_cursor(painter: &egui::Painter, p: Pos2) {
    let pts: Vec<Pos2> = (0..=20).map(|i| {
        let t = (-200.0 + i as f32 * 11.0).to_radians();
        p + vec2(t.cos(), t.sin()) * 8.0
    }).collect();
    let end = *pts.last().unwrap();
    for (w, c) in [(3.2, Color32::from_black_alpha(170)), (1.6, Color32::WHITE)] {
        painter.add(Shape::line(pts.clone(), Stroke::new(w, c)));
        painter.line_segment([end, end + vec2(-4.0, -1.0)], Stroke::new(w, c));
        painter.line_segment([end, end + vec2(0.5, -4.0)], Stroke::new(w, c));
    }
}

/// Starts a brush stroke (into the selected brush part, the target mask, or a new mask).
fn begin_stroke(app: &mut AppState, q: [f64; 2], alt: bool) -> Op {
    let st = app.state();
    let current = app
        .selected_mask
        .and_then(|m| st.mask(m).map(|mm| (m, mm.clone())))
        .and_then(|(m, mm)| mm.parts.iter().find(|p| Some(p.id) == app.selected_part && matches!(p.shape, MaskShape::Brush { .. })).map(|p| (m, p.id)));
    let (m, p) = match (current, app.mask_draw_into) {
        (Some(c), None) => c,
        _ => {
            let into = app.mask_draw_into.take();
            app.add_mask_part(MaskShape::Brush { strokes: vec![] }, into)
        }
    };
    let b = app.brush;
    let stroke = BrushStroke { erase: app.brush_erase ^ alt, size: b.size, feather: b.feather, flow: b.flow, points: vec![[q[0] as f32, q[1] as f32]] };
    let n = app.state().mask(m).and_then(|mm| mm.parts.iter().find(|x| x.id == p)).map(|x| if let MaskShape::Brush { strokes } = &x.shape { strokes.len() } else { 0 }).unwrap_or(0);
    app.update_part(m, p, Some(format!("stroke-{p}-{n}")), |x| {
        if let MaskShape::Brush { strokes } = &mut x.shape {
            strokes.push(stroke);
        }
    });
    Op::Paint { mask: m, part: p }
}

fn part_shape(app: &AppState, mask: Uuid, part: Uuid) -> Option<MaskShape> {
    app.state().mask(mask)?.parts.iter().find(|p| p.id == part).map(|p| p.shape.clone())
}

fn part_center(app: &AppState, mask: Uuid, part: Uuid) -> Option<[f64; 2]> {
    match part_shape(app, mask, part)? {
        MaskShape::Linear { x, y, .. } | MaskShape::Radial { x, y, .. } => Some([x, y]),
        _ => None,
    }
}

fn part_angle(app: &AppState, mask: Uuid, part: Uuid) -> f64 {
    match part_shape(app, mask, part) {
        Some(MaskShape::Linear { angle, .. }) | Some(MaskShape::Radial { angle, .. }) => angle,
        _ => 0.0,
    }
}

/// Elliptical radius of `q` (1 = on the ellipse) for a radial gradient.
fn radial_norm(g: &Geo, c: [f64; 2], rx: f64, ry: f64, angle: f64, q: [f64; 2]) -> f64 {
    let (dx, dy) = g.long(c, q);
    let a = angle.to_radians();
    let (u, v) = (dx * a.cos() + dy * a.sin(), -dx * a.sin() + dy * a.cos());
    ((u / rx.max(1e-6)).powi(2) + (v / ry.max(1e-6)).powi(2)).sqrt()
}

/// Screen geometry of a gradient part: centre and the linear side points or radial edge handles.
fn handle_points(g: &Geo, shape: &MaskShape) -> Option<(Pos2, Vec<(Pos2, Op)>)> {
    let nil = Uuid::nil();
    match *shape {
        MaskShape::Linear { x, y, angle, width } => {
            let a = angle.to_radians();
            let (dx, dy) = (a.cos() * width / 2.0, a.sin() * width / 2.0);
            let c = g.to_screen([x, y]);
            let near = g.to_screen(g.offset([x, y], -dx, -dy));
            let far = g.to_screen(g.offset([x, y], dx, dy));
            Some((c, vec![(near, Op::LinearSide { mask: nil, part: nil, far: false }), (far, Op::LinearSide { mask: nil, part: nil, far: true })]))
        }
        MaskShape::Radial { x, y, rx, ry, angle, .. } => {
            let a = angle.to_radians();
            let (ux, uy) = (a.cos(), a.sin());
            let c = g.to_screen([x, y]);
            let pts = vec![
                (g.to_screen(g.offset([x, y], ux * rx, uy * rx)), Op::RadialEdge { mask: nil, part: nil, x_axis: true }),
                (g.to_screen(g.offset([x, y], -ux * rx, -uy * rx)), Op::RadialEdge { mask: nil, part: nil, x_axis: true }),
                (g.to_screen(g.offset([x, y], -uy * ry, ux * ry)), Op::RadialEdge { mask: nil, part: nil, x_axis: false }),
                (g.to_screen(g.offset([x, y], uy * ry, -ux * ry)), Op::RadialEdge { mask: nil, part: nil, x_axis: false }),
            ];
            Some((c, pts))
        }
        _ => None,
    }
}

fn with_ids(o: Op, mask: Uuid, part: Uuid) -> Op {
    match o {
        Op::LinearSide { far, .. } => Op::LinearSide { mask, part, far },
        Op::RadialEdge { x_axis, .. } => Op::RadialEdge { mask, part, x_axis },
        other => other,
    }
}

/// Distance from `p` to the infinite line through `a` with direction `d` (unit).
fn line_dist(p: Pos2, a: Pos2, d: egui::Vec2) -> f32 { ((p - a).x * d.y - (p - a).y * d.x).abs() }

/// What is under `p`, by priority: handles of the selected part, any pin (move), the rotation
/// ring, the dashed feather ellipse / the side and middle lines of the selected part.
fn hit_test(g: &Geo, mask: Option<&Mask>, selected: Option<Uuid>, p: Pos2) -> Option<Op> {
    let m = mask?;
    let sel = m.parts.iter().find(|x| Some(x.id) == selected);
    if let Some(sp) = sel {
        if let Some((_, hs)) = handle_points(g, &sp.shape) {
            if matches!(sp.shape, MaskShape::Radial { .. }) {
                for (hp, o) in &hs {
                    if hp.distance(p) <= HIT {
                        return Some(with_ids(*o, m.id, sp.id));
                    }
                }
            }
        }
    }
    let mv = |part: Uuid| Op::Move { mask: m.id, part, grab: [0.0; 2], orig: [0.0; 2] };
    for part in &m.parts {
        if let Some((c, _)) = handle_points(g, &part.shape) {
            if c.distance(p) <= HIT {
                return Some(mv(part.id));
            }
        }
    }
    let sp = sel?;
    let (c, hs) = handle_points(g, &sp.shape)?;
    if c.distance(p) <= RING {
        return Some(Op::Rotate { mask: m.id, part: sp.id, grab: 0.0, orig: 0.0 });
    }
    match sp.shape {
        MaskShape::Radial { x, y, rx, ry, angle, feather } => {
            // the dashed inner ellipse, within a few screen pixels
            let rn = radial_norm(g, [x, y], rx, ry, angle, g.to_img(p));
            let px = (rn - (1.0 - feather)).abs() * rx.min(ry) * g.ppu() as f64;
            if px <= HIT as f64 {
                return Some(Op::RadialFeather { mask: m.id, part: sp.id });
            }
        }
        MaskShape::Linear { angle, .. } => {
            let a = angle.to_radians() as f32;
            let perp = vec2(-a.sin(), a.cos());
            for (hp, o) in &hs {
                if line_dist(p, *hp, perp) <= HIT {
                    return Some(with_ids(*o, m.id, sp.id));
                }
            }
            if line_dist(p, c, perp) <= HIT {
                return Some(mv(sp.id));
            }
        }
        _ => {}
    }
    None
}

/// Draws a part's overlay: lines / ellipses, the pin and handles, highlighting `hover`.
fn draw_part(painter: &egui::Painter, g: &Geo, part: &MaskPart, selected: bool, hover: Option<&Op>) {
    let Some((c, hs)) = handle_points(g, &part.shape) else { return };
    let shadow = Stroke::new(2.6, Color32::from_black_alpha(90));
    let is = |o: &Op| hover.map(|h| h.same_target(&with_ids(*o, Uuid::nil(), part.id)) || (h.part() == Some(part.id) && std::mem::discriminant(h) == std::mem::discriminant(o) && matches!(o, Op::RadialFeather { .. } | Op::Rotate { .. } | Op::Move { .. }))).unwrap_or(false);
    let thin = |hot: bool, a: f32| Stroke::new(if hot { 2.2 } else { 1.2 }, with_alpha(if hot { accent() } else { Color32::WHITE }, if hot { 1.0 } else { a }));
    if selected {
        match part.shape {
            MaskShape::Linear { angle, .. } => {
                // three lines across the photo, perpendicular to the gradient direction
                let a = angle.to_radians() as f32;
                let perp = vec2(-a.sin(), a.cos());
                let span = g.photo.width().max(g.photo.height()) * 2.0;
                for (i, (pt, o)) in hs.iter().enumerate() {
                    let seg = [*pt - perp * span, *pt + perp * span];
                    painter.line_segment(seg, shadow);
                    painter.line_segment(seg, thin(is(o), if i == 0 { 0.9 } else { 0.6 }));
                }
                let mid = [c - perp * span, c + perp * span];
                let hot = hover.map(|h| matches!(h, Op::Move { part: p, .. } if *p == part.id)).unwrap_or(false);
                painter.add(Shape::dashed_line(&mid, thin(hot, 0.55), 6.0, 5.0));
            }
            MaskShape::Radial { x, y, rx, ry, angle, feather } => {
                let ell = |k: f64| -> Vec<Pos2> {
                    let a = angle.to_radians();
                    (0..=72)
                        .map(|i| {
                            let t = i as f64 / 72.0 * std::f64::consts::TAU;
                            let (ex, ey) = (t.cos() * rx * k, t.sin() * ry * k);
                            g.to_screen(g.offset([x, y], ex * a.cos() - ey * a.sin(), ex * a.sin() + ey * a.cos()))
                        })
                        .collect()
                };
                let outer = ell(1.0);
                painter.add(Shape::line(outer.clone(), shadow));
                painter.add(Shape::line(outer, thin(false, 0.9)));
                let inner = ell((1.0 - feather).max(0.02));
                let hot = is(&Op::RadialFeather { mask: Uuid::nil(), part: part.id });
                painter.add(Shape::dashed_line(&inner, thin(hot, 0.55), 5.0, 4.0));
                for (hp, o) in &hs {
                    let hov = is(o);
                    painter.circle_filled(*hp, if hov { 5.5 } else { 4.5 }, if hov { accent() } else { Color32::WHITE });
                    painter.circle_stroke(*hp, if hov { 5.5 } else { 4.5 }, Stroke::new(1.0, Color32::from_black_alpha(150)));
                }
            }
            _ => {}
        }
        // rotation ring, shown while hovering it or rotating
        if hover.map(|h| matches!(h, Op::Rotate { part: p, .. } if *p == part.id)).unwrap_or(false) {
            painter.circle_stroke(c, RING - 4.0, Stroke::new(2.0, with_alpha(accent(), 0.5)));
        }
    }
    // centre pin (Lightroom: filled when selected)
    let hot = hover.map(|h| matches!(h, Op::Move { part: p, .. } if *p == part.id)).unwrap_or(false);
    painter.circle_filled(c, 7.0, Color32::from_black_alpha(120));
    painter.circle_stroke(c, 6.5, Stroke::new(if hot { 2.2 } else { 1.5 }, Color32::WHITE));
    if selected {
        painter.circle_filled(c, 3.5, accent());
    }
}

fn with_alpha(c: Color32, a: f32) -> Color32 { Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a * 255.0) as u8) }
