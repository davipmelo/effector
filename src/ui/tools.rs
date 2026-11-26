//! Right sidebar tools (Lightroom Classic style): a row of icons under the histogram — Edit,
//! Crop, Canvas & Borders and Masks — and the panels of Crop, Canvas and Masks.

use egui::{pos2, vec2, Color32, CornerRadius, Id, Rect, Sense, Stroke, Ui};
use uuid::Uuid;

use super::theme::*;
use super::widgets::*;
use crate::app::{AppState, MaskDraw, Tool};
use crate::l10n::{tr, trf};
use crate::model::*;
use crate::shortcuts::{self, Action};

const TOOLS: [(Tool, &str, &str, Action); 4] = [
    (Tool::Edit, "slider.horizontal.3", "Edit", Action::ToolEdit),
    (Tool::Crop, "crop", "Crop", Action::Crop),
    (Tool::Canvas, "aspectratio", "Canvas & Borders", Action::ToolCanvas),
    (Tool::Masks, "circle.dashed.inset.filled", "Masks", Action::ToolMasks),
];

/// The icon row. A small dot marks tools that change the image (crop applied, canvas on, masks).
pub fn tool_strip(ui: &mut Ui, app: &mut AppState) {
    let has = app.has_image();
    let st = app.state();
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    let n = TOOLS.len() as f32;
    let bw = 38.0;
    let gap = ((rect.width() - 28.0 - bw * n) / (n - 1.0)).clamp(6.0, 22.0);
    let total = bw * n + gap * (n - 1.0);
    let mut x = rect.center().x - total / 2.0;
    for (tool, sym, label, action) in TOOLS {
        let r = Rect::from_min_size(pos2(x, rect.center().y - 14.0), vec2(bw, 28.0));
        x += bw + gap;
        let resp = ui.interact(r, Id::new(("tool", label)), if has { Sense::click() } else { Sense::hover() });
        let active = has && app.tool == tool;
        if active {
            ui.painter().rect_filled(r, CornerRadius::same(7), field_strong());
        } else if resp.hovered() && has {
            ui.painter().rect_filled(r, CornerRadius::same(7), row_hover());
        }
        let c = if !has { with_alpha(text_dim(), 0.35) } else if active { accent_text() } else if resp.hovered() { text() } else { text_dim() };
        paint_icon(ui, r, sym, 15.0, c);
        let marked = match tool {
            Tool::Crop => !st.crop.is_full(),
            Tool::Canvas => st.active_canvas().is_some(),
            Tool::Masks => !st.masks.is_empty(),
            Tool::Edit => false,
        };
        if marked && has {
            ui.painter().circle_filled(pos2(r.right() - 6.0, r.top() + 6.0), 2.2, ACCENT);
        }
        let keys = shortcuts::display(action);
        let tip = if keys.is_empty() { tr(label) } else { format!("{}  {}", tr(label), keys) };
        if resp.on_hover_text(tip).clicked() {
            app.set_tool(tool);
        }
    }
    ui.painter().rect_filled(Rect::from_min_size(pos2(rect.left(), rect.bottom() - 1.0), vec2(w, 1.0)), CornerRadius::ZERO, separator());
}

/// What the right sidebar shows under the tool row.
pub fn tool_panel(ui: &mut Ui, app: &mut AppState) {
    if !app.has_image() {
        super::inspector::inspector(ui, app);
        return;
    }
    match app.tool {
        Tool::Edit => super::inspector::inspector(ui, app),
        Tool::Crop => crop_panel(ui, app),
        Tool::Canvas => canvas_panel(ui, app),
        Tool::Masks => masks_panel(ui, app),
    }
}

fn hint(ui: &mut Ui, s: &str) {
    ui.label(egui::RichText::new(s).font(regular(11.0)).color(text_faint()));
}

// MARK: - Crop

