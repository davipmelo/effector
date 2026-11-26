//! Central canvas: rendered image with zoom/pan, before/after (toggle and split), crop overlay,
//! empty state, badges and toast messages.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};

use super::theme::*;
use super::widgets::*;
use crate::app::AppState;
use crate::l10n::tr;
use crate::model::CropRect;

pub struct CanvasTextures {
    pub output: Option<egui::TextureId>,
    pub original: Option<egui::TextureId>,
    pub crop: Option<egui::TextureId>,
    /// Red overlay of the selected mask (Masks tool).
    pub mask: Option<egui::TextureId>,
    pub checker: egui::TextureId,
    /// texture px per screen px of the current output (for nearest filtering when zoomed in).
    pub out_size: egui::Vec2,
}

pub const CROP_ASPECTS: [(&str, &str, Option<f64>); 16] = [
    ("free", "Free", None),
    ("original", "Original", Some(-1.0)),
    ("1:1", "1:1", Some(1.0)),
    ("4:5", "4:5", Some(4.0 / 5.0)),
    ("5:4", "5:4", Some(5.0 / 4.0)),
    ("3:2", "3:2", Some(3.0 / 2.0)),
    ("2:3", "2:3", Some(2.0 / 3.0)),
    ("4:3", "4:3", Some(4.0 / 3.0)),
    ("3:4", "3:4", Some(3.0 / 4.0)),
    ("16:9", "16:9", Some(16.0 / 9.0)),
    ("9:16", "9:16", Some(9.0 / 16.0)),
    ("1.85:1", "1.85:1 (Cinema)", Some(1.85)),
    ("2:1", "2:1", Some(2.0)),
    ("2.35:1", "2.35:1 (Scope)", Some(2.35)),
    ("2.7:1", "2.7:1 (XPan)", Some(2.7)),
    ("1:2.35", "1:2.35", Some(1.0 / 2.35)),
];

