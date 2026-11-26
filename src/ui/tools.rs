//! Right sidebar tools (Lightroom Classic style): a row of icons under the histogram — Edit,
//! Crop, Canvas & Borders and Masks — and the panels of Crop, Canvas and Masks.

use egui::{pos2, vec2, Color32, CornerRadius, Id, Rect, Sense, Stroke, Ui};
use uuid::Uuid;

use super::theme::*;
use super::widgets::*;
use crate::app::{AppState, Tool};
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
                let items: Vec<PickerItem> = super::canvas_view::CROP_ASPECTS.iter().map(|o| PickerItem::new(o.0, &tr(o.1), None)).collect();
                let r = picker_field(ui, "crop.aspect.panel", &items, &app.crop_aspect.clone(), None, &[], false);
                if let Some(v) = r.selected {
                    app.crop_aspect = v;
                    super::canvas_view::apply_crop_aspect(app);
                }
            });
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
const DEPTH_PRESETS: [(&str, &str, f64, f64); 3] = [
    ("Subject", "Near the camera", 0.55, 1.0),
    ("Background", "Far from the camera", 0.0, 0.42),
    ("Middle", "Middle distance", 0.3, 0.7),
];

fn depth_shape(from: f64, to: f64) -> MaskShape { MaskShape::Depth { from, to, feather: 0.12 } }

/// "Create Mask" button with its menu of mask kinds.
fn create_mask_button(ui: &mut Ui, app: &mut AppState, apply_to: Option<Uuid>) {
    let resp = pill_icon_button(ui, "plus", &tr("Create Mask"), true);
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(220.0);
        ui.label(egui::RichText::new(spaced_upper(&tr("Depth"))).font(section_font()).color(text_faint()));
        for (name, sub, a, b) in DEPTH_PRESETS {
            if ui.add(egui::Button::new(format!("{}  ·  {}", tr(name), tr(sub)))).clicked() {
                let id = app.create_mask(depth_shape(a, b), apply_to);
                let nm = format!("{} {}", tr(name), app.state().masks.iter().filter(|m| m.name.starts_with(&tr(name))).count() + 1);
                app.update_mask(id, None, |m| m.name = nm.clone());
                ui.close();
            }
        }
        ui.separator();
        ui.add_enabled(false, egui::Button::new(tr("Brush, gradients and color range: coming soon")));
    });
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
            }
            for m in &st.masks {
                mask_list_row(ui, app, &st, m);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| create_mask_button(ui, app, None));
        });
        let st = app.state();
        let Some(mask) = app.selected_mask.and_then(|id| st.mask(id).cloned()) else { return };
        ui.add_space(4.0);
        separator_line(ui);
        mask_editor(ui, app, &st, &mask);
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
    if app.depth_busy() {
        egui::Frame::new().inner_margin(egui::Margin::symmetric(14, 2)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(11.0));
                ui.label(egui::RichText::new(tr("Calculating depth…")).font(regular(11.0)).color(text_dim()));
            });
        });
    } else if !crate::depth::model_available() {
        egui::Frame::new().inner_margin(egui::Margin::symmetric(14, 2)).show(ui, |ui| {
            hint(ui, &tr("Depth model not found: reinstall EFFECTOR to use depth"));
        });
    }
    let n = mask.parts.len();
    for (i, part) in mask.parts.iter().enumerate() {
        mask_part(ui, app, mask, i, part, n);
    }
    egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 14, top: 6, bottom: 10 }).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tr("Depth range")).font(label_font()).color(text_dim()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pill_icon_button(ui, "minus", &tr("Subtract"), false).on_hover_text(tr("Remove a depth range from this mask")).clicked() {
                    app.update_mask(mid, None, |m| m.parts.push(MaskPart::new(depth_shape(0.0, 0.42), MaskMode::Subtract)));
                }
                if pill_icon_button(ui, "plus", &tr("Add"), false).on_hover_text(tr("Add another depth range to this mask")).clicked() {
                    app.update_mask(mid, None, |m| m.parts.push(MaskPart::new(depth_shape(0.3, 0.7), MaskMode::Add)));
                }
            });
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
                if ui.checkbox(&mut v, egui::RichText::new(format!("{}  {}", icon_text(sym), node.name)).font(regular(12.0)).color(text())).changed() {
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
        let dv = app.show_depth;
        if ui.add(egui::Button::selectable(dv, format!("{}  {}", tr("View depth map"), shortcuts::display(Action::ShowDepth)))).clicked() {
            app.show_depth = !dv;
            app.request_render();
        }
        hint(ui, &tr("0% = farthest, 100% = nearest. The depth map is estimated by AI on this computer."));
    });
}