pub fn crop_panel(ui: &mut Ui, app: &mut AppState) {
    header(ui, &tr("Crop"), Some("crop"));
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::new().inner_margin(14).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            labeled_row(ui, "Aspect Ratio", |ui| {
                let items = super::canvas_view::aspect_items();
                let title = app.crop_aspect.strip_prefix("custom:").map(|t| t.to_string());
                let r = picker_field(ui, "crop.aspect.panel", &items, &app.crop_aspect.clone(), title.as_deref(), &[], false);
                if let Some(v) = r.selected {
                    if v == "custom" {
                        ui.data_mut(|d| d.insert_temp(Id::new("crop-custom-focus"), true));
                    }
                    ui.data_mut(|d| d.remove::<String>(Id::new("crop-custom-text")));
                    app.crop_aspect = v;
                    super::canvas_view::apply_crop_aspect(app);
                }
            });
            if app.crop_aspect.starts_with("custom") {
                // typed ratio: "2,39x1", "2.39:1", "16/9"…
                let tid = Id::new("crop-custom-text");
                let mut t: String = ui.data(|d| d.get_temp(tid)).unwrap_or_else(|| app.crop_aspect.strip_prefix("custom:").unwrap_or("").to_string());
                let ok = super::canvas_view::parse_ratio(&t).is_some() || t.trim().is_empty();
                labeled_row(ui, "Custom", |ui| {
                    let te = ui.add(
                        egui::TextEdit::singleline(&mut t)
                            .hint_text("2,39x1")
                            .desired_width(110.0)
                            .text_color(if ok { text() } else { Color32::from_rgb(240, 90, 90) }),
                    );
                    if ui.data(|d| d.get_temp::<bool>(Id::new("crop-custom-focus")).unwrap_or(false)) {
                        te.request_focus();
                        ui.data_mut(|d| d.remove::<bool>(Id::new("crop-custom-focus")));
                    }
                    if te.changed() {
                        if super::canvas_view::parse_ratio(&t).is_some() {
                            app.crop_aspect = format!("custom:{}", t.trim());
                            super::canvas_view::apply_crop_aspect(app);
                        }
                    }
                    if te.lost_focus() {
                        super::canvas_view::remember_custom_ratio(&t);
                    }
                });
                ui.data_mut(|d| d.insert_temp(tid, t));
                hint(ui, &tr("Width × height, e.g. 2,39x1 · 4:5 · 16/9"));
            }
            if let Some(src) = app.source() {
                let r = app.crop_draft.pixel_rect(src.width, src.height);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(tr("Size")).font(label_font()).color(text_dim()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{} × {} px", r.2 as i64, r.3 as i64)).font(value_font()).color(text()));
                    });
                });
            }
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if pill_button(ui, &tr("Reset"), false).clicked() {
                    app.reset_crop();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, &tr("Done"), true).clicked() {
                        app.commit_crop();
                    }
                    if pill_button(ui, &tr("Cancel"), false).clicked() {
                        app.cancel_crop();
                    }
                });
            });
            ui.add_space(4.0);
            hint(ui, &keys_hint(&tr("Drag the corners or edges to crop, drag inside to move. ↩ applies, ⎋ cancels. Choosing another tool also applies the crop.")));
        });
    });
}

// MARK: - Canvas & Borders

pub fn canvas_panel(ui: &mut Ui, app: &mut AppState) {
    header(ui, &tr("Canvas & Borders"), Some("aspectratio"));
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::new().inner_margin(14).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            super::inspector::canvas_controls(ui, app);
            ui.add_space(4.0);
            hint(ui, &tr("Fits the photo in another aspect ratio with borders around it (Instagram etc.). It applies to the whole image, whatever layer is selected."));
        });
    });
}

// MARK: - Masks

/// Quick depth selections offered when creating a mask or a part.
/// Colour and sign of each mask mode (green add, red subtract, blue intersect).
pub fn mode_style(m: MaskMode) -> (Color32, &'static str) {
    match m {
        MaskMode::Add => (Color32::from_rgb(70, 200, 120), "+"),
        MaskMode::Subtract => (Color32::from_rgb(240, 85, 85), "−"),
        MaskMode::Intersect => (Color32::from_rgb(90, 160, 255), "∩"),
    }
}