fn paint_image(ui: &Ui, tex: egui::TextureId, rect: Rect, clip: Rect, checker: egui::TextureId, checker_px: f32) {
    let p = ui.painter().with_clip_rect(clip);
    // checkerboard (8 pt cells) under transparent images
    let uv = Rect::from_min_max(pos2(rect.left() / checker_px, rect.top() / checker_px), pos2(rect.right() / checker_px, rect.bottom() / checker_px));
    p.image(checker, rect, uv, Color32::WHITE);
    p.image(tex, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}

pub fn canvas(ui: &mut Ui, app: &mut AppState, tex: &CanvasTextures) {
    let rect = ui.available_rect_before_wrap();
    let resp = ui.allocate_rect(rect, Sense::click_and_drag());
    let bg = if app.presenting { Color32::BLACK } else { backdrop_color() };
    ui.painter().rect_filled(rect, CornerRadius::ZERO, bg);
    // viewport bookkeeping (in points)
    let vs = rect.size();
    if (app.viewport - vs).length() > 0.5 {
        app.viewport = vs;
        app.update_preview_scale(false);
    }
    if !app.has_image() {
        empty_state(ui, app, rect);
        toast(ui, app, rect);
        return;
    }
    let Some(src) = app.source() else { return };
    // image rect
    let (w, h) = if app.is_cropping { (src.width, src.height) } else { app.output_size() };
    let (z, pan) = if app.is_cropping {
        (((vs.x - 80.0) as f64 / w as f64).min((vs.y - 130.0) as f64 / h as f64), egui::vec2(0.0, -25.0))
    } else {
        (app.display_zoom(), app.pan())
    };
    let size = vec2((w as f64 * z) as f32, (h as f64 * z) as f32);
    let img_rect = Rect::from_min_size(rect.center() - size / 2.0 + pan, size);
    let checker_px = 16.0;
    if app.is_cropping {
        if let Some(t) = tex.crop.or(tex.original) {
            paint_image(ui, t, img_rect, rect, tex.checker, checker_px);
        }
        crop_overlay(ui, app, rect, img_rect);
    } else {
        let main = if app.show_original { tex.original } else { tex.output };
        if let Some(t) = main {
            paint_image(ui, t, img_rect, rect, tex.checker, checker_px);
        }
        if let (Some(m), false) = (tex.mask, app.show_original) {
            super::tools::paint_mask_overlay(ui, app, m, img_rect, rect);
        }
        if app.compare_split && !app.show_original {
            if let Some(o) = tex.original {
                let x = rect.left() + rect.width() * app.split_pos as f32;
                let left_clip = Rect::from_min_max(rect.min, pos2(x, rect.bottom()));
                // the original covers the same area as the output (no canvas)
                let (ow, oh) = src.cropped_size(&app.state().crop);
                let osz = vec2((ow as f64 * z) as f32, (oh as f64 * z) as f32);
                let orect = Rect::from_center_size(img_rect.center(), osz);
                ui.painter().with_clip_rect(left_clip).rect_filled(img_rect, CornerRadius::ZERO, bg);
                paint_image(ui, o, orect, left_clip, tex.checker, checker_px);
            }
            split_divider(ui, app, rect);
        }
        // masks: draw gradients, paint, move handles (takes the pointer from panning)
        let mask_busy = !app.show_original && super::mask_canvas::interact(ui, app, &resp, img_rect);
        // right-click outside the image: background colour of the canvas (Lightroom-style)
        if !app.presenting {
            backdrop_menu(ui, &resp, img_rect);
        }
        // interaction: pan / zoom. The image can only be dragged along an axis where it is larger
        // than the view (Lightroom-style): at "fit" or any smaller zoom, dragging does nothing.
        let pannable = (size.x > vs.x + 0.5, size.y > vs.y + 0.5);
        let can_pan = pannable.0 || pannable.1;
        if resp.double_clicked() {
            app.set_zoom(None);
        } else if can_pan && ((resp.dragged_by(egui::PointerButton::Primary) && !mask_busy) || resp.dragged_by(egui::PointerButton::Middle)) {
            let p = app.pan() + resp.drag_delta();
            app.set_pan(p);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if can_pan && resp.hovered() && !mask_busy && ui.rect_contains_pointer(img_rect) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if resp.hovered() {
            let (scroll, zoom_delta, cmd, mouse) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers.command, i.pointer.hover_pos()));
            if let Some(m) = mouse {
                if zoom_delta != 1.0 {
                    zoom_at(app, rect, zoom_delta as f64, m);
                } else if scroll != egui::Vec2::ZERO {
                    if cmd || scroll.x == 0.0 && is_wheel(ui) {
                        zoom_at(app, rect, (scroll.y as f64 * 0.0025).exp(), m);
                    } else if can_pan {
                        let p = app.pan() + scroll;
                        app.set_pan(p);
                    }
                }
            }
        }
        // keep the image inside the view: no offset along an axis that fits, and never past the edges
        let (z2, pan2) = (app.display_zoom(), app.pan());
        let size2 = vec2((w as f64 * z2) as f32, (h as f64 * z2) as f32);
        let lim = ((size2 - vs) / 2.0).max(vec2(0.0, 0.0));
        let clamped = vec2(pan2.x.clamp(-lim.x, lim.x), pan2.y.clamp(-lim.y, lim.y));
        if clamped != pan2 {
            app.set_pan(clamped);
        }
    }
    // badges
    let mut bx = rect.left() + 12.0;
    if app.show_original {
        bx = badge(ui, pos2(bx, rect.top() + 12.0), &tr("ORIGINAL")) + 6.0;
    }
    if app.subject_busy() {
        bx = badge(ui, pos2(bx, rect.top() + 12.0), &tr("Detecting the subject…")) + 6.0;
    }
    if let Some(s) = app.solo_id {
        if let Some(n) = app.layers().iter().flat_map(|n| std::iter::once(n.clone()).chain(n.children.clone())).find(|n| n.id == s).or_else(|| {
            use crate::model::LayerTreeRead;
            app.layers().find(s).cloned()
        }) {
            badge(ui, pos2(bx, rect.top() + 12.0), &format!("{} · {}", tr("SOLO"), n.name));
        }
    }
    if app.is_exporting {
        let r = Rect::from_center_size(rect.center(), vec2(72.0, 72.0));
        ui.painter().rect_filled(r, CornerRadius::same(14), with_alpha(panel_bg(), 0.92));
        ui.put(Rect::from_center_size(rect.center(), vec2(32.0, 32.0)), egui::Spinner::new().size(32.0).color(text()));
    }
    toast(ui, app, rect);
}

/// Heuristic: plain mouse wheels zoom (like the macOS app), trackpads pan.
fn is_wheel(ui: &Ui) -> bool {
    ui.input(|i| {
        i.raw.events.iter().any(|e| matches!(e, egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, .. }))
    })
}