fn mask_part(ui: &mut Ui, app: &mut AppState, mask: &Mask, i: usize, part: &MaskPart, n: usize) {
    let mid = mask.id;
    let pid = part.id;
    let set = move |app: &mut AppState, key: &str, f: &dyn Fn(&mut MaskPart)| {
        app.update_mask(mid, Some(format!("{pid}-{key}")), |m| {
            if let Some(p) = m.parts.iter_mut().find(|p| p.id == pid) {
                f(p);
            }
        });
    };
    egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 10, top: 4, bottom: 4 }).show(ui, |ui| {
        egui::Frame::new().fill(with_alpha(field(), 0.6)).corner_radius(CornerRadius::same(8)).inner_margin(10).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.horizontal(|ui| {
                let (ir, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                paint_icon(ui, ir, part.shape.symbol(), 13.0, accent_text());
                let title = if i == 0 { tr(part.shape.kind_label()) } else { format!("{} · {}", tr(part.mode.label()), tr(part.shape.kind_label())) };
                ui.label(egui::RichText::new(title).font(semibold(12.0)).color(if part.enabled { text() } else { text_faint() }));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if n > 1 && icon_button(ui, "xmark", "Remove", false, 11.0).clicked() {
                        app.update_mask(mid, None, |m| m.parts.retain(|p| p.id != pid));
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
            match &part.shape {
                MaskShape::Depth { from, to, feather } => {
                    let (from, to, feather) = (*from, *to, *feather);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        for (name, _, a, b) in [DEPTH_PRESETS[1], DEPTH_PRESETS[2], DEPTH_PRESETS[0]] {
                            let on = (from - a).abs() < 1e-3 && (to - b).abs() < 1e-3;
                            if ui.add(egui::Button::selectable(on, tr(name))).clicked() {
                                set(app, "preset", &move |p| {
                                    if let MaskShape::Depth { from, to, .. } = &mut p.shape {
                                        *from = a;
                                        *to = b;
                                    }
                                });
                            }
                        }
                    });
                    let track: Vec<Rgba> = (0..8)
                        .map(|i| {
                            let v = 0.08 + 0.84 * i as f64 / 7.0;
                            Rgba::new(v, v, v)
                        })
                        .collect();
                    let id = Id::new(("mask-part", pid));
                    if let Some(v) = value_slider(ui, id.with("from"), SliderSpec::new("Far edge", from * 100.0, 0.0, 100.0, 0.0).step(1.0).unit("%").track(Some(&track))) {
                        set(app, "from", &move |p| {
                            if let MaskShape::Depth { from, .. } = &mut p.shape {
                                *from = v / 100.0;
                            }
                        });
                    }
                    if let Some(v) = value_slider(ui, id.with("to"), SliderSpec::new("Near edge", to * 100.0, 0.0, 100.0, 100.0).step(1.0).unit("%").track(Some(&track))) {
                        set(app, "to", &move |p| {
                            if let MaskShape::Depth { to, .. } = &mut p.shape {
                                *to = v / 100.0;
                            }
                        });
                    }
                    if let Some(v) = value_slider(ui, id.with("feather"), SliderSpec::new("Softness", feather * 100.0, 0.0, 50.0, 12.0).step(1.0).unit("%")) {
                        set(app, "feather", &move |p| {
                            if let MaskShape::Depth { feather, .. } = &mut p.shape {
                                *feather = v / 100.0;
                            }
                        });
                    }
                }
            }
            if let Some(v) = toggle_row(ui, "Invert", part.invert) {
                set(app, "invert", &move |p| p.invert = v);
            }
        });
    });
}

/// "Mask" row of a layer's settings (Edit tool): which mask limits it, inverted or not.
pub fn layer_mask_row(ui: &mut Ui, app: &mut AppState, layer: &LayerNode) {
    let st = app.state();
    let lid = layer.id;
    let mut items = vec![PickerItem::new("none", &tr("None (whole image)"), None)];
    for m in &st.masks {
        items.push(PickerItem::new(&m.id.to_string(), &m.name, Some(tr("Masks"))));
    }
    let cur = layer.mask.filter(|r| st.mask(r.id).is_some());
    let sel = cur.map(|r| r.id.to_string()).unwrap_or_else(|| "none".into());
    let actions = vec![
        PickerAction { label: tr("New Mask…"), symbol: "plus", destructive: false },
        PickerAction { label: tr("Edit Masks…"), symbol: "circle.dashed.inset.filled", destructive: false },
    ];
    labeled_row(ui, "Mask", |ui| {
        let r = picker_field(ui, &format!("layer-mask.{lid}"), &items, &sel, None, &actions, false);
        if let Some(v) = r.selected {
            let invert = cur.map(|c| c.invert).unwrap_or(false);
            let m = if v == "none" { None } else { Uuid::parse_str(&v).ok().map(|id| MaskRef { id, invert }) };
            app.set_layer_mask(lid, m);
        }
        match r.action {
            Some(0) => {
                app.create_mask(depth_shape(DEPTH_PRESETS[0].2, DEPTH_PRESETS[0].3), Some(lid));
                app.set_tool(Tool::Masks);
            }
            Some(1) => {
                app.selected_mask = cur.map(|c| c.id).or(app.selected_mask);
                app.set_tool(Tool::Masks);
            }
            _ => {}
        }
    });
    if let Some(c) = cur {
        if let Some(v) = toggle_row(ui, "Invert Mask", c.invert) {
            app.set_layer_mask(lid, Some(MaskRef { id: c.id, invert: v }));
        }
    }
}

/// Overlay of the selected mask on the image (Masks tool), placed over the photo inside the
/// canvas borders.
pub fn paint_mask_overlay(ui: &Ui, app: &AppState, tex: egui::TextureId, img_rect: Rect, clip: Rect) {
    let Some(src) = app.source() else { return };
    let st = app.state();
    let mut r = img_rect;
    if let Some(c) = st.active_canvas() {
        let (cw, ch) = src.cropped_size(&st.crop);
        let ((ow, oh), (ix, iy, iw, ih)) = c.layout(cw as f64, ch as f64);
        let (sx, sy) = (img_rect.width() as f64 / ow, img_rect.height() as f64 / oh);
        r = Rect::from_min_size(pos2(img_rect.left() + (ix * sx) as f32, img_rect.top() + (iy * sy) as f32), vec2((iw * sx) as f32, (ih * sy) as f32));
    }
    ui.painter().with_clip_rect(clip).image(tex, r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}