/// Outlined button in a mode colour, with its sign: "+ Add", "− Subtract".
fn mode_button(ui: &mut Ui, mode: MaskMode, label: &str) -> egui::Response {
    let (c, sign) = mode_style(mode);
    let g = ui.painter().layout_no_wrap(format!("{sign}  {}", tr(label)), semibold(12.0), c);
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 26.0, 28.0), Sense::click());
    ui.painter().rect_filled(r, CornerRadius::same(7), with_alpha(c, if resp.hovered() { 0.22 } else { 0.1 }));
    ui.painter().rect_stroke(r, CornerRadius::same(7), Stroke::new(1.2, with_alpha(c, 0.85)), egui::StrokeKind::Inside);
    ui.painter().galley(r.center() - g.size() / 2.0, g, c);
    resp
}

/// The five ways to make a mask (Lightroom Classic order), with their shortcut actions.
const MASK_KINDS: [(&str, &str, Action); 5] = [
    ("mask.subject", "Select Subject", Action::MaskSubject),
    ("mask.background", "Select Background", Action::MaskBackground),
    ("paintbrush", "Brush", Action::MaskBrush),
    ("mask.linear", "Linear Gradient", Action::MaskLinear),
    ("mask.radial", "Radial Gradient", Action::MaskRadial),
];

/// Starts making a part of kind `i` of `MASK_KINDS`: AI kinds are added at once, drawing kinds
/// arm the tool (the part appears when you drag on the image).
fn begin_kind(app: &mut AppState, i: usize, into: Option<(Uuid, MaskMode)>) {
    match i {
        0 => {
            app.add_mask_part(MaskShape::subject_default(), into);
        }
        1 => {
            app.add_mask_part(MaskShape::background_default(), into);
        }
        _ => {
            app.mask_draw_into = into;
            let d = match i {
                2 => MaskDraw::Brush,
                3 => MaskDraw::Linear,
                _ => MaskDraw::Radial,
            };
            app.start_mask_draw(Some(d));
            app.mask_draw = Some(d);
        }
    }
}

/// Menu with the five mask kinds. Returns the chosen index.
fn kinds_menu(ui: &mut Ui) -> Option<usize> {
    ui.set_min_width(230.0);
    let mut out = None;
    for (i, (sym, label, action)) in MASK_KINDS.iter().enumerate() {
        if i == 2 {
            ui.separator();
        }
        let keys = shortcuts::display(*action);
        let text = if keys.is_empty() { format!("{}  {}", icon_text(sym), tr(label)) } else { format!("{}  {}      {}", icon_text(sym), tr(label), keys) };
        if ui.add(egui::Button::new(egui::RichText::new(text).font(regular(12.0)))).clicked() {
            out = Some(i);
            ui.close();
        }
    }
    out
}