fn zoom_at(app: &mut AppState, rect: Rect, factor: f64, p: egui::Pos2) {
    let old = app.display_zoom();
    let new = (old * factor).clamp(0.02, 32.0);
    let center = rect.center() + app.pan();
    let rel = (p - center) / old as f32;
    let new_center = p - rel * new as f32;
    app.set_pan(new_center - rect.center());
    app.set_zoom(Some(new));
}

fn badge(ui: &Ui, at: egui::Pos2, s: &str) -> f32 {
    let g = ui.painter().layout_no_wrap(s.to_string(), bold(10.5), Color32::BLACK);
    let r = Rect::from_min_size(at, g.size() + vec2(18.0, 8.0));
    ui.painter().rect_filled(r, CornerRadius::same(255), ACCENT);
    ui.painter().galley(r.min + vec2(9.0, 4.0), g, Color32::BLACK);
    r.right()
}

fn toast(ui: &Ui, app: &AppState, rect: Rect) {
    if let Some((t, _)) = &app.toast {
        let g = ui.painter().layout_no_wrap(t.clone(), medium(12.0), text());
        let size = g.size() + vec2(28.0, 16.0);
        let r = Rect::from_min_size(pos2(rect.center().x - size.x / 2.0, rect.bottom() - 16.0 - size.y - 12.0), size);
        ui.painter().rect_filled(r, CornerRadius::same(255), with_alpha(panel_bg2(), 0.95));
        ui.painter().rect_stroke(r, CornerRadius::same(255), Stroke::new(1.0, separator()), StrokeKind::Inside);
        ui.painter().galley(r.min + vec2(14.0, 8.0), g, text());
    }
}

fn empty_state(ui: &mut Ui, app: &mut AppState, rect: Rect) {
    let c = rect.center();
    let hover = ui.rect_contains_pointer(rect);
    let frame = Rect::from_center_size(c - vec2(0.0, 70.0), vec2(120.0, 92.0));
    dashed_rect(ui, frame, if hover { ACCENT } else { text_faint() });
    paint_icon(ui, frame, "photo.badge.plus", 34.0, text_dim());
    ui.painter().text(c + vec2(0.0, 6.0), Align2::CENTER_CENTER, tr("Drop an image here"), semibold(17.0), text());
    ui.painter().text(c + vec2(0.0, 30.0), Align2::CENTER_CENTER, tr("JPG, PNG, HEIC, TIFF, RAW or .effector projects"), regular(12.0), text_faint());
    let open_label = tr("Open Image…");
    let paste_label = tr("Paste");
    let bw1 = ui.painter().layout_no_wrap(open_label.clone(), medium(12.0), text()).size().x + 24.0;
    let bw2 = ui.painter().layout_no_wrap(paste_label.clone(), medium(12.0), text()).size().x + 24.0;
    let total = bw1 + bw2 + 10.0;
    let row = Rect::from_min_size(pos2(c.x - total / 2.0, c.y + 52.0), vec2(total, 30.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(row), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            if pill_button(ui, &open_label, true).clicked() {
                app.open_panel();
            }
            if pill_button(ui, &paste_label, false).clicked() {
                app.paste_image();
            }
        });
    });
    let hint = format!("{} · {}", crate::shortcuts::display(crate::shortcuts::Action::Open), crate::shortcuts::display(crate::shortcuts::Action::Paste));
    ui.painter().text(c + vec2(0.0, 100.0), Align2::CENTER_CENTER, hint, regular(11.0), text_faint());
}

fn dashed_rect(ui: &Ui, r: Rect, c: Color32) {
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    for w in pts.windows(2) {
        let shapes = egui::Shape::dashed_line(&[w[0], w[1]], Stroke::new(1.5, c), 6.0, 5.0);
        ui.painter().extend(shapes);
    }
}

