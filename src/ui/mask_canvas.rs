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

#[derive(Clone, Copy, Debug)]
enum Op {
    /// drawing a new gradient from `start`
    Create { mask: Uuid, part: Uuid, start: [f64; 2] },
    Paint { mask: Uuid, part: Uuid },
    Move { mask: Uuid, part: Uuid, grab: [f64; 2], orig: [f64; 2] },
    /// linear end line (`far` = the fading side)
    LinearEnd { mask: Uuid, part: Uuid, far: bool },
    /// radial handle: along its own x axis (true) or y axis
    RadialEdge { mask: Uuid, part: Uuid, x_axis: bool },
}

const HIT: f32 = 9.0;

fn op_id() -> Id { Id::new("mask-canvas-op") }

fn accent() -> Color32 { Color32::from_rgb(255, 196, 0) }

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
    let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));
    let st = app.state();
    let mask = app.selected_mask.and_then(|m| st.mask(m).cloned());
    let part = mask.as_ref().and_then(|m| m.parts.iter().find(|p| Some(p.id) == app.selected_part).cloned());

    // ---- draw the selected part's handles (and pins of the other gradients of the mask)
    let mut hit: Option<Op> = None;
    if app.mask_draw.is_none() {
        if let Some(m) = &mask {
            for p in &m.parts {
                let sel = Some(p.id) == app.selected_part;
                if let Some(h) = draw_handles(&painter, &g, m.id, p, sel, pointer) {
                    if hit.is_none() {
                        hit = Some(h);
                    }
                }
            }
        }
    }

    // ---- start of a drag
    if resp.drag_started_by(egui::PointerButton::Primary) && op.is_none() {
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
                    // a handle under the press point
                    if let Some(h) = origin_hit(&g, mask.as_ref(), part.as_ref(), o) {
                        op = Some(match h {
                            Op::Move { mask, part, .. } => {
                                let orig = mask_center(app, mask, part).unwrap_or(q);
                                Op::Move { mask, part, grab: q, orig }
                            }
                            other => other,
                        });
                        if let Op::Move { part: p, .. } | Op::LinearEnd { part: p, .. } | Op::RadialEdge { part: p, .. } = op.unwrap() {
                            app.selected_part = Some(p);
                        }
                    }
                }
            }
        }
    } else if resp.clicked() && app.mask_draw == Some(MaskDraw::Brush) {
        // a click paints one dab
        if let Some(pp) = pointer {
            let o = begin_stroke(app, g.to_img(pp), alt);
            let _ = o;
        }
    } else if resp.clicked() && app.mask_draw.is_none() {
        if let Some(Op::Move { part, .. }) = hit {
            app.selected_part = Some(part);
        }
    }

    // ---- dragging
    if let (Some(o), Some(pp)) = (op, pointer) {
        let q = g.to_img(pp);
        let key = |p: Uuid| Some(format!("mask-canvas-{p}"));
        match o {
            Op::Create { mask, part, start } => {
                let (dx, dy) = g.long(start, q);
                app.update_part(mask, part, key(part), |p| match &mut p.shape {
                    MaskShape::Linear { x, y, angle, width } => {
                        let c = [(start[0] + q[0]) / 2.0, (start[1] + q[1]) / 2.0];
                        *x = c[0];
                        *y = c[1];
                        *angle = dy.atan2(dx).to_degrees();
                        *width = (dx * dx + dy * dy).sqrt().max(0.0005);
                    }
                    MaskShape::Radial { rx, ry, .. } => {
                        let (a, b) = (dx.abs().max(0.0005), dy.abs().max(0.0005));
                        if shift {
                            let r = a.max(b);
                            (*rx, *ry) = (r, r);
                        } else {
                            (*rx, *ry) = (a, b);
                        }
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
            Op::Move { mask, part, grab, orig } => {
                let (nx, ny) = (orig[0] + q[0] - grab[0], orig[1] + q[1] - grab[1]);
                app.update_part(mask, part, key(part), |p| match &mut p.shape {
                    MaskShape::Linear { x, y, .. } | MaskShape::Radial { x, y, .. } => {
                        *x = nx;
                        *y = ny;
                    }
                    _ => {}
                });
            }
            Op::LinearEnd { mask, part, far } => {
                app.update_part(mask, part, key(part), |p| {
                    if let MaskShape::Linear { x, y, angle, width } = &mut p.shape {
                        let (mut dx, mut dy) = g.long([*x, *y], q);
                        if !far {
                            dx = -dx;
                            dy = -dy;
                        }
                        *angle = dy.atan2(dx).to_degrees();
                        *width = ((dx * dx + dy * dy).sqrt() * 2.0).max(0.002);
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
        Some(MaskDraw::Brush) if over => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::None);
            if let Some(p) = pointer {
                let r = app.brush.size * g.ppu();
                let erase = app.brush_erase ^ alt;
                let c = if erase { Color32::from_rgb(255, 120, 120) } else { Color32::WHITE };
                painter.circle_stroke(p, r, Stroke::new(1.5, Color32::from_black_alpha(160)));
                painter.circle_stroke(p, r, Stroke::new(1.0, c));
                let inner = r * (1.0 - app.brush.feather).max(0.0);
                if inner > 2.0 {
                    painter.circle_stroke(p, inner, Stroke::new(1.0, with_alpha(c, 0.55)));
                }
                if erase {
                    painter.line_segment([p - vec2(4.0, 0.0), p + vec2(4.0, 0.0)], Stroke::new(1.2, c));
                } else {
                    painter.line_segment([p - vec2(4.0, 0.0), p + vec2(4.0, 0.0)], Stroke::new(1.2, c));
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
            if hit.is_some() {
                ui.ctx().set_cursor_icon(if op.is_some() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
                busy = true;
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

fn mask_center(app: &AppState, mask: Uuid, part: Uuid) -> Option<[f64; 2]> {
    let st = app.state();
    let p = st.mask(mask)?.parts.iter().find(|p| p.id == part)?.clone();
    match p.shape {
        MaskShape::Linear { x, y, .. } | MaskShape::Radial { x, y, .. } => Some([x, y]),
        _ => None,
    }
}

/// Handle positions on screen: (centre, linear near / far ends or radial ±x / ±y handles).
fn handle_points(g: &Geo, shape: &MaskShape) -> Option<(Pos2, Vec<(Pos2, Op)>)> {
    let nil = Uuid::nil();
    match *shape {
        MaskShape::Linear { x, y, angle, width } => {
            let a = angle.to_radians();
            let (dx, dy) = (a.cos() * width / 2.0, a.sin() * width / 2.0);
            let c = g.to_screen([x, y]);
            let near = g.to_screen(g.offset([x, y], -dx, -dy));
            let far = g.to_screen(g.offset([x, y], dx, dy));
            Some((c, vec![(near, Op::LinearEnd { mask: nil, part: nil, far: false }), (far, Op::LinearEnd { mask: nil, part: nil, far: true })]))
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
        Op::LinearEnd { far, .. } => Op::LinearEnd { mask, part, far },
        Op::RadialEdge { x_axis, .. } => Op::RadialEdge { mask, part, x_axis },
        other => other,
    }
}

/// What is under `p`: a handle of the selected part, or any gradient pin of the selected mask.
fn origin_hit(g: &Geo, mask: Option<&Mask>, sel: Option<&MaskPart>, p: Pos2) -> Option<Op> {
    let m = mask?;
    if let Some(sp) = sel {
        if let Some((c, hs)) = handle_points(g, &sp.shape) {
            for (hp, o) in hs {
                if hp.distance(p) <= HIT {
                    return Some(with_ids(o, m.id, sp.id));
                }
            }
            if c.distance(p) <= HIT + 2.0 {
                return Some(Op::Move { mask: m.id, part: sp.id, grab: [0.0; 2], orig: [0.0; 2] });
            }
        }
    }
    for part in &m.parts {
        if let Some((c, _)) = handle_points(g, &part.shape) {
            if c.distance(p) <= HIT + 2.0 {
                return Some(Op::Move { mask: m.id, part: part.id, grab: [0.0; 2], orig: [0.0; 2] });
            }
        }
    }
    None
}

/// Draws a part's overlay (lines / ellipse, pins, handles). Returns what the pointer is over.
fn draw_handles(painter: &egui::Painter, g: &Geo, mask: Uuid, part: &MaskPart, selected: bool, pointer: Option<Pos2>) -> Option<Op> {
    let (c, hs) = handle_points(g, &part.shape)?;
    let line = Stroke::new(1.2, Color32::from_white_alpha(if selected { 230 } else { 120 }));
    let shadow = Stroke::new(2.6, Color32::from_black_alpha(90));
    let mut over = None;
    if selected {
        match part.shape {
            MaskShape::Linear { angle, .. } => {
                // three lines across the photo, perpendicular to the gradient direction
                let a = angle.to_radians() as f32;
                let perp = vec2(-a.sin(), a.cos());
                let span = g.photo.width().max(g.photo.height()) * 2.0;
                for (i, (pt, _)) in hs.iter().enumerate() {
                    let seg = [*pt - perp * span, *pt + perp * span];
                    painter.line_segment(seg, shadow);
                    painter.line_segment(seg, if i == 0 { line } else { Stroke::new(1.0, with_alpha(Color32::WHITE, 0.6)) });
                }
                let mid = [c - perp * span, c + perp * span];
                painter.add(Shape::dashed_line(&mid, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.55)), 6.0, 5.0));
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
                painter.add(Shape::line(outer, line));
                let inner = ell((1.0 - feather).max(0.02));
                painter.add(Shape::dashed_line(&inner, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.55)), 5.0, 4.0));
            }
            _ => {}
        }
        for (hp, o) in &hs {
            let hov = pointer.map(|p| p.distance(*hp) <= HIT).unwrap_or(false);
            painter.circle_filled(*hp, if hov { 5.5 } else { 4.5 }, Color32::WHITE);
            painter.circle_stroke(*hp, if hov { 5.5 } else { 4.5 }, Stroke::new(1.0, Color32::from_black_alpha(150)));
            if hov && over.is_none() {
                over = Some(with_ids(*o, mask, part.id));
            }
        }
    }
    // centre pin (Lightroom: filled when selected)
    let hov = pointer.map(|p| p.distance(c) <= HIT + 2.0).unwrap_or(false);
    painter.circle_filled(c, 7.0, Color32::from_black_alpha(120));
    painter.circle_stroke(c, 6.5, Stroke::new(1.5, Color32::WHITE));
    if selected {
        painter.circle_filled(c, 3.5, accent());
    }
    if hov && over.is_none() {
        over = Some(Op::Move { mask, part: part.id, grab: [0.0; 2], orig: [0.0; 2] });
    }
    over
}

fn with_alpha(c: Color32, a: f32) -> Color32 { Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a * 255.0) as u8) }