/// Big buttons for the five kinds (shown when there is no mask yet).
fn kinds_grid(ui: &mut Ui, app: &mut AppState) {
    let w = ui.available_width();
    for (i, (sym, label, action)) in MASK_KINDS.iter().enumerate() {
        let (r, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
        let armed = match i {
            2 => app.mask_draw == Some(MaskDraw::Brush),
            3 => app.mask_draw == Some(MaskDraw::Linear),
            4 => app.mask_draw == Some(MaskDraw::Radial),
            _ => false,
        };
        ui.painter().rect_filled(r, CornerRadius::same(7), if armed { row_selected() } else if resp.hovered() { field_strong() } else { field() });
        paint_icon(ui, Rect::from_center_size(pos2(r.left() + 16.0, r.center().y), vec2(16.0, 16.0)), sym, 13.0, if armed { accent_text() } else { text_dim() });
        ui.painter().text(pos2(r.left() + 32.0, r.center().y), egui::Align2::LEFT_CENTER, tr(label), regular(12.0), text());
        let keys = shortcuts::display(*action);
        ui.painter().text(pos2(r.right() - 10.0, r.center().y), egui::Align2::RIGHT_CENTER, keys, regular(10.5), text_faint());
        if resp.clicked() {
            begin_kind(app, i, None);
        }
        ui.add_space(2.0);
    }
}

/// "Create New Mask" button with its menu.
fn create_mask_button(ui: &mut Ui, app: &mut AppState) {
    let resp = pill_icon_button(ui, "plus", &tr("Create New Mask"), true);
    if let Some(i) = egui::Popup::menu(&resp).show(kinds_menu).and_then(|r| r.inner) {
        begin_kind(app, i, None);
    }
}

pub fn masks_panel(ui: &mut Ui, app: &mut AppState) {
    // O shows / hides the overlay, like Lightroom
    let typing = ui.ctx().egui_wants_keyboard_input() || ui.ctx().memory(|m| m.focused().is_some());
    if !typing && ui.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.is_none()) {
        crate::settings::update(|s| s.mask_overlay = !s.mask_overlay);
        app.request_render();
    }
    header(ui, &tr("Masks"), Some("circle.dashed.inset.filled"));
    let st = app.state();
    if app.selected_mask.map(|id| st.mask(id).is_none()).unwrap_or(false) {
        app.selected_mask = st.masks.first().map(|m| m.id);
    }
    egui::ScrollArea::vertical().id_salt("masks-scroll").auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 10, top: 10, bottom: 6 }).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            if st.masks.is_empty() {
                egui::Frame::new().inner_margin(egui::Margin::symmetric(4, 4)).show(ui, |ui| {
                    hint(ui, &tr("Masks limit effects to a part of the image. Create a mask, then choose which effects use it. One mask can be used by many effects."));
                });
                ui.add_space(8.0);
                kinds_grid(ui, app);
            } else {
                for m in &st.masks {
                    mask_list_row(ui, app, &st, m);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| create_mask_button(ui, app));
            }
        });
        draw_status(ui, app);
        let st = app.state();
        let Some(mask) = app.selected_mask.and_then(|id| st.mask(id).cloned()) else { return };
        ui.add_space(4.0);
        separator_line(ui);
        mask_editor(ui, app, &st, &mask);
    });
}

/// What the armed drawing tool does, and its settings (brush).
fn draw_status(ui: &mut Ui, app: &mut AppState) {
    let Some(d) = app.mask_draw else { return };
    egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 10, top: 4, bottom: 8 }).show(ui, |ui| {
        egui::Frame::new().fill(with_alpha(ACCENT, 0.12)).corner_radius(CornerRadius::same(8)).inner_margin(10).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.horizontal(|ui| {
                let (sym, title) = match d {
                    MaskDraw::Brush => ("paintbrush", "Brush"),
                    MaskDraw::Linear => ("mask.linear", "Linear Gradient"),
                    MaskDraw::Radial => ("mask.radial", "Radial Gradient"),
                };
                let (ir, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                paint_icon(ui, ir, sym, 13.0, accent_text());
                ui.label(egui::RichText::new(tr(title)).font(semibold(12.0)).color(text()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(tr("Done"))).on_hover_text(keys_hint("⎋")).clicked() {
                        app.mask_draw = None;
                    }
                });
            });
            let into = app.mask_draw_into.and_then(|(m, mode)| app.state().mask(m).map(|mm| (mm.name.clone(), mode)));
            let target = match &into {
                Some((n, MaskMode::Subtract)) => trf("Subtracting from %@", &[n]),
                Some((n, MaskMode::Intersect)) => trf("Intersecting with %@", &[n]),
                Some((n, _)) => trf("Adding to %@", &[n]),
                None => String::new(),
            };
            match d {
                MaskDraw::Brush => {
                    hint(ui, &keys_hint(&tr("Paint on the image · hold ⌥ to erase · [ ] size · ⇧[ ⇧] feather")));
                    let b = app.brush;
                    let id = Id::new("brush-settings");
                    if let Some(v) = value_slider(ui, id.with("size"), SliderSpec::new("Size", b.size as f64 * 1000.0, 2.0, 400.0, 35.0).step(1.0).curve(1.8)) {
                        app.brush.size = (v / 1000.0) as f32;
                    }
                    if let Some(v) = value_slider(ui, id.with("feather"), SliderSpec::new("Feather", b.feather as f64 * 100.0, 0.0, 100.0, 60.0).step(1.0).unit("%")) {
                        app.brush.feather = (v / 100.0) as f32;
                    }
                    if let Some(v) = value_slider(ui, id.with("flow"), SliderSpec::new("Flow", b.flow as f64 * 100.0, 1.0, 100.0, 80.0).step(1.0).unit("%")) {
                        app.brush.flow = (v / 100.0) as f32;
                    }
                    if let Some(v) = toggle_row(ui, "Erase", app.brush_erase) {
                        app.brush_erase = v;
                    }
                }
                MaskDraw::Linear => hint(ui, &tr("Drag on the image: from where the effect is full to where it fades out.")),
                MaskDraw::Radial => hint(ui, &keys_hint(&tr("Drag on the image from the centre outward · ⇧ for a circle."))),
            }
            if !target.is_empty() {
                hint(ui, &target);
            }
        });
    });
}