fn split_divider(ui: &mut Ui, app: &mut AppState, rect: Rect) {
    let x = rect.left() + rect.width() * app.split_pos as f32;
    ui.painter().rect_filled(Rect::from_min_max(pos2(x - 0.75, rect.top()), pos2(x + 0.75, rect.bottom())), CornerRadius::ZERO, with_alpha(Color32::WHITE, 0.9));
    let knob = Rect::from_center_size(pos2(x, rect.center().y), vec2(26.0, 26.0));
    let resp = ui.interact(knob.expand(4.0), Id::new("split-knob"), Sense::drag());
    ui.painter().circle_filled(knob.center() + vec2(0.0, 1.0), 13.5, Color32::from_black_alpha(90));
    ui.painter().circle_filled(knob.center(), 13.0, Color32::WHITE);
    paint_icon(ui, knob, "arrow.left.and.right", 12.0, Color32::BLACK);
    if resp.dragged() {
        if let Some(p) = resp.interact_pointer_pos() {
            app.split_pos = (((p.x - rect.left()) / rect.width().max(1.0)) as f64).clamp(0.02, 0.98);
        }
    }
    resp.on_hover_text(tr("Drag to compare"));
    let lab = |s: &str, at: egui::Pos2| {
        let g = ui.painter().layout_no_wrap(s.to_string(), bold(10.0), Color32::WHITE);
        let r = Rect::from_min_size(at, g.size() + vec2(14.0, 6.0));
        ui.painter().rect_filled(r, CornerRadius::same(255), Color32::from_black_alpha(140));
        ui.painter().galley(r.min + vec2(7.0, 3.0), g, Color32::WHITE);
    };
    lab(&tr("BEFORE"), pos2((x - 70.0).max(rect.left() + 8.0), rect.top() + 12.0));
    lab(&tr("AFTER"), pos2((x + 10.0).min(rect.right() - 60.0), rect.top() + 12.0));
}

fn norm_aspect(app: &AppState) -> Option<f64> {
    let src = app.source()?;
    let a = CROP_ASPECTS.iter().find(|o| o.0 == app.crop_aspect)?.2?;
    let pa = if a < 0.0 { src.width as f64 / src.height as f64 } else { a };
    Some(pa * src.height as f64 / src.width as f64)
}

pub fn apply_crop_aspect(app: &mut AppState) {
    let Some(a) = norm_aspect(app) else { return };
    let mut n = app.crop_draft;
    let (cx, cy) = (n.x + n.w / 2.0, n.y + n.h / 2.0);
    let mut w = n.w;
    let mut h = w / a;
    if h > 1.0 {
        h = 1.0;
        w = h * a;
    }
    if w > 1.0 {
        w = 1.0;
        h = w / a;
    }
    // fit as large as possible around the current center
    let max_w = a.min(1.0);
    if w < max_w * 0.5 {
        w = max_w * 0.9;
        h = w / a;
    }
    n.w = w;
    n.h = h;
    n.x = (cx - w / 2.0).clamp(0.0, 1.0 - w);
    n.y = (cy - h / 2.0).clamp(0.0, 1.0 - h);
    app.crop_draft = n;
}