fn mask_list_row(ui: &mut Ui, app: &mut AppState, st: &DocumentState, m: &Mask) {
    let sel = app.selected_mask == Some(m.id);
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
    if sel {
        ui.painter().rect_filled(r, CornerRadius::same(6), row_selected());
    } else if resp.hovered() {
        ui.painter().rect_filled(r, CornerRadius::same(6), row_hover());
    }
    // tiny swatch of the mask: red disc, hollow when no part is on
    let c = pos2(r.left() + 14.0, r.center().y);
    if m.is_active() {
        ui.painter().circle_filled(c, 5.5, Color32::from_rgb(235, 70, 70));
    } else {
        ui.painter().circle_stroke(c, 5.0, Stroke::new(1.2, text_faint()));
    }
    let used = st.layers_using(m.id).len();
    let count = if used == 0 { tr("unused") } else if used == 1 { tr("1 effect") } else { trf("%d effects", &[&used]) };
    let cg = ui.painter().layout_no_wrap(count, regular(10.5), text_faint());
    let cx = r.right() - 8.0 - cg.size().x;
    ui.painter().galley(pos2(cx, r.center().y - cg.size().y / 2.0), cg, text_faint());
    let mut job = egui::text::LayoutJob::simple_singleline(m.name.clone(), if sel { medium(12.0) } else { regular(12.0) }, text());
    job.wrap = egui::text::TextWrapping::truncate_at_width(cx - r.left() - 34.0);
    let g = ui.painter().layout_job(job);
    ui.painter().galley(pos2(r.left() + 28.0, r.center().y - g.size().y / 2.0), g, text());
    if resp.clicked() {
        if app.selected_mask != Some(m.id) {
            app.selected_part = m.parts.first().map(|p| p.id);
        }
        app.selected_mask = Some(m.id);
        app.request_render();
    }
    let id = m.id;
    resp.context_menu(|ui| {
        if ui.button(tr("Duplicate")).clicked() {
            app.duplicate_mask(id);
            ui.close();
        }
        ui.separator();
        if ui.button(egui::RichText::new(tr("Delete")).color(Color32::from_rgb(235, 80, 70))).clicked() {
            app.delete_mask(id);
            ui.close();
        }
    });
}

fn mask_editor(ui: &mut Ui, app: &mut AppState, st: &DocumentState, mask: &Mask) {
    let mid = mask.id;
    egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 14, top: 10, bottom: 4 }).show(ui, |ui| {
        ui.horizontal(|ui| {
            let name_id = Id::new(("mask-name", mid));
            let mut name: String = ui.data(|d| d.get_temp(name_id)).unwrap_or_else(|| mask.name.clone());
            let te = ui.add(egui::TextEdit::singleline(&mut name).font(semibold(13.0)).frame(egui::Frame::NONE).desired_width(ui.available_width() - 60.0));
            if te.has_focus() {
                ui.data_mut(|d| d.insert_temp(name_id, name.clone()));
            }
            if te.lost_focus() {
                let n = name.trim().to_string();
                if !n.is_empty() && n != mask.name {
                    app.update_mask(mid, None, |m| m.name = n.clone());
                }
                ui.data_mut(|d| d.remove::<String>(name_id));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if icon_button(ui, "trash", "Delete mask", false, 12.0).clicked() {
                    app.delete_mask(mid);
                }
                if icon_button(ui, "plus.square.on.square", "Duplicate mask", false, 12.0).clicked() {
                    app.duplicate_mask(mid);
                }
            });
        });
    });
    if mask.needs_subject() {
        if app.subject_busy() {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(14, 2)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(11.0));
                    ui.label(egui::RichText::new(tr("Detecting the subject…")).font(regular(11.0)).color(text_dim()));
                });
            });
        } else if !crate::subject::model_available() {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(14, 2)).show(ui, |ui| {
                hint(ui, &tr("Subject detection model not found: reinstall EFFECTOR"));
            });
        }
    }
    if app.selected_part.map(|p| !mask.parts.iter().any(|x| x.id == p)).unwrap_or(true) {
        app.selected_part = mask.parts.first().map(|p| p.id);
    }
    let n = mask.parts.len();
    for (i, part) in mask.parts.iter().enumerate() {
        mask_part(ui, app, mask, i, part, n);
    }
    egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 14, top: 6, bottom: 10 }).show(ui, |ui| {
        // Lightroom-style: add to (green) or subtract from (red) this mask with any kind
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let add = mode_button(ui, MaskMode::Add, "Add").on_hover_text(tr("Add to this mask: the new part makes the selection bigger"));
            if let Some(i) = egui::Popup::menu(&add).show(kinds_menu).and_then(|r| r.inner) {
                begin_kind(app, i, Some((mid, MaskMode::Add)));
            }
            let sub = mode_button(ui, MaskMode::Subtract, "Subtract").on_hover_text(tr("Subtract from this mask: the new part cuts a hole in the selection"));
            if let Some(i) = egui::Popup::menu(&sub).show(kinds_menu).and_then(|r| r.inner) {
                begin_kind(app, i, Some((mid, MaskMode::Subtract)));
            }
        });
    });
    separator_line(ui);
    // which effects use it
    panel_section(ui, Id::new(("mask-uses", mid)), "Effects Using This Mask", None, None, |ui| {
        let rows = st.layers.flattened();
        if rows.is_empty() {
            hint(ui, &tr("No effects yet. Add one, then come back to limit it with this mask."));
        }
        ui.spacing_mut().item_spacing.y = 2.0;
        for (node, depth) in rows {
            let on = node.mask.map(|r| r.id == mid).unwrap_or(false);
            let other = node.mask.filter(|r| r.id != mid).and_then(|r| st.mask(r.id)).map(|m| m.name.clone());
            ui.horizontal(|ui| {
                ui.add_space(depth as f32 * 12.0);
                let mut v = on;
                let sym = if node.is_group { "folder" } else { crate::effects::effect(&node.effect_id).map(|e| e.symbol).unwrap_or("square") };
                let changed = ui.checkbox(&mut v, "").changed();
                let (ir, _) = ui.allocate_exact_size(vec2(14.0, 16.0), Sense::hover());
                paint_icon(ui, ir, sym, 11.0, text_dim());
                let name = ui.add(egui::Label::new(egui::RichText::new(&node.name).font(regular(12.0)).color(text())).sense(Sense::click()));
                if changed || name.clicked() {
                    if name.clicked() {
                        v = !on;
                    }
                    app.set_layer_mask(node.id, if v { Some(MaskRef { id: mid, invert: false }) } else { None });
                }
                if let Some(o) = other {
                    ui.label(egui::RichText::new(trf("uses %@", &[&o])).font(regular(10.5)).color(text_faint()));
                } else if on && node.mask.map(|r| r.invert).unwrap_or(false) {
                    ui.label(egui::RichText::new(tr("inverted")).font(regular(10.5)).color(text_faint()));
                }
            });
        }
    });
    egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 14, top: 8, bottom: 14 }).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        let ov = crate::settings::get().mask_overlay;
        if let Some(v) = toggle_row(ui, &format!("{}  (O)", tr("Show Overlay")), ov) {
            crate::settings::update(|s| s.mask_overlay = v);
            app.request_render();
        }
    });
}