fn crop_overlay(ui: &mut Ui, app: &mut AppState, view: Rect, ir: Rect) {
    let c = app.crop_draft;
    let cr = Rect::from_min_size(
        pos2(ir.left() + c.x as f32 * ir.width(), ir.top() + c.y as f32 * ir.height()),
        vec2(c.w as f32 * ir.width(), c.h as f32 * ir.height()),
    );
    let dim = Color32::from_black_alpha(140);
    let p = ui.painter().with_clip_rect(view);
    p.rect_filled(Rect::from_min_max(view.min, pos2(view.right(), cr.top())), CornerRadius::ZERO, dim);
    p.rect_filled(Rect::from_min_max(pos2(view.left(), cr.bottom()), view.max), CornerRadius::ZERO, dim);
    p.rect_filled(Rect::from_min_max(pos2(view.left(), cr.top()), pos2(cr.left(), cr.bottom())), CornerRadius::ZERO, dim);
    p.rect_filled(Rect::from_min_max(pos2(cr.right(), cr.top()), pos2(view.right(), cr.bottom())), CornerRadius::ZERO, dim);
    for i in 1..3 {
        let f = i as f32 / 3.0;
        p.line_segment([pos2(cr.left() + cr.width() * f, cr.top()), pos2(cr.left() + cr.width() * f, cr.bottom())], Stroke::new(0.5, with_alpha(Color32::WHITE, 0.35)));
        p.line_segment([pos2(cr.left(), cr.top() + cr.height() * f), pos2(cr.right(), cr.top() + cr.height() * f)], Stroke::new(0.5, with_alpha(Color32::WHITE, 0.35)));
    }
    p.rect_stroke(cr, CornerRadius::ZERO, Stroke::new(1.5, ACCENT), StrokeKind::Middle);
    let start_id = Id::new("crop-start");
    // move area
    let move_r = cr.shrink(12.0);
    if move_r.width() > 0.0 && move_r.height() > 0.0 {
        let mresp = ui.interact(move_r, Id::new("crop-move"), Sense::drag());
        if mresp.drag_started() {
            ui.data_mut(|d| d.insert_temp(start_id, (app.crop_draft, mresp.interact_pointer_pos().unwrap_or_default())));
        }
        if mresp.dragged() {
            if let (Some((s, p0)), Some(pos)) = (ui.data(|d| d.get_temp::<(CropRect, egui::Pos2)>(start_id)), mresp.interact_pointer_pos()) {
                let t = pos - p0;
                let mut n = s;
                n.x = (s.x + (t.x / ir.width()) as f64).clamp(0.0, 1.0 - s.w);
                n.y = (s.y + (t.y / ir.height()) as f64).clamp(0.0, 1.0 - s.h);
                app.crop_draft = n;
            }
        }
        if mresp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
        }
    }
    // handles
    let handles = [(0.0, 0.0), (0.5, 0.0), (1.0, 0.0), (0.0, 0.5), (1.0, 0.5), (0.0, 1.0), (0.5, 1.0), (1.0, 1.0)];
    for (i, (hx, hy)) in handles.iter().enumerate() {
        let hp = pos2(cr.left() + cr.width() * hx, cr.top() + cr.height() * hy);
        let hr = Rect::from_center_size(hp, vec2(12.0, 12.0));
        let resp = ui.interact(hr.expand(4.0), Id::new(("crop-h", i)), Sense::drag());
        ui.painter().rect_filled(hr, CornerRadius::same(2), Color32::WHITE);
        ui.painter().rect_stroke(hr, CornerRadius::same(2), Stroke::new(1.0, Color32::from_black_alpha(100)), StrokeKind::Inside);
        if resp.drag_started() {
            ui.data_mut(|d| d.insert_temp(start_id, (app.crop_draft, resp.interact_pointer_pos().unwrap_or_default())));
        }
        if resp.dragged() {
            if let (Some((s, p0)), Some(pos)) = (ui.data(|d| d.get_temp::<(CropRect, egui::Pos2)>(start_id)), resp.interact_pointer_pos()) {
                let t = pos - p0;
                let (dx, dy) = ((t.x / ir.width()) as f64, (t.y / ir.height()) as f64);
                let (mut x0, mut y0, mut x1, mut y1) = (s.x, s.y, s.x + s.w, s.y + s.h);
                if *hx == 0.0 {
                    x0 = (x0 + dx).max(0.0).min(x1 - 0.02);
                }
                if *hx == 1.0 {
                    x1 = (x1 + dx).min(1.0).max(x0 + 0.02);
                }
                if *hy == 0.0 {
                    y0 = (y0 + dy).max(0.0).min(y1 - 0.02);
                }
                if *hy == 1.0 {
                    y1 = (y1 + dy).min(1.0).max(y0 + 0.02);
                }
                let mut n = CropRect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
                if let Some(a) = norm_aspect(app) {
                    if *hx == 0.5 {
                        // vertical edge drag -> width follows
                        let w = n.h * a;
                        n.x = (n.x + (n.w - w) / 2.0).min(1.0 - w).max(0.0);
                        n.w = w.min(1.0);
                    } else {
                        let hh = n.w / a;
                        if *hy == 0.0 {
                            n.y = (y1 - hh).max(0.0);
                        }
                        n.h = hh.min(1.0 - n.y);
                        n.w = n.h * a;
                        if *hx == 0.0 {
                            n.x = x1 - n.w;
                        }
                    }
                }
                app.crop_draft = n;
            }
        }
    }
    // crop bar (only when the side panels are hidden: the Crop panel has the same controls)
    if !(app.hide_side_panels || app.hide_all_panels) {
        let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        if enter {
            app.commit_crop();
        } else if esc {
            app.cancel_crop();
        }
        return;
    }
    let bar_w = 520.0f32.min(view.width() - 20.0);
    let bar = Rect::from_min_size(pos2(view.center().x - bar_w / 2.0, view.bottom() - 12.0 - 44.0), vec2(bar_w, 44.0));
    ui.painter().rect_filled(bar, CornerRadius::same(10), with_alpha(panel_bg2(), 0.96));
    ui.painter().rect_stroke(bar, CornerRadius::same(10), Stroke::new(1.0, separator()), StrokeKind::Inside);
    ui.scope_builder(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(12.0, 8.0))), |ui| {
        ui.horizontal_centered(|ui| {
            ui.label(egui::RichText::new(icon_text("crop")).font(icon_font(14.0)).color(accent_text()));
            let items: Vec<PickerItem> = CROP_ASPECTS.iter().map(|o| PickerItem::new(o.0, &tr(o.1), None)).collect();
            let r = picker_field(ui, "crop.aspect", &items, &app.crop_aspect.clone(), None, &[], false);
            if let Some(v) = r.selected {
                app.crop_aspect = v;
                apply_crop_aspect(app);
            }
            if let Some(src) = app.source() {
                let r = app.crop_draft.pixel_rect(src.width, src.height);
                ui.label(egui::RichText::new(format!("{} × {}", r.2 as i64, r.3 as i64)).font(value_font()).color(text_dim()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pill_button(ui, &tr("Apply"), true).clicked() {
                    app.commit_crop();
                }
                if pill_button(ui, &tr("Cancel"), false).clicked() {
                    app.cancel_crop();
                }
                if pill_button(ui, &tr("Reset"), false).clicked() {
                    app.reset_crop();
                }
            });
        });
    });
    let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
    if enter {
        app.commit_crop();
    } else if esc {
        app.cancel_crop();
    }
}