fn mask_part(ui: &mut Ui, app: &mut AppState, mask: &Mask, i: usize, part: &MaskPart, n: usize) {
    let mid = mask.id;
    let pid = part.id;
    let sel = app.selected_part == Some(pid);
    let set = move |app: &mut AppState, key: &str, f: &dyn Fn(&mut MaskPart)| {
        app.update_part(mid, pid, Some(format!("{pid}-{key}")), |p| f(p));
    };
    egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 10, top: 4, bottom: 4 }).show(ui, |ui| {
        let (mc, sign) = mode_style(part.mode);
        let fr = egui::Frame::new()
            .fill(with_alpha(field(), if sel { 0.95 } else { 0.5 }))
            .stroke(Stroke::new(1.0, with_alpha(mc, if sel { 0.9 } else { 0.35 })))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin { left: 14, right: 10, top: 10, bottom: 10 })
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.horizontal(|ui| {
                    // mode badge: + green, − red, ∩ blue
                    let (br, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    ui.painter().circle_filled(br.center(), 8.5, mc);
                    ui.painter().text(br.center() - vec2(0.0, 0.5), egui::Align2::CENTER_CENTER, sign, semibold(13.0), Color32::BLACK);
                    let (ir, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                    paint_icon(ui, ir, part.shape.symbol(), 13.0, accent_text());
                    let title = if i == 0 { tr(part.shape.kind_label()) } else { format!("{} · {}", tr(part.mode.label()), tr(part.shape.kind_label())) };
                    let t = ui.add(egui::Label::new(egui::RichText::new(title).font(semibold(12.0)).color(if part.enabled { text() } else { text_faint() })).sense(Sense::click()));
                    if t.clicked() {
                        app.selected_part = Some(pid);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if icon_button(ui, if n > 1 { "xmark" } else { "trash" }, "Remove", false, 11.0).clicked() {
                            app.delete_part(mid, pid);
                        }
                        if icon_button(ui, if part.enabled { "eye" } else { "eye.slash" }, "Show / Hide", false, 12.0).clicked() {
                            let on = !part.enabled;
                            set(app, "on", &move |p| p.enabled = on);
                        }
                    });
                });
                if i > 0 {
                    let modes = vec![("add".to_string(), tr("Add")), ("subtract".to_string(), tr("Subtract")), ("intersect".to_string(), tr("Intersect"))];
                    let cur = match part.mode {
                        MaskMode::Add => "add",
                        MaskMode::Subtract => "subtract",
                        MaskMode::Intersect => "intersect",
                    };
                    if let Some(v) = segmented_row(ui, "", &modes, cur) {
                        let mode = match v.as_str() {
                            "subtract" => MaskMode::Subtract,
                            "intersect" => MaskMode::Intersect,
                            _ => MaskMode::Add,
                        };
                        set(app, "mode", &move |p| p.mode = mode);
                    }
                }
                let id = Id::new(("mask-part", pid));
                let sl = |ui: &mut Ui, key: &str, label: &str, v: f64, min: f64, max: f64, def: f64, unit: &'static str| -> Option<f64> {
                    value_slider(ui, id.with(key.to_string()), SliderSpec::new(label, v, min, max, def).step(1.0).unit(unit))
                };
                match part.shape.clone() {
                    MaskShape::Linear { angle, width, .. } => {
                        if let Some(v) = sl(ui, "width", "Softness", width * 100.0, 1.0, 150.0, 30.0, "%") {
                            set(app, "width", &move |p| if let MaskShape::Linear { width, .. } = &mut p.shape { *width = v / 100.0 });
                        }
                        if let Some(v) = sl(ui, "angle", "Angle", angle, -180.0, 180.0, 90.0, "°") {
                            set(app, "angle", &move |p| if let MaskShape::Linear { angle, .. } = &mut p.shape { *angle = v });
                        }
                    }
                    MaskShape::Radial { rx, ry, angle, feather, .. } => {
                        let size = (rx.max(ry) * 100.0).max(0.1);
                        if let Some(v) = sl(ui, "size", "Size", size, 1.0, 150.0, 25.0, "%") {
                            let k = v / size;
                            set(app, "size", &move |p| if let MaskShape::Radial { rx, ry, .. } = &mut p.shape { *rx *= k; *ry *= k });
                        }
                        if let Some(v) = sl(ui, "round", "Roundness", (rx.min(ry) / rx.max(ry).max(1e-6)) * 100.0, 5.0, 100.0, 72.0, "%") {
                            set(app, "round", &move |p| if let MaskShape::Radial { rx, ry, .. } = &mut p.shape {
                                let big = rx.max(*ry);
                                if *rx >= *ry { *ry = big * v / 100.0 } else { *rx = big * v / 100.0 }
                            });
                        }
                        if let Some(v) = sl(ui, "feather", "Feather", feather * 100.0, 0.0, 100.0, 50.0, "%") {
                            set(app, "feather", &move |p| if let MaskShape::Radial { feather, .. } = &mut p.shape { *feather = v / 100.0 });
                        }
                        if let Some(v) = sl(ui, "angle", "Angle", angle, -180.0, 180.0, 0.0, "°") {
                            set(app, "angle", &move |p| if let MaskShape::Radial { angle, .. } = &mut p.shape { *angle = v });
                        }
                    }
                    MaskShape::Brush { strokes } => {
                        ui.horizontal(|ui| {
                            let painting = app.mask_draw == Some(MaskDraw::Brush) && sel;
                            if ui.add(egui::Button::selectable(painting, format!("{}  {}", icon_text("paintbrush"), tr("Paint")))).clicked() {
                                app.selected_part = Some(pid);
                                app.mask_draw_into = None;
                                app.mask_draw = if painting { None } else { Some(MaskDraw::Brush) };
                            }
                            if ui.add_enabled(!strokes.is_empty(), egui::Button::new(tr("Clear"))).clicked() {
                                set(app, "clear", &|p| if let MaskShape::Brush { strokes } = &mut p.shape { strokes.clear() });
                            }
                        });
                        hint(ui, &trf("%d strokes", &[&strokes.len()]));
                    }
                    MaskShape::Subject { softness, grow } | MaskShape::Background { softness, grow } => {
                        if let Some(v) = sl(ui, "soft", "Edge Softness", softness * 100.0, 0.0, 100.0, 15.0, "%") {
                            set(app, "soft", &move |p| match &mut p.shape {
                                MaskShape::Subject { softness, .. } | MaskShape::Background { softness, .. } => *softness = v / 100.0,
                                _ => {}
                            });
                        }
                        if let Some(v) = sl(ui, "grow", "Expand / Contract", grow * 100.0, -100.0, 100.0, 0.0, "%") {
                            set(app, "grow", &move |p| match &mut p.shape {
                                MaskShape::Subject { grow, .. } | MaskShape::Background { grow, .. } => *grow = v / 100.0,
                                _ => {}
                            });
                        }
                    }
                    MaskShape::Depth { .. } => {}
                }
                if let Some(v) = toggle_row(ui, &keys_hint(&format!("{}  (')", tr("Invert"))), part.invert) {
                    set(app, "invert", &move |p| p.invert = v);
                }
            });
        // colour stripe on the left edge (the mode at a glance)
        let r = fr.response.rect;
        ui.painter().rect_filled(Rect::from_min_size(r.min + vec2(1.0, 8.0), vec2(3.0, r.height() - 16.0)), CornerRadius::same(2), mc);
        if fr.response.interact(Sense::click()).clicked() {
            app.selected_part = Some(pid);
        }
    });
}

/// Overlay of the selected mask on the image (Masks tool), placed over the photo inside the
/// canvas borders.
pub fn paint_mask_overlay(ui: &Ui, app: &AppState, tex: egui::TextureId, img_rect: Rect, clip: Rect) {
    let r = super::mask_canvas::photo_rect(app, img_rect);
    ui.painter().with_clip_rect(clip).image(tex, r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}