/// Backdrop swatches offered on right-click around the image (white → greys → black).
pub const BACKDROPS: [(&str, &str, f32); 6] = [
    ("white", "White", 1.0),
    ("light", "Light Gray", 0.72),
    ("mid", "Medium Gray", 0.45),
    ("dark", "Dark Gray", 0.22),
    ("charcoal", "Charcoal", 0.1),
    ("black", "Black", 0.0),
];

fn backdrop_color() -> Color32 {
    let id = crate::settings::get().canvas_backdrop;
    match BACKDROPS.iter().find(|b| b.0 == id) {
        Some((_, _, v)) => {
            let c = (v * 255.0).round() as u8;
            Color32::from_rgb(c, c, c)
        }
        None => canvas_bg(),
    }
}

fn backdrop_menu(ui: &mut Ui, resp: &egui::Response, img_rect: Rect) {
    let id = Id::new("canvas-backdrop-menu");
    if resp.secondary_clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            if !img_rect.contains(p) {
                ui.data_mut(|d| d.insert_temp(id, p));
                egui::Popup::open_id(ui.ctx(), id);
            }
        }
    }
    let Some(at) = ui.data(|d| d.get_temp::<egui::Pos2>(id)) else { return };
    let current = crate::settings::get().canvas_backdrop;
    egui::Popup::new(id, ui.ctx().clone(), egui::PopupAnchor::Position(at), ui.layer_id())
        .open_memory(None)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
        .show(|ui| {
            ui.set_min_width(170.0);
            ui.label(egui::RichText::new(tr("Background")).font(semibold(11.0)).color(text_dim()));
            ui.add_space(2.0);
            ui.spacing_mut().item_spacing.y = 1.0;
            for (key, label, v) in BACKDROPS {
                let c = (v * 255.0).round() as u8;
                let on = current == key;
                // the whole row is the button, not only its text
                let w = ui.available_width().max(170.0);
                let (r, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
                if on || resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(5), if resp.hovered() { row_hover() } else { row_selected() });
                }
                let sw = Rect::from_center_size(pos2(r.left() + 13.0, r.center().y), vec2(14.0, 14.0));
                ui.painter().rect_filled(sw, CornerRadius::same(3), Color32::from_rgb(c, c, c));
                ui.painter().rect_stroke(sw, CornerRadius::same(3), Stroke::new(1.0, separator()), StrokeKind::Inside);
                ui.painter().text(pos2(sw.right() + 9.0, r.center().y), Align2::LEFT_CENTER, tr(label), regular(12.5), text());
                if on {
                    paint_icon(ui, Rect::from_center_size(pos2(r.right() - 12.0, r.center().y), vec2(12.0, 12.0)), "checkmark", 11.0, accent_text());
                }
                if resp.clicked() {
                    crate::settings::update(|s| s.canvas_backdrop = key.to_string());
                    ui.close();
                }
            }
        });
}

